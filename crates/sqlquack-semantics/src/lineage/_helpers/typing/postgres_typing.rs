//! PostgreSQL's result types from its 18.6 catalogue: string literals, `||` and built-in calls.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::OnceLock;

use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::{DataType, Expression};

use crate::lineage::_helpers::checks::coercion::string_literal;
use crate::lineage::_helpers::checks::families::{Family, type_family};
use crate::lineage::_helpers::checks::signatures::call;
use crate::lineage::_helpers::tracing::postgres_names::postgres_name;
use crate::lineage::models::{Lineage, TransformKind};

/// The built-in functions of `pg_catalog`, one overload per row, read from the 18.6 server.
const PG_PROC: &str = include_str!("postgres_catalog/pg_proc.csv");
/// The built-in operators of `pg_catalog`, one per row, read from the 18.6 server.
const PG_OPERATOR: &str = include_str!("postgres_catalog/pg_operator.csv");
/// The casts between built-in types and the contexts they apply in, read from the 18.6 server.
const PG_CAST: &str = include_str!("postgres_catalog/pg_cast.csv");
/// The `castcontext` of a cast PostgreSQL applies implicitly.
const IMPLICIT_CONTEXT: &str = "i";
/// The `oprkind` of a binary operator.
const BINARY_KIND: &str = "b";
/// The `oprkind` of a prefix operator.
const PREFIX_KIND: &str = "l";
/// The string category, which an untyped literal prefers when operators differ there.
const STRING_CATEGORY: char = 'S';
/// The equality operator NULLIF resolves.
const EQUALS: &str = "=";
/// The prefix operator a unary minus applies.
const NEGATION: &str = "-";
/// The bound past which an integer literal is `bigint`, then `numeric`.
const INTEGER_LITERAL_TYPES: [(i128, &str); 2] =
    [(i32::MAX as i128, "INT"), (i64::MAX as i128, "BIGINT")];
/// The type of a numeric literal that is not an integer, or too large for `bigint`.
const NUMERIC: &str = "NUMERIC";
/// The built-in types of `pg_catalog` with their categories, read from the 18.6 server.
const PG_TYPE: &str = include_str!("postgres_catalog/pg_type.csv");
/// The type PostgreSQL gives an untyped string literal once a query's outputs are resolved.
const TEXT: &str = "TEXT";
/// The category of pseudo-types such as `anyelement`, which only a polymorphic call resolves.
const PSEUDO_CATEGORY: char = 'P';
/// The pseudo-types an argument of any type matches.
const ANY_TYPES: [&str; 5] = [
    "\"any\"",
    "anyelement",
    "anynonarray",
    "anycompatible",
    "anycompatiblenonarray",
];
/// The SQL value functions gram.y turns into `SQLValueFunction`, which `pg_proc` does not list.
const VALUE_FUNCTIONS: [(&str, &str); 11] = [
    ("current_date", "DATE"),
    ("current_time", "TIMETZ"),
    ("current_timestamp", "TIMESTAMPTZ"),
    ("localtime", "TIME"),
    ("localtimestamp", "TIMESTAMP"),
    ("current_role", "NAME"),
    ("current_user", "NAME"),
    ("session_user", "NAME"),
    ("user", "NAME"),
    ("current_catalog", "NAME"),
    ("current_schema", "NAME"),
];
/// How `psql --csv` writes a true boolean.
const CSV_TRUE: &str = "t";
/// The `prokind` of a window function.
const WINDOW_KIND: &str = "w";
/// The `prokind` of a procedure, which a query cannot call.
const PROCEDURE_KIND: &str = "p";
/// The most bits of precision `float(n)` keeps as `real`.
const FLOAT4_BITS: u32 = 24;
/// The pseudo-types an array argument matches.
const ANY_ARRAY_TYPES: [&str; 2] = ["anyarray", "anycompatiblearray"];
/// The concatenation operator, which joins strings, arrays and other values of one type.
const CONCATENATION: &str = "||";
/// Polymorphic results that take the type of the arguments in their parameter group, unmodified.
const ELEMENT_GROUPS: [&[&str]; 2] = [
    &["anyelement", "anynonarray"],
    &["anycompatible", "anycompatiblenonarray"],
];
/// The categories of pseudo-types and `unknown`, whose casts a view column does not keep.
const UNRESOLVED_CATEGORIES: [char; 2] = [PSEUDO_CATEGORY, 'X'];
/// The category of range and multirange types.
const RANGE_CATEGORY: char = 'R';
/// The functions that read one bound of a range or multirange as its element type.
const RANGE_BOUND_FUNCTIONS: [&str; 2] = ["lower", "upper"];
/// Each built-in range and multirange type with its element type.
const RANGE_ELEMENTS: [(&str, &str); 12] = [
    ("int4range", "integer"),
    ("int8range", "bigint"),
    ("numrange", "numeric"),
    ("tsrange", "timestamp without time zone"),
    ("tstzrange", "timestamp with time zone"),
    ("daterange", "date"),
    ("int4multirange", "integer"),
    ("int8multirange", "bigint"),
    ("nummultirange", "numeric"),
    ("tsmultirange", "timestamp without time zone"),
    ("tstzmultirange", "timestamp with time zone"),
    ("datemultirange", "date"),
];
/// The radix prefixes of PostgreSQL's non-decimal integer literals.
const RADIX_PREFIXES: [(&str, u32); 3] = [("0x", 16), ("0o", 8), ("0b", 2)];

