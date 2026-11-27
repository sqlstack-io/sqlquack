//! Function calls: the aggregates and scalar functions DuckDB computes as Databricks does.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::expression;
use crate::databricks::conversion::_helpers::literals::typed;
use crate::databricks::conversion::_helpers::operators::{as_type, common_type};
use crate::databricks::conversion::_helpers::scope::Site;
use crate::databricks::conversion::_helpers::tree::{
    field, list, node_kind, refused_function, understood,
};
use crate::databricks::conversion::constants::{
    AGGREGATE_RULE, CONCAT_RULE, COUNT_NAME, MIN_LIST_ARGUMENTS, SAME_FUNCTION_RULE, SAME_FUNCTIONS,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Signature};

/// A call the parser kept as a function or an aggregate function.
pub(crate) fn call(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let call = understood(payload, kind, &["name", "args", "set_quantifier"])?;
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
        "COUNT" | "MIN" | "MAX" => aggregate(&name, args, distinct, site),
        _ if distinct => Err(refused_function(
            &name,
            format!("{name} is no aggregate DISTINCT applies to"),
        )),
        "CONCAT" => concat(args, site),
        _ => same_function(&name, args, site),
    }
}

/// `COUNT(*)`, `COUNT([DISTINCT] x)`, `MIN(x)` and `MAX(x)`, whose extremes DISTINCT leaves alone.
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
    match (name, value.data_type) {
        ("COUNT", _) => {
            let distinct = match distinct {
                true => "DISTINCT ",
                false => "",
            };
            Ok(typed(
                format!("count({distinct}{})", value.sql),
                DatabricksType::BigInt,
            ))
        }
        (_, DatabricksType::Null) => Err(refused_function(
            name,
            format!("no rule types {name} of a bare NULL"),
        )),
        (_, data_type) => Ok(typed(
            format!("{}({})", name.to_ascii_lowercase(), value.sql),
            data_type,
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
        (Signature::StringToString, [DatabricksType::String]) => {
            (vec![values[0].sql.clone()], DatabricksType::String)
        }
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
