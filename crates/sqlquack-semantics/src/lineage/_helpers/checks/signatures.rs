//! Built-in function signatures: argument counts and argument families, as polyglot records them.

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;

use crate::lineage::_helpers::checks::coercion::{
    Engine, comparable, invalid_literal_for, literal_coerces, predicate, string_literal,
    temporal_function_argument,
};
use crate::lineage::_helpers::checks::families::Family;
use crate::lineage::_helpers::checks::issues::{issue, type_location};
use crate::lineage::constants::TYPE_PHASE;
use crate::lineage::models::NameFinding;

/// DuckDB's STRFTIME, which also takes its format first.
const STRFTIME: &str = "strftime";
const STRFTIME_ARGUMENTS: usize = 2;
/// LAG, whose Snowflake offset must be a constant.
const LAG: &str = "lag";
/// NULLIF, whose arguments compare rather than unify, so a mismatch only warns.
const NULLIF: &str = "nullif";

/// What a built-in argument position accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Accepts {
    Any,
    Numeric,
    Integer,
    String,
    Sequence,
    Temporal,
    Boolean,
}

impl Accepts {
    fn name(self) -> &'static str {
        match self {
            Self::Any => "Any",
            Self::Numeric => "Numeric",
            Self::Integer => "Integer",
            Self::String => "String",
            Self::Sequence => "Sequence",
            Self::Temporal => "Temporal",
            Self::Boolean => "Boolean",
        }
    }
}

/// A Snowflake built-in's argument count: at least, and at most when bounded.
fn builtin_arity(engine: Engine, name: &str) -> Option<(usize, Option<usize>)> {
    if engine != Engine::Snowflake {
        return None;
    }
    Some(match name {
        "sum" | "avg" | "min" | "max" | "stddev" | "stddev_pop" | "stddev_samp" | "variance"
        | "var_pop" | "var_samp" | "abs" | "sqrt" | "upper" | "lower" | "length"
        | "char_length" | "ntile" | "first_value" | "last_value" => (1, Some(1)),
        "date_trunc" | "date_part" | "left" | "right" | "nullif" | "ifnull" | "nvl" | "power"
        | "pow" | "mod" => (2, Some(2)),
        "datediff" | "dateadd" | "split_part" | "iff" => (3, Some(3)),
        "substring" | "substr" | "replace" => (2, Some(3)),
        "lag" | "lead" | "round" => (1, Some(3)),
        "floor" | "ceil" | "ceiling" | "trim" | "ltrim" | "rtrim" | "listagg" => (1, Some(2)),
        "row_number" | "rank" | "dense_rank" => (0, Some(0)),
        "coalesce" | "greatest" | "least" | "concat" => (1, None),
        _ => return None,
    })
}

/// BigQuery's built-ins' argument families, by canonical name.
fn bigquery_argument_types(name: &str) -> Option<&'static [Accepts]> {
    use Accepts::{Any, Boolean, Integer, Numeric, Sequence, Temporal};
    Some(match name {
        "sum" | "avg" | "stddev" | "stddev_pop" | "stddev_samp" | "variance" | "var_pop"
        | "var_samp" => &[Numeric],
        "min" | "max" | "count" | "row_number" | "rank" | "dense_rank" | "concat" | "coalesce"
        | "nullif" | "ifnull" | "greatest" | "least" | "first_value" | "last_value" => &[Any],
        "ntile" => &[Integer],
        "lag" | "lead" => &[Any, Integer, Any],
        "abs" | "round" | "floor" | "ceil" | "ceiling" => &[Numeric, Integer],
        "sqrt" | "power" | "pow" | "mod" => &[Numeric, Numeric],
        "upper" | "lower" | "trim" | "ltrim" | "rtrim" => &[Sequence, Sequence],
        "substring" | "substr" => &[Sequence, Integer, Integer],
        "left" | "right" => &[Sequence, Integer],
        "replace" => &[Sequence, Sequence, Sequence],
        "string_agg" => &[Sequence, Sequence],
        "date_trunc" | "dateadd" => &[Temporal],
        "datediff" => &[Temporal, Temporal],
        "if" | "iff" => &[Boolean, Any, Any],
        _ => return None,
    })
}