/// One overload: its kind (`f`, `a`, `w` or `p`), result, parameters, variadic element and defaults.
struct Overload {
    kind: String,
    returns_set: bool,
    result: String,
    parameters: Vec<String>,
    variadic: Option<String>,
    defaults: usize,
}

/// The fields of one CSV line, with quoted fields unquoted.
fn fields(line: &str) -> Vec<String> {
    let mut fields: Vec<String> = vec![String::new()];
    let mut quoted: bool = false;
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        match (character, quoted) {
            ('"', true) if characters.peek() == Some(&'"') => {
                characters.next();
                if let Some(field) = fields.last_mut() {
                    field.push('"');
                }
            }
            ('"', _) => quoted = !quoted,
            (',', false) => fields.push(String::new()),
            _ => {
                if let Some(field) = fields.last_mut() {
                    field.push(character);
                }
            }
        }
    }
    fields
}

/// The rows of a CSV extract, keyed by its header's column names.
fn rows(csv: &'static str) -> impl Iterator<Item = HashMap<String, String>> {
    let mut lines = csv.lines();
    let header: Vec<String> = lines.next().map(fields).unwrap_or_default();
    lines.map(move |line| header.iter().cloned().zip(fields(line)).collect())
}

/// Every overload of each built-in function, by name.
fn overloads() -> &'static HashMap<String, Vec<Overload>> {
    static OVERLOADS: OnceLock<HashMap<String, Vec<Overload>>> = OnceLock::new();
    OVERLOADS.get_or_init(|| {
        let mut by_name: HashMap<String, Vec<Overload>> = HashMap::new();
        for row in rows(PG_PROC) {
            let field = |name: &str| row.get(name).cloned().unwrap_or_default();
            let parameters: String = field("argument_types");
            let overload = Overload {
                kind: field("kind"),
                returns_set: field("returns_set") == CSV_TRUE,
                result: field("return_type"),
                parameters: parameters
                    .split(',')
                    .filter(|parameter| !parameter.is_empty())
                    .map(str::to_owned)
                    .collect(),
                variadic: Some(field("variadic")).filter(|variadic| !variadic.is_empty()),
                defaults: field("defaults").parse().unwrap_or(0),
            };
            by_name.entry(field("name")).or_default().push(overload);
        }
        by_name
    })
}

/// The category of each built-in type and whether it is its category's preferred type.
fn categories() -> &'static HashMap<String, (char, bool)> {
    static CATEGORIES: OnceLock<HashMap<String, (char, bool)>> = OnceLock::new();
    CATEGORIES.get_or_init(|| {
        rows(PG_TYPE)
            .filter_map(|row| {
                let category: char = row.get("category")?.chars().next()?;
                let preferred: bool = row.get("preferred")? == CSV_TRUE;
                Some((row.get("formatted")?.clone(), (category, preferred)))
            })
            .collect()
    })
}

fn category(formatted: &str) -> Option<char> {
    categories().get(formatted).map(|(category, _)| *category)
}

