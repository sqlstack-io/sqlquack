//! PostgreSQL's result types from its 18.6 catalogue: string literals, `||` and built-in calls.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Literal;

use crate::lineage::_helpers::checks::coercion::string_literal;
use crate::lineage::_helpers::checks::families::{Family, type_family};
use crate::lineage::_helpers::checks::signatures::call;
use crate::lineage::_helpers::tracing::postgres_names::postgres_name;
use crate::lineage::models::Lineage;

/// The built-in functions of `pg_catalog`, one overload per row, read from the 18.6 server.
const PG_PROC: &str = include_str!("postgres_catalog/pg_proc.csv");
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
/// The `prokind` of a procedure, which a query cannot call.
const PROCEDURE_KIND: &str = "p";
/// The pseudo-types an array argument matches.
const ANY_ARRAY_TYPES: [&str; 2] = ["anyarray", "anycompatiblearray"];

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

/// The common type of values whose untyped literals adopt the others' type, as PostgreSQL unifies.
fn common_type(
    values: &[&Expression],
    parts: &[(&Expression, Lineage)],
) -> Option<Option<Rc<str>>> {
    let typed: Vec<&str> = values
        .iter()
        .filter_map(|value| argument_type(value, parts))
        .collect();
    match typed.split_first() {
        None if values.iter().all(|value| string_literal(value)) => Some(Some(Rc::from(TEXT))),
        Some((first, rest)) if rest.iter().all(|other| other == first) => {
            Some(Some(Rc::from(*first)))
        }
        _ => None,
    }
}

/// A string literal's type: PostgreSQL resolves an untyped string to text.
pub(crate) fn postgres_literal_type(literal: &Literal) -> Option<&'static str> {
    match literal {
        Literal::String(_) | Literal::EscapeString(_) | Literal::DollarString(_) => Some(TEXT),
        _ => None,
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
        return (textual && !sides.contains(&Some(Family::Array))).then(|| Some(Rc::from(TEXT)));
    }
    if let Expression::Case(case) = expression {
        let mut values: Vec<&Expression> = case.whens.iter().map(|(_, value)| value).collect();
        values.extend(case.else_.iter());
        return common_type(&values, parts);
    }
    let (name, arguments) = called(expression)?;
    if let Some((_, result)) = VALUE_FUNCTIONS.iter().find(|(value, _)| *value == name) {
        return Some(Some(Rc::from(*result)));
    }
    match name.as_str() {
        "coalesce" | "greatest" | "least" => return common_type(&arguments, parts),
        "nullif" => {
            let first: Option<&Expression> = arguments.first().copied();
            let textual: bool = first.is_some_and(|first| {
                argument_family(first, parts) == Some(Family::String)
                    && argument_type(first, parts).map(catalogue_name).as_deref()
                        != Some("character")
            });
            return textual.then(|| Some(Rc::from(TEXT)));
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
        _ => None,
    }
}