/// A built-in's argument families; absence means unchecked, never invalid.
fn argument_types(engine: Engine, name: &str) -> Option<&'static [Accepts]> {
    use Accepts::{Any, Boolean, Integer, Numeric, Sequence, String, Temporal};
    if engine == Engine::BigQuery {
        return bigquery_argument_types(name);
    }
    let duckdb: bool = engine == Engine::DuckDB;
    Some(match name {
        "sum" | "avg" if duckdb => &[Any],
        "sum" | "avg" | "stddev" | "stddev_pop" | "stddev_samp" | "variance" | "var_pop"
        | "var_samp" => &[Numeric],
        "min" | "max" | "first_value" | "last_value" | "count" | "row_number" | "rank"
        | "dense_rank" | "concat" | "coalesce" | "nullif" | "ifnull" | "nvl" | "greatest"
        | "least" => &[Any],
        "ntile" => &[Integer],
        "lag" | "lead" => &[Any, Integer, Any],
        "abs" | "round" | "floor" | "ceil" | "ceiling" => &[Numeric, Integer],
        "sqrt" | "power" | "pow" | "mod" => &[Numeric, Numeric],
        "upper" | "lower" | "trim" | "ltrim" | "rtrim" => &[String, String],
        "length" | "char_length" => &[Sequence],
        "substring" | "substr" => &[String, Integer, Integer],
        "left" | "right" => &[String, Integer],
        "replace" => &[String, String, String],
        "split_part" => &[String, String, Integer],
        "regexp_matches" | "regexp_like" | "regexp_full_match" => &[String, String, String],
        "regexp_replace" => &[String, String, String, String],
        "regexp_extract" => &[String, String, Any, String],
        "string_agg" | "listagg" => &[Any, String],
        "date_trunc" | "extract" | "date_part" => &[String, Temporal],
        "date_diff" | "datediff" => &[String, Temporal, Temporal],
        "dateadd" => &[String, Integer, Temporal],
        "strftime" => &[Temporal, String],
        "to_char" => &[Any, String],
        "if" | "iff" => &[Boolean, Any, Any],
        _ => return None,
    })
}

/// Whether the engine records argument families for the function or typed node `name`.
pub(crate) fn has_signature(engine: Engine, name: &str) -> bool {
    argument_types(engine, name).is_some()
}

/// A function's name as signatures know it: its last dotted part's canonical name, in lower case.
pub(crate) fn dispatch_name(name: &str) -> String {
    sqlquack_syntax::canonical_call_name(name)
}

/// The built-in call `node` makes: its name and arguments, when signatures know its form.
pub(crate) fn call(node: &Expression) -> Option<(String, Vec<&Expression>)> {
    let arguments: Vec<&Expression> = match node {
        Expression::Function(function) => {
            return Some((
                dispatch_name(&function.name),
                function.args.iter().collect(),
            ));
        }
        Expression::AggregateFunction(function) => {
            return Some((
                dispatch_name(&function.name),
                function.args.iter().collect(),
            ));
        }
        Expression::Sum(function)
        | Expression::Avg(function)
        | Expression::Min(function)
        | Expression::Max(function)
        | Expression::Stddev(function)
        | Expression::StddevPop(function)
        | Expression::StddevSamp(function)
        | Expression::Variance(function)
        | Expression::VarPop(function)
        | Expression::VarSamp(function) => vec![&function.this],
        Expression::Lag(function) | Expression::Lead(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.offset.iter());
            arguments.extend(function.default.iter());
            arguments
        }
        Expression::NTile(function) => {
            return named("ntile", function.num_buckets.iter().collect());
        }
        Expression::NullIf(function) => {
            return named("nullif", vec![&function.this, &function.expression]);
        }
        Expression::IfNull(function) => {
            return named("ifnull", vec![&function.this, &function.expression]);
        }
        Expression::FirstValue(function) | Expression::LastValue(function) => vec![&function.this],
        Expression::Coalesce(function)
        | Expression::Greatest(function)
        | Expression::Least(function) => function.expressions.iter().collect(),
        Expression::StringAgg(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.separator.iter());
            arguments
        }
        Expression::ListAgg(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.separator.iter());
            return named("listagg", arguments);
        }
        Expression::Upper(function)
        | Expression::Lower(function)
        | Expression::LTrim(function)
        | Expression::RTrim(function)
        | Expression::Abs(function)
        | Expression::Sqrt(function)
        | Expression::Length(function) => vec![&function.this],
        Expression::Substring(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this, &function.start];
            arguments.extend(function.length.iter());
            arguments
        }
        Expression::Replace(function) => vec![&function.this, &function.old, &function.new],
        Expression::Left(function) | Expression::Right(function) => {
            vec![&function.this, &function.length]
        }
        Expression::Round(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.decimals.iter());
            arguments
        }
        Expression::Floor(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.scale.iter());
            arguments
        }
        Expression::Ceil(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.decimals.iter());
            arguments
        }
        Expression::Power(function) => vec![&function.this, &function.expression],
        Expression::Trim(function) => {
            let mut arguments: Vec<&Expression> = vec![&function.this];
            arguments.extend(function.characters.iter());
            arguments
        }
        _ => return None,
    };
    Some((node.variant_name().to_owned(), arguments))
}