/// The `format_type` name of a SQLQuack spelling, its modifiers dropped.
fn catalogue_name(spelling: &str) -> String {
    if let Some(element) = spelling.trim().strip_suffix("[]") {
        return format!("{}[]", catalogue_name(element));
    }
    let base: String = spelling
        .split('(')
        .next()
        .unwrap_or(spelling)
        .trim()
        .to_lowercase();
    let name: &str = match base.as_str() {
        "int" | "int4" => "integer",
        "int8" => "bigint",
        "int2" => "smallint",
        "decimal" => "numeric",
        "double" | "float8" | "float" => "double precision",
        "float4" => "real",
        "varchar" => "character varying",
        "char" | "bpchar" => "character",
        "bool" => "boolean",
        "timestamp" => "timestamp without time zone",
        "timestamptz" => "timestamp with time zone",
        "time" => "time without time zone",
        "timetz" => "time with time zone",
        other => other,
    };
    name.to_owned()
}

/// The category PostgreSQL files a value of `family` under; none when the family is unknown.
fn family_category(family: Family) -> Option<char> {
    Some(match family {
        Family::String => 'S',
        Family::Integer | Family::Numeric => 'N',
        Family::Boolean => 'B',
        Family::Date | Family::Time | Family::Timestamp => 'D',
        Family::Interval => 'T',
        Family::Array => 'A',
        _ => return None,
    })
}

/// Whether a parameter of type `parameter` accepts an argument of `family` (none: unknown).
fn accepts(parameter: &str, family: Option<Family>) -> bool {
    let Some(family) = family else {
        return true;
    };
    if ANY_TYPES.contains(&parameter) {
        return true;
    }
    if ANY_ARRAY_TYPES.contains(&parameter) {
        return family == Family::Array;
    }
    match (category(parameter), family_category(family)) {
        (Some(PSEUDO_CATEGORY), _) => false,
        (Some(category), Some(wanted)) => category == wanted,
        _ => true,
    }
}

/// Whether `overload` takes these arguments: their count, and each one's category.
fn viable(overload: &Overload, arguments: &[Option<Family>]) -> bool {
    let count: usize = overload.parameters.len();
    let fits: bool = match &overload.variadic {
        Some(_) => arguments.len() + 1 >= count,
        None => arguments.len() <= count && arguments.len() + overload.defaults >= count,
    };
    fits && arguments.iter().enumerate().all(|(index, family)| {
        let parameter: Option<&String> = match &overload.variadic {
            Some(element) if index + 1 >= count => Some(element),
            _ => overload.parameters.get(index),
        };
        parameter.is_some_and(|parameter| accepts(parameter, *family))
    })
}

/// SQLQuack's spelling of a `format_type` name.
fn spelling(formatted: &str) -> String {
    if let Some(element) = formatted.strip_suffix("[]") {
        return format!("{}[]", spelling(element));
    }
    let spelled: &str = match formatted {
        "character varying" => "VARCHAR",
        "character" => "CHAR",
        "integer" => "INT",
        "double precision" => "DOUBLE",
        "timestamp without time zone" => "TIMESTAMP",
        "timestamp with time zone" => "TIMESTAMPTZ",
        "time without time zone" => "TIME",
        "time with time zone" => "TIMETZ",
        other => return other.to_uppercase(),
    };
    spelled.to_owned()
}

/// The built-in function `expression` calls, by its catalogue name, and its arguments.
fn called(expression: &Expression) -> Option<(String, Vec<&Expression>)> {
    match expression {
        Expression::Function(function) if !function.quoted => {
            let name: &str = function.name.rsplit('.').next().unwrap_or(&function.name);
            Some((name.to_lowercase(), function.args.iter().collect()))
        }
        _ => {
            let (name, arguments) = call(expression)?;
            match overloads().contains_key(&name) {
                true => Some((name, arguments)),
                false => Some((postgres_name(expression), arguments)),
            }
        }
    }
}

/// An argument's type, by its lineage; none for an untyped literal or an unknown type.
fn argument_type<'p>(
    argument: &Expression,
    parts: &'p [(&Expression, Lineage)],
) -> Option<&'p str> {
    if string_literal(argument) {
        return None;
    }
    parts
        .iter()
        .find(|(part, _)| std::ptr::eq(*part, argument))
        .and_then(|(_, lineage)| lineage.data_type.as_deref())
}

/// An argument's family, by its lineage; none for an untyped literal or an unknown type.
fn argument_family(argument: &Expression, parts: &[(&Expression, Lineage)]) -> Option<Family> {
    argument_type(argument, parts)
        .map(type_family)
        .filter(|family| *family != Family::Unknown)
}

