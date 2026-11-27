//! Function calls: the aggregates and scalar functions DuckDB computes as Databricks does.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::literals::{cast_value, typed};
use crate::databricks::conversion::_helpers::expressions::operators::{as_type, common_type};
use crate::databricks::conversion::_helpers::functions::rewrites::{applied, rewrite};
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::{
    field, list, node_kind, refused_function, understood,
};
use crate::databricks::conversion::constants::{
    AGGREGATE_RULE, AGGREGATES, CAST_FUNCTIONS, CAST_RULE, CONCAT_RULE, COUNT_NAME,
    MAX_DECIMAL_PRECISION, MIN_LIST_ARGUMENTS, SAME_FUNCTION_RULE, SAME_FUNCTIONS,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Signature};

/// A call the parser kept as a function or an aggregate function.
pub(crate) fn call(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let call = understood(
        payload,
        kind,
        &["name", "args", "set_quantifier", "from_separator"],
    )?;
    let name = field(call, "name")
        .as_str()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let distinct = match field(call, "set_quantifier").as_str() {
        None | Some("none" | "all") => false,
        Some("distinct") => true,
        Some(other) => {
            return Err(refused_function(
                &name,
                format!("no rule reads the set quantifier `{other}`"),
            ));
        }
    };
    let args = list(call, "args");
    match name.as_str() {
        _ if AGGREGATES.iter().any(|(aggregate, _)| *aggregate == name) => {
            aggregate(&name, args, distinct, site)
        }
        _ if distinct => Err(refused_function(
            &name,
            format!("{name} is no aggregate DISTINCT applies to"),
        )),
        "CONCAT" => concat(args, site),
        _ if CAST_FUNCTIONS.iter().any(|(function, _)| *function == name) => {
            cast_function(&name, args, site)
        }
        _ => match rewrite(&name) {
            Some(rewrite) => applied(&name, rewrite, &arguments(args, site)?, site),
            None => same_function(&name, args, site),
        },
    }
}

/// An aggregate of one argument or `COUNT(*)`, typed as Databricks types it: an INT SUM as BIGINT, COUNT_IF 0 over NULLs.
fn aggregate(
    name: &str,
    args: &[Value],
    distinct: bool,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    if !site.aggregates {
        return Err(refused_function(
            name,
            format!("no rule converts {name} where an aggregate cannot appear"),
        ));
    }
    let [argument] = args else {
        return Err(refused_function(
            name,
            format!("only {name} of one argument converts"),
        ));
    };
    let _rule = site.log.rule(AGGREGATE_RULE);
    if let (true, Some(("star", star))) = (name == COUNT_NAME && !distinct, node_kind(argument)) {
        understood(star, "star", &[])?;
        return Ok(typed("count(*)".to_owned(), DatabricksType::BigInt));
    }
    let value = expression(argument, site.without_aggregates())?;
    let function = AGGREGATES
        .iter()
        .find(|(databricks, _)| *databricks == name)
        .map_or("", |(_, duckdb)| *duckdb);
    let quantified = match distinct {
        true => format!("DISTINCT {}", value.sql),
        false => value.sql.clone(),
    };
    let call = format!("{function}({quantified})");
    use DatabricksType as T;
    let result = match (name, value.data_type) {
        ("COUNT", _) => Some(typed(call, T::BigInt)),
        (_, T::Null) => None,
        ("MIN" | "MAX", data_type) => Some(typed(call, data_type)),
        ("SUM", T::TinyInt | T::SmallInt | T::Int) => {
            Some(typed(format!("CAST({call} AS BIGINT)"), T::BigInt))
        }
        ("SUM", T::Decimal { precision, scale }) => {
            let sum = T::Decimal {
                precision: (precision + 10).min(MAX_DECIMAL_PRECISION),
                scale,
            };
            Some(typed(format!("CAST({call} AS {})", sum.duckdb_name()), sum))
        }
        ("BOOL_AND" | "EVERY" | "BOOL_OR" | "SOME" | "ANY", T::Boolean) => {
            Some(typed(call, T::Boolean))
        }
        ("COUNT_IF", T::Boolean) if !distinct => Some(typed(
            format!("CAST(coalesce({call}, 0) AS BIGINT)"),
            T::BigInt,
        )),
        ("BIT_AND" | "BIT_OR", data_type) if data_type.is_integral() => {
            Some(typed(call, data_type))
        }
        ("BIT_XOR", data_type) if data_type.is_integral() && !distinct => {
            Some(typed(call, data_type))
        }
        _ => None,
    };
    result.ok_or_else(|| {
        refused_function(
            name,
            format!(
                "no rule converts {name} of {}{}",
                value.data_type.name(),
                match distinct {
                    true => " with DISTINCT",
                    false => "",
                }
            ),
        )
    })
}