fn named<'a>(name: &str, arguments: Vec<&'a Expression>) -> Option<(String, Vec<&'a Expression>)> {
    Some((name.to_owned(), arguments))
}

/// What checking a built-in call found, and whether the engine records its signature.
pub(crate) struct Checked {
    pub(crate) findings: Vec<NameFinding>,
    pub(crate) signed: bool,
}

/// A wrong argument family, unless the value is unknown or accepted.
pub(crate) fn argument_finding(
    (name, index): (&str, usize),
    (family, expected): (Family, &str),
    valid: bool,
    node: &Expression,
) -> Option<NameFinding> {
    if family == Family::Unknown || valid {
        return None;
    }
    let code: &'static str = match family.fully_modelled() {
        true => "E213",
        false => "W216",
    };
    Some(issue(
        (code, TYPE_PHASE),
        format!(
            "Function '{name}' argument {} expects {expected}, found {}",
            index + 1,
            family.name()
        ),
        None,
        node,
    ))
}

/// The findings of a built-in call `node`: its argument count, families and unified values.
pub(crate) fn check_call(
    node: &Expression,
    engine: Engine,
    family: &dyn Fn(&Expression) -> Family,
) -> Option<Checked> {
    let (name, mut arguments) = call(node)?;
    let arity_name: String = match node {
        Expression::Function(function) => function.name.to_ascii_lowercase(),
        _ => name.clone(),
    };
    let mut findings: Vec<NameFinding> = Vec::new();
    if let Some((least, most)) = builtin_arity(engine, &arity_name)
        && (arguments.len() < least || most.is_some_and(|most| arguments.len() > most))
    {
        findings.push(issue(
            ("E203", TYPE_PHASE),
            format!(
                "Invalid arity for built-in function '{arity_name}': got {}",
                arguments.len()
            ),
            None,
            node,
        ));
    }
    let Some(accepted) = argument_types(engine, &name) else {
        return Some(Checked {
            findings,
            signed: false,
        });
    };
    if name == STRFTIME
        && engine == Engine::DuckDB
        && arguments.len() == STRFTIME_ARGUMENTS
        && family(arguments[1]).is_temporal()
    {
        arguments.swap(0, 1);
    }
    if engine == Engine::Snowflake
        && name == LAG
        && arguments.get(1).is_some_and(|offset| {
            offset
                .dfs()
                .any(|node| matches!(node, Expression::Column(_) | Expression::Subquery(_)))
        })
    {
        findings.push(issue(
            ("E213", TYPE_PHASE),
            "LAG offset must be a constant expression".to_owned(),
            None,
            node,
        ));
    }
    for (index, (argument, expected)) in arguments.iter().zip(accepted.iter()).enumerate() {
        findings.extend(argument_findings(
            (&name, index),
            (argument, family(argument)),
            (*expected, engine),
            node,
        ));
    }
    if engine == Engine::DuckDB
        && let Some(first) = arguments.first()
    {
        let first_family: Family = family(first);
        let duckdb_rule = match name.as_str() {
            "avg" => Some((
                "numeric or temporal",
                first_family.is_numeric() || first_family.is_temporal(),
            )),
            "sum" => Some((
                "numeric or boolean",
                first_family.is_numeric() || first_family == Family::Boolean,
            )),
            _ => None,
        };
        if let Some((expected, valid)) = duckdb_rule {
            findings.extend(argument_finding(
                (&name, 0),
                (first_family, expected),
                valid,
                node,
            ));
        }
    }
    if matches!(
        name.as_str(),
        "coalesce" | "ifnull" | "nvl" | "greatest" | "least" | "nullif"
    ) {
        let lenient: bool =
            name == NULLIF && !matches!(engine, Engine::Postgres | Engine::BigQuery);
        findings.extend(unify(&arguments, engine, family, (lenient, node)));
    }
    if matches!(node, Expression::Coalesce(_)) && arguments.is_empty() {
        findings.push(issue(
            ("E203", TYPE_PHASE),
            "COALESCE requires at least one argument".to_owned(),
            None,
            node,
        ));
    }
    Some(Checked {
        findings,
        signed: true,
    })
}