/// How well `overload` fits: exact parameter matches first, then preferred parameter types.
fn fit(overload: &Overload, arguments: &[Option<String>]) -> (usize, usize) {
    let parameter = |index: usize| match &overload.variadic {
        Some(element) if index + 1 >= overload.parameters.len() => Some(element),
        _ => overload.parameters.get(index),
    };
    let exact: usize = (0..arguments.len())
        .filter(|index| {
            parameter(*index).is_some() && parameter(*index) == arguments[*index].as_ref()
        })
        .count();
    let preferred: usize = (0..arguments.len())
        .filter(|index| {
            parameter(*index)
                .and_then(|parameter| categories().get(parameter))
                .is_some_and(|(_, preferred)| *preferred)
        })
        .count();
    (exact, preferred)
}

/// One value as PostgreSQL resolves it: `None` an untyped literal, else its catalogue name and spelling.
type Operand = Option<(String, Rc<str>)>;

/// An argument as an operand; none when its type is unknown and it is not an untyped literal.
fn operand(argument: &Expression, parts: &[(&Expression, Lineage)]) -> Option<Operand> {
    if string_literal(argument) || matches!(argument, Expression::Null(_)) {
        return Some(None);
    }
    argument_type(argument, parts).map(|spelled| Some((catalogue_name(spelled), Rc::from(spelled))))
}

/// The types each built-in type casts to implicitly.
fn implicit_casts() -> &'static HashSet<(String, String)> {
    static CASTS: OnceLock<HashSet<(String, String)>> = OnceLock::new();
    CASTS.get_or_init(|| {
        rows(PG_CAST)
            .filter(|row| row.get("context").map(String::as_str) == Some(IMPLICIT_CONTEXT))
            .filter_map(|row| Some((row.get("source")?.clone(), row.get("target")?.clone())))
            .collect()
    })
}

/// Whether a value of `from` becomes `to` without an explicit cast.
fn coerces(from: &str, to: &str) -> bool {
    from == to
        || ANY_TYPES.contains(&to)
        || implicit_casts().contains(&(from.to_owned(), to.to_owned()))
}

fn preferred(formatted: &str) -> bool {
    categories()
        .get(formatted)
        .is_some_and(|(_, preferred)| *preferred)
}

/// One operator signature: its operand types, left first, and its result.
struct Operator {
    operands: Vec<String>,
    result: String,
}

/// Every built-in operator, by its name and kind.
fn operators() -> &'static HashMap<(String, String), Vec<Operator>> {
    static OPERATORS: OnceLock<HashMap<(String, String), Vec<Operator>>> = OnceLock::new();
    OPERATORS.get_or_init(|| {
        let mut by_name: HashMap<(String, String), Vec<Operator>> = HashMap::new();
        for row in rows(PG_OPERATOR) {
            let field = |name: &str| row.get(name).cloned().unwrap_or_default();
            let operands: Vec<String> = [field("left_type"), field("right_type")]
                .into_iter()
                .filter(|operand| !operand.is_empty())
                .collect();
            let operator = Operator {
                operands,
                result: field("result_type"),
            };
            by_name
                .entry((field("name"), field("kind")))
                .or_default()
                .push(operator);
        }
        by_name
    })
}

/// Whether `operator`'s operands equal these inputs, all of them typed.
fn matches_exactly(operator: &Operator, inputs: &[Option<String>]) -> bool {
    operator.operands.len() == inputs.len() && exact_count(operator, inputs) == inputs.len()
}

/// Whether every input converts implicitly to `operator`'s operand; an untyped literal to any real type.
fn accepts_inputs(operator: &Operator, inputs: &[Option<String>]) -> bool {
    if operator.operands.len() != inputs.len() {
        return false;
    }
    for (operand, input) in operator.operands.iter().zip(inputs) {
        let accepted: bool = match input {
            Some(input) => coerces(input, operand),
            None => category(operand) != Some(PSEUDO_CATEGORY),
        };
        if !accepted {
            return false;
        }
    }
    true
}

/// How many of `operator`'s operands are exactly the input's type.
fn exact_count(operator: &Operator, inputs: &[Option<String>]) -> usize {
    let mut count: usize = 0;
    for (operand, input) in operator.operands.iter().zip(inputs) {
        count += usize::from(input.as_ref() == Some(operand));
    }
    count
}

/// How many typed inputs `operator` takes exactly or as its category's preferred type.
fn close_count(operator: &Operator, inputs: &[Option<String>]) -> usize {
    let mut count: usize = 0;
    for (operand, input) in operator.operands.iter().zip(inputs) {
        count +=
            usize::from(input.is_some() && (input.as_ref() == Some(operand) || preferred(operand)));
    }
    count
}