/// A function named after a type, such as `BIGINT(x)`: a cast of its one argument to that type.
fn cast_function(name: &str, args: &[Value], site: Site<'_>) -> Result<Emitted, Refusal> {
    let target = CAST_FUNCTIONS
        .iter()
        .find(|(function, _)| *function == name)
        .map(|(_, target)| *target);
    let values = arguments(args, site)?;
    let _rule = site.log.rule(CAST_RULE);
    match (target, values.as_slice()) {
        (Some(target), [value]) => cast_value(value, target).ok_or_else(|| {
            refused_function(
                name,
                format!(
                    "no rule shows DuckDB casts {} to {} as Databricks does",
                    value.data_type.name(),
                    target.name()
                ),
            )
        }),
        _ => Err(refused_function(
            name,
            format!("only {name} of one argument converts"),
        )),
    }
}

/// `CONCAT` of strings as `||`: NULL when any argument is NULL, as in Databricks.
fn concat(args: &[Value], site: Site<'_>) -> Result<Emitted, Refusal> {
    let values = arguments(args, site)?;
    let _rule = site.log.rule(CONCAT_RULE);
    match !values.is_empty()
        && values
            .iter()
            .all(|value| value.data_type == DatabricksType::String)
    {
        true => Ok(typed(
            format!(
                "({})",
                values
                    .iter()
                    .map(|value| value.sql.as_str())
                    .collect::<Vec<_>>()
                    .join(" || ")
            ),
            DatabricksType::String,
        )),
        false => Err(refused_function(
            "CONCAT",
            "only CONCAT of strings converts".to_owned(),
        )),
    }
}

/// A function DuckDB has with the same results, when the arguments fit its signature.
fn same_function(name: &str, args: &[Value], site: Site<'_>) -> Result<Emitted, Refusal> {
    let Some((_, duckdb, signature)) = SAME_FUNCTIONS
        .iter()
        .find(|(databricks, _, _)| *databricks == name)
    else {
        return Err(refused_function(
            name,
            format!("no rule converts the function {name}"),
        ));
    };
    let values = arguments(args, site)?;
    let _rule = site.log.rule(SAME_FUNCTION_RULE);
    let types: Vec<DatabricksType> = values.iter().map(|value| value.data_type).collect();
    let (sqls, data_type) = match (signature, types.as_slice()) {
        (Signature::TrimSpaces, [DatabricksType::String]) => (
            vec![values[0].sql.clone(), "' '".to_owned()],
            DatabricksType::String,
        ),
        (Signature::SameNumber, [data_type]) if data_type.is_numeric() => {
            (vec![values[0].sql.clone()], *data_type)
        }
        (Signature::SameType, types) if types.len() >= MIN_LIST_ARGUMENTS => {
            match common_type(&values) {
                Some(common) if common != DatabricksType::Null => (
                    values.iter().map(|value| as_type(value, common)).collect(),
                    common,
                ),
                _ => return Err(mismatched(name, types)),
            }
        }
        (Signature::TwoOfOneType, [_, _]) => match common_type(&values) {
            Some(common) if common != DatabricksType::Null => (
                values.iter().map(|value| as_type(value, common)).collect(),
                common,
            ),
            _ => return Err(mismatched(name, &types)),
        },
        _ => return Err(mismatched(name, &types)),
    };
    Ok(typed(format!("{duckdb}({})", sqls.join(", ")), data_type))
}

fn mismatched(name: &str, types: &[DatabricksType]) -> Refusal {
    let names: Vec<String> = types.iter().map(|data_type| data_type.name()).collect();
    refused_function(
        name,
        format!(
            "no rule converts {name} of ({}): the arguments are outside its signature",
            names.join(", ")
        ),
    )
}

/// Each argument converted at the call's own site.
fn arguments(args: &[Value], site: Site<'_>) -> Result<Vec<Emitted>, Refusal> {
    args.iter()
        .map(|argument| expression(argument, site))
        .collect()
}