/// One argument's findings: a runtime string conversion, and a family its position refuses.
fn argument_findings(
    (name, index): (&str, usize),
    (argument, family): (&Expression, Family),
    (expected, engine): (Accepts, Engine),
    node: &Expression,
) -> Vec<NameFinding> {
    let valid: bool = match expected {
        Accepts::Any => true,
        Accepts::Numeric => family.is_numeric(),
        Accepts::Integer => family == Family::Integer || family.is_numeric(),
        Accepts::String => family == Family::String,
        Accepts::Sequence => matches!(family, Family::String | Family::Binary | Family::Array),
        Accepts::Temporal => temporal_function_argument(name, engine, (argument, family)),
        Accepts::Boolean => predicate(engine, family),
    };
    let implicit: bool = engine == Engine::Snowflake
        && (matches!(expected, Accepts::String | Accepts::Sequence)
            || (family == Family::String
                && (matches!(expected, Accepts::Numeric | Accepts::Integer)
                    || (expected == Accepts::Temporal
                        && temporal_function_argument(name, engine, (argument, family))))));
    let literal: bool = match expected {
        Accepts::Integer | Accepts::Numeric => literal_coerces(engine, argument, Family::Numeric),
        Accepts::Boolean => literal_coerces(engine, argument, Family::Boolean),
        _ => false,
    };
    let interval_aggregate: bool =
        family == Family::Interval && matches!(name, "sum" | "avg") && engine != Engine::DuckDB;
    let target: Option<Family> = match expected {
        Accepts::Numeric | Accepts::Integer => Some(Family::Numeric),
        Accepts::Temporal => Some(Family::Timestamp),
        Accepts::Boolean => Some(Family::Boolean),
        _ => None,
    };
    let mut findings: Vec<NameFinding> = Vec::new();
    if family == Family::String
        && (valid || implicit || literal)
        && target.is_some_and(|target| {
            !string_literal(argument) || invalid_literal_for(argument, target)
        })
    {
        findings.push(issue(
            ("W216", TYPE_PHASE),
            format!(
                "Function '{name}' argument {} uses a runtime conversion",
                index + 1
            ),
            None,
            node,
        ));
    }
    findings.extend(argument_finding(
        (name, index),
        (family, expected.name()),
        valid || implicit || literal || interval_aggregate,
        node,
    ));
    findings
}

/// Values that must share a family, as a CASE's results or COALESCE's arguments.
pub(crate) fn unify(
    values: &[&Expression],
    engine: Engine,
    family: &dyn Fn(&Expression) -> Family,
    (lenient, node): (bool, &Expression),
) -> Option<NameFinding> {
    let base: &Expression = values
        .iter()
        .find(|value| !string_literal(value) && family(value) != Family::Unknown)?;
    let base_family: Family = family(base);
    values.iter().find_map(|value| {
        let other: Family = family(value);
        let boolean_numeric: bool = engine == Engine::DuckDB
            && ((base_family == Family::Boolean && other.is_numeric())
                || (other == Family::Boolean && base_family.is_numeric()));
        (!boolean_numeric && !comparable(engine, (base, value), (base_family, other))).then(|| {
            issue(
                (if lenient { "W216" } else { "E213" }, TYPE_PHASE),
                format!("Cannot unify {} and {}", base_family.name(), other.name()),
                None,
                node,
            )
        })
    })
}

/// `findings` placed where polyglot places a node's type findings; only unplaced ones read the subtree.
pub(crate) fn located(mut findings: Vec<NameFinding>, node: &Expression) -> Vec<NameFinding> {
    if findings.iter().all(|finding| finding.span.is_some()) {
        return findings;
    }
    let span = type_location(node);
    for finding in &mut findings {
        if finding.span.is_none() {
            finding.span = span;
        }
    }
    findings
}