/// Whether `operator` takes each untyped literal as a string, and how many as a preferred type.
fn unknown_fit(operator: &Operator, inputs: &[Option<String>]) -> (bool, usize) {
    let mut textual: bool = true;
    let mut preferred_count: usize = 0;
    for (operand, input) in operator.operands.iter().zip(inputs) {
        if input.is_none() {
            textual &= category(operand) == Some(STRING_CATEGORY);
            preferred_count += usize::from(preferred(operand));
        }
    }
    (textual, preferred_count)
}

/// The inputs with untyped literals taken as the known type, when every typed input has one type.
fn assumed_inputs(inputs: &[Option<String>]) -> Vec<Option<String>> {
    let mut shared: Option<&String> = None;
    for known in inputs.iter().flatten() {
        match shared {
            Some(first) if first != known => return inputs.to_vec(),
            _ => shared = Some(known),
        }
    }
    match shared {
        Some(shared) => vec![Some(shared.clone()); inputs.len()],
        None => inputs.to_vec(),
    }
}

/// The operator PostgreSQL picks for these operands, as `oper_select_candidate` does.
fn resolved_operator(
    name: &str,
    kind: &str,
    inputs: &[Option<String>],
) -> Option<&'static Operator> {
    let signatures: &Vec<Operator> = operators().get(&(name.to_owned(), kind.to_owned()))?;
    let assumed: Vec<Option<String>> = assumed_inputs(inputs);
    if let Some(exact) = signatures
        .iter()
        .find(|operator| matches_exactly(operator, &assumed))
    {
        return Some(exact);
    }
    let viable: Vec<&Operator> = signatures
        .iter()
        .filter(|operator| accepts_inputs(operator, inputs))
        .collect();
    let mut best: Vec<&Operator> = keep_best(viable, |operator| exact_count(operator, inputs));
    best = keep_best(best, |operator| close_count(operator, inputs));
    if best.len() > 1 && inputs.iter().any(Option::is_none) {
        best.retain(|operator| unknown_fit(operator, inputs).0);
        best = keep_best(best, |operator| unknown_fit(operator, inputs).1);
    }
    match best.as_slice() {
        [chosen] if category(&chosen.result) != Some(PSEUDO_CATEGORY) => Some(chosen),
        _ => None,
    }
}

/// The candidates that score highest.
fn keep_best(
    candidates: Vec<&'static Operator>,
    score: impl Fn(&Operator) -> usize,
) -> Vec<&'static Operator> {
    let top: usize = candidates
        .iter()
        .map(|candidate| score(candidate))
        .max()
        .unwrap_or(0);
    candidates
        .into_iter()
        .filter(|candidate| score(candidate) == top)
        .collect()
}

/// Each operand's catalogue name; none for an untyped literal.
fn operand_names(operands: &[Operand]) -> Vec<Option<String>> {
    let mut names: Vec<Option<String>> = Vec::with_capacity(operands.len());
    for operand in operands {
        names.push(operand.as_ref().map(|(name, _)| name.clone()));
    }
    names
}

/// The result of operator `name` over these operands; none when PostgreSQL would refuse them.
fn operator_type(name: &str, kind: &str, operands: &[Operand]) -> Option<Rc<str>> {
    resolved_operator(name, kind, &operand_names(operands))
        .map(|operator| Rc::from(spelling(&operator.result)))
}

/// NULLIF's type: the left operand type of the `=` it resolves, keeping the first value's modifiers.
fn nullif_type(operands: &[Operand]) -> Option<Rc<str>> {
    let left: &String = resolved_operator(EQUALS, BINARY_KIND, &operand_names(operands))?
        .operands
        .first()?;
    match operands.first() {
        Some(Some((name, spelled))) if name == left => Some(spelled.clone()),
        _ => Some(Rc::from(spelling(left))),
    }
}

/// The type values unify to, as `select_common_type` and `select_common_typmod` decide it.
fn unified(operands: &[Operand]) -> Option<Option<Rc<str>>> {
    let mut chosen: Option<&String> = None;
    for (name, _) in operands.iter().flatten() {
        let Some(current) = chosen else {
            chosen = Some(name);
            continue;
        };
        if name == current {
            continue;
        }
        match (category(current), category(name)) {
            (Some(left), Some(right)) if left != right => return Some(None),
            (Some(_), Some(_)) => {}
            _ => return None,
        }
        if !preferred(current) && coerces(current, name) && !coerces(name, current) {
            chosen = Some(name);
        }
    }
    let Some(chosen) = chosen else {
        return Some(Some(Rc::from(TEXT)));
    };
    let spellings: Vec<&Rc<str>> = operands
        .iter()
        .flatten()
        .map(|(_, spelled)| spelled)
        .collect();
    let kept: bool = operands.iter().all(Option::is_some)
        && spellings
            .windows(2)
            .all(|pair| pair[0].eq_ignore_ascii_case(pair[1]));
    match (kept, spellings.first()) {
        (true, Some(first)) => Some(Some((*first).clone())),
        _ => Some(Some(Rc::from(spelling(chosen)))),
    }
}

/// The common type of these values; none when one has an unknown type.
fn common_type(
    values: &[&Expression],
    parts: &[(&Expression, Lineage)],
) -> Option<Option<Rc<str>>> {
    let operands: Vec<Operand> = values
        .iter()
        .map(|value| operand(value, parts))
        .collect::<Option<_>>()?;
    unified(&operands)
}

/// The type of a set operation's column from its branches; a branch's untyped literal adopts the rest.
pub(crate) fn postgres_set_type(branches: &[Lineage]) -> Option<Option<Rc<str>>> {
    let operands: Vec<Operand> = branches
        .iter()
        .map(|branch| {
            let literal: bool = branch.kind == TransformKind::Constant
                && branch.sources.is_empty()
                && branch
                    .data_type
                    .as_deref()
                    .is_none_or(|data_type| data_type == TEXT);
            match (literal, &branch.data_type) {
                (true, _) => Some(None),
                (false, Some(spelled)) => Some(Some((catalogue_name(spelled), spelled.clone()))),
                (false, None) => None,
            }
        })
        .collect::<Option<_>>()?;
    unified(&operands)
}

/// A cast target's spelling as PostgreSQL names the type, with the modifiers it keeps.
pub(crate) fn postgres_cast_spelling(data_type: &DataType) -> Option<String> {
    let length = |name: &str, length: &Option<u32>| match length {
        Some(length) => format!("{name}({length})"),
        None => name.to_owned(),
    };
    Some(match data_type {
        DataType::Float {
            real_spelling: true,
            ..
        } => "REAL".to_owned(),
        DataType::Float {
            precision: Some(bits),
            ..
        } if *bits <= FLOAT4_BITS => "REAL".to_owned(),
        DataType::Float { .. } | DataType::Double { .. } => "DOUBLE".to_owned(),
        DataType::SmallInt { .. } => "SMALLINT".to_owned(),
        DataType::Text => TEXT.to_owned(),
        DataType::VarChar { length: size, .. } => length("VARCHAR", size),
        DataType::Char { length: size } => length("CHAR", &Some(size.unwrap_or(1))),
        DataType::Decimal {
            precision: Some(precision),
            scale,
        } => {
            format!("{NUMERIC}({precision},{})", scale.unwrap_or(0))
        }
        DataType::Decimal {
            precision: None, ..
        } => NUMERIC.to_owned(),
        DataType::Timestamp {
            precision,
            timezone,
        } => length(
            ["TIMESTAMP", "TIMESTAMPTZ"][usize::from(*timezone)],
            precision,
        ),
        DataType::Time {
            precision,
            timezone,
        } => length(["TIME", "TIMETZ"][usize::from(*timezone)], precision),
        DataType::Interval {
            unit: None,
            to: None,
        } => "INTERVAL".to_owned(),
        DataType::Json => "JSON".to_owned(),
        DataType::JsonB => "JSONB".to_owned(),
        DataType::Uuid => "UUID".to_owned(),
        DataType::Array { element_type, .. } => match element_type.as_ref() {
            DataType::Array { .. } => postgres_cast_spelling(element_type)?,
            element => format!("{}[]", postgres_cast_spelling(element)?),
        },
        DataType::Custom { name }
            if category(&catalogue_name(name))
                .is_some_and(|category| UNRESOLVED_CATEGORIES.contains(&category)) =>
        {
            return None;
        }
        other => crate::lineage::_helpers::typing::rules::rendered_type(other)?,
    })
}

/// Whether `expression` calls a built-in window function, which PostgreSQL runs only with OVER.
pub(crate) fn postgres_window_only(expression: &Expression) -> bool {
    called(expression)
        .and_then(|(name, _)| overloads().get(&name))
        .is_some_and(|overloads| {
            overloads
                .iter()
                .any(|overload| overload.kind == WINDOW_KIND)
        })
}

/// The operator an arithmetic node applies, and its operands.
fn arithmetic(expression: &Expression) -> Option<(&'static str, &BinaryOp)> {
    Some(match expression {
        Expression::Add(operation) => ("+", operation),
        Expression::Sub(operation) => ("-", operation),
        Expression::Mul(operation) => ("*", operation),
        Expression::Div(operation) => ("/", operation),
        Expression::Mod(operation) => ("%", operation),
        _ => return None,
    })
}

/// A string literal's type: PostgreSQL resolves an untyped string to text.
pub(crate) fn postgres_literal_type(literal: &Literal) -> Option<&'static str> {
    match literal {
        Literal::String(_) | Literal::EscapeString(_) | Literal::DollarString(_) => Some(TEXT),
        Literal::Number(digits) => Some(integer_value(digits).map_or(NUMERIC, |value| {
            INTEGER_LITERAL_TYPES
                .iter()
                .find(|(bound, _)| value <= *bound)
                .map_or(NUMERIC, |(_, name)| name)
        })),
        _ => None,
    }
}

/// An integer literal's value: decimal or `0x`/`0o`/`0b`, with `_` separators; none otherwise.
fn integer_value(digits: &str) -> Option<i128> {
    let digits: String = digits.replace('_', "");
    let lower: String = digits.to_ascii_lowercase();
    match RADIX_PREFIXES
        .iter()
        .find(|(prefix, _)| lower.starts_with(prefix))
    {
        Some((prefix, radix)) => i128::from_str_radix(&lower[prefix.len()..], *radix).ok(),
        None => digits.parse::<i128>().ok(),
    }
}

/// The type PostgreSQL gives `expression`, when the catalogue decides it; none leaves it to rules.
pub(crate) fn postgres_type(
    expression: &Expression,
    parts: &[(&Expression, Lineage)],
) -> Option<Option<Rc<str>>> {
    if let Expression::Concat(concat) = expression {
        let sides: [Option<Family>; 2] = [
            argument_family(&concat.left, parts),
            argument_family(&concat.right, parts),
        ];
        let textual: bool = sides.contains(&Some(Family::String))
            || string_literal(&concat.left)
            || string_literal(&concat.right);
        if textual && !sides.contains(&Some(Family::Array)) {
            return Some(Some(Rc::from(TEXT)));
        }
        return Some(concatenation_type(&concat.left, &concat.right, parts));
    }
    if let Expression::Case(case) = expression {
        let mut values: Vec<&Expression> = case.else_.iter().collect();
        values.extend(case.whens.iter().map(|(_, value)| value));
        let mut operands: Vec<Operand> = values
            .iter()
            .map(|value| operand(value, parts))
            .collect::<Option<_>>()?;
        if case.else_.is_none() {
            operands.insert(0, None);
        }
        return unified(&operands);
    }
    if let Some((name, operation)) = arithmetic(expression) {
        let operands: Vec<Operand> = [&operation.left, &operation.right]
            .into_iter()
            .map(|side| operand(side, parts))
            .collect::<Option<_>>()?;
        return Some(operator_type(name, BINARY_KIND, &operands));
    }
    if let Expression::Neg(negated) = expression {
        let operands: Vec<Operand> = vec![operand(&negated.this, parts)?];
        return Some(operator_type(NEGATION, PREFIX_KIND, &operands));
    }
    let (name, arguments) = called(expression)?;
    if let Some((_, result)) = VALUE_FUNCTIONS.iter().find(|(value, _)| *value == name) {
        return Some(value_function_type(result, &arguments));
    }
    if RANGE_BOUND_FUNCTIONS.contains(&name.as_str())
        && let [argument] = arguments.as_slice()
        && let Some(range) = argument_type(argument, parts).map(catalogue_name)
        && category(&range) == Some(RANGE_CATEGORY)
    {
        return Some(
            RANGE_ELEMENTS
                .iter()
                .find(|(known, _)| *known == range)
                .map(|(_, element)| Rc::from(spelling(element))),
        );
    }
    match name.as_str() {
        "coalesce" | "greatest" | "least" => return common_type(&arguments, parts),
        "nullif" => {
            let operands: Vec<Operand> = arguments
                .iter()
                .map(|argument| operand(argument, parts))
                .collect::<Option<_>>()?;
            return Some(nullif_type(&operands));
        }
        _ => {}
    }
    let families: Vec<Option<Family>> = arguments
        .iter()
        .map(|argument| argument_family(argument, parts))
        .collect();
    let types: Vec<Option<String>> = arguments
        .iter()
        .map(|argument| argument_type(argument, parts).map(catalogue_name))
        .collect();
    let candidates: Vec<&Overload> = overloads()
        .get(&name)?
        .iter()
        .filter(|overload| overload.kind != PROCEDURE_KIND && !overload.returns_set)
        .filter(|overload| viable(overload, &families))
        .collect();
    let best: (usize, usize) = candidates
        .iter()
        .map(|overload| fit(overload, &types))
        .max()?;
    let mut results: Vec<&str> = candidates
        .iter()
        .filter(|overload| fit(overload, &types) == best)
        .map(|overload| overload.result.as_str())
        .collect();
    results.sort_unstable();
    results.dedup();
    match results.as_slice() {
        [result] if category(result) != Some(PSEUDO_CATEGORY) => {
            Some(Some(Rc::from(spelling(result))))
        }
        [result] => {
            let group: &[&str] = ELEMENT_GROUPS.iter().find(|group| group.contains(result))?;
            let chosen: Vec<&&Overload> = candidates
                .iter()
                .filter(|overload| fit(overload, &types) == best)
                .collect();
            Some(polymorphic_type(group, &chosen, &types))
        }
        _ => None,
    }
}

/// The type a polymorphic result takes: the one type every argument in its group has, without
/// modifiers, as PostgreSQL gives function results; none unless every chosen overload agrees.
fn polymorphic_type(
    group: &[&str],
    overloads: &[&&Overload],
    types: &[Option<String>],
) -> Option<Rc<str>> {
    let mut found: Option<&String> = None;
    for overload in overloads {
        for (index, argument) in types.iter().enumerate() {
            let parameter: Option<&String> = match &overload.variadic {
                Some(element) if index + 1 >= overload.parameters.len() => Some(element),
                _ => overload.parameters.get(index),
            };
            if !parameter.is_some_and(|parameter| group.contains(&parameter.as_str())) {
                continue;
            }
            let argument: &String = argument.as_ref()?;
            if category(argument).is_none_or(|category| category == PSEUDO_CATEGORY) {
                return None;
            }
            match found {
                Some(first) if first != argument => return None,
                _ => found = Some(argument),
            }
        }
    }
    found.map(|found| Rc::from(spelling(found)))
}

/// A SQL value function's type, with the precision a literal argument gives it; none when the
/// precision is not a literal.
fn value_function_type(result: &str, arguments: &[&Expression]) -> Option<Rc<str>> {
    match arguments {
        [] => Some(Rc::from(result)),
        [Expression::Literal(literal)] => match literal.as_ref() {
            Literal::Number(digits) if digits.parse::<u8>().is_ok() => {
                Some(Rc::from(format!("{result}({digits})")))
            }
            _ => None,
        },
        _ => None,
    }
}

/// The type of `left || right` when neither side is a string: an array joined to an array or an
/// element of its own type keeps that array type, and any other pair resolves through
/// `pg_operator`; none when a side's type is unknown or the operator's result is polymorphic.
fn concatenation_type(
    left: &Expression,
    right: &Expression,
    parts: &[(&Expression, Lineage)],
) -> Option<Rc<str>> {
    let operands: Vec<Operand> = vec![operand(left, parts)?, operand(right, parts)?];
    let names: Vec<&String> = operands.iter().flatten().map(|(name, _)| name).collect();
    let [left_name, right_name] = names.as_slice() else {
        return None;
    };
    let element = |name: &str| name.strip_suffix("[]").map(str::to_owned);
    let array: Option<String> = match (element(left_name), element(right_name)) {
        (Some(left), Some(right)) if left == right => Some(left),
        (Some(left), None) if left == **right_name => Some(left),
        (None, Some(right)) if right == **left_name => Some(right),
        (Some(_), _) | (_, Some(_)) => return None,
        (None, None) => None,
    };
    match array {
        Some(element) => Some(Rc::from(spelling(&format!("{element}[]")))),
        None => operator_type(CONCATENATION, BINARY_KIND, &operands),
    }
}
