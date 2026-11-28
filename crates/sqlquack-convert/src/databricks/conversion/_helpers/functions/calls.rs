//! Function calls: the aggregates and scalar functions DuckDB computes as Databricks does.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::literals::{cast_value, typed};
use crate::databricks::conversion::_helpers::expressions::operators::{as_type, common_type};
use crate::databricks::conversion::_helpers::expressions::texts::{
    narrowed_arguments, text_arguments,
};
use crate::databricks::conversion::_helpers::functions::rewrites::small_integer;
use crate::databricks::conversion::_helpers::functions::rewrites::{applied, rewrite};
use crate::databricks::conversion::_helpers::support::scope::Site;
use crate::databricks::conversion::_helpers::support::tree::{
    field, flag, list, node_kind, outside, outside_function, refused, refused_function, understood,
};
use crate::databricks::conversion::constants::{
    AGGREGATE_RULE, AGGREGATES, CAST_FUNCTIONS, CAST_RULE, COALESCE_NAME, CONCAT_RULE, COUNT_NAME,
    DECODE_CHARSET_ARGUMENTS, DOUBLE_EXACT_INTEGERS, EXACT_FRACTION_STEPS, INEXACT_SUM_ERROR,
    MAX_DECIMAL_PRECISION, MIN_LIST_ARGUMENTS, ORDERING_FUNCTIONS, SAME_FUNCTION_RULE,
    SAME_FUNCTIONS, WHOLE_AGGREGATES, WITHIN_GROUP_KIND,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal, Signature};

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
            let call = Aggregate {
                name: &name,
                args,
                distinct,
                over: None,
            };
            aggregate(&call, site)
        }
        _ if distinct => Err(refused_function(
            &name,
            format!("{name} is no aggregate DISTINCT applies to"),
        )),
        "CONCAT" => concat(args, site),
        "REGR_COUNT" => pair_count(args, site),
        "DECODE" if args.len() != DECODE_CHARSET_ARGUMENTS => Err(refused_function(
            &name,
            "no rule converts DECODE of a value and its search results".to_owned(),
        )),
        _ if CAST_FUNCTIONS.iter().any(|(function, _)| *function == name) => {
            cast_function(&name, args, site)
        }
        _ => match rewrite(&name) {
            Some(rewrite) => applied(
                &name,
                rewrite,
                &narrowed_arguments(&name, text_arguments(&name, arguments(args, site)?, site)),
                site,
            ),
            None => same_function(&name, args, site),
        },
    }
}

/// An aggregate call: its name, arguments, set quantifier, and `OVER` clause when it is a window function.
pub(crate) struct Aggregate<'a> {
    pub(crate) name: &'a str,
    pub(crate) args: &'a [Value],
    pub(crate) distinct: bool,
    pub(crate) over: Option<&'a str>,
}

/// An aggregate of one argument or `COUNT(*)`, typed as Databricks types it: an INT SUM as BIGINT, COUNT_IF 0 over NULLs.
pub(crate) fn aggregate(call: &Aggregate<'_>, site: Site<'_>) -> Result<Emitted, Refusal> {
    let Aggregate {
        name,
        args,
        distinct,
        over,
    } = *call;
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
    if over.is_some() && WHOLE_AGGREGATES.contains(&name) {
        return Err(refused_function(
            name,
            format!("no rule converts {name} as a window function"),
        ));
    }
    let _rule = site.log.rule(AGGREGATE_RULE);
    if let (true, Some(("star", star))) = (name == COUNT_NAME && !distinct, node_kind(argument)) {
        understood(star, "star", &[])?;
        return Ok(typed(
            format!("count(*){}", over.unwrap_or_default()),
            DatabricksType::BigInt,
        ));
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
    let call = format!("{function}({quantified}){}", over.unwrap_or_default());
    use DatabricksType as T;
    let result = match (name, value.data_type) {
        ("COUNT", _) => Some(typed(call, T::BigInt)),
        (_, T::Null) => None,
        ("MIN" | "MAX", T::Array(_)) => None,
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
        ("AVG" | "MEAN" | "TRY_AVG", data_type) if data_type.is_integral() && !distinct => {
            let x = &value.sql;
            Some(typed(
                format!(
                    "CASE WHEN count({x}) = 0 THEN NULL \
                     WHEN sum(abs(CAST({x} AS HUGEINT))) >= {DOUBLE_EXACT_INTEGERS} \
                     THEN CAST(error('{INEXACT_SUM_ERROR}') AS DOUBLE) \
                     ELSE CAST(sum(CAST({x} AS BIGINT)) AS DOUBLE) / CAST(count({x}) AS DOUBLE) END"
                ),
                T::Double,
            ))
        }
        ("MEDIAN", T::TinyInt | T::SmallInt | T::Int) if !distinct => Some(typed(
            format!(
                "CAST(quantile_cont(CAST({} AS DOUBLE), 0.5) AS DOUBLE)",
                value.sql
            ),
            T::Double,
        )),
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
        outside_function(
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

/// `REGR_COUNT(y, x)` of numbers: the rows where neither is NULL, as BIGINT.
fn pair_count(args: &[Value], site: Site<'_>) -> Result<Emitted, Refusal> {
    let refusal = |reason: &str| refused_function("REGR_COUNT", reason.to_owned());
    if !site.aggregates {
        return Err(refusal(
            "no rule converts REGR_COUNT where an aggregate cannot appear",
        ));
    }
    let values = arguments(args, site.without_aggregates())?;
    let _rule = site.log.rule(AGGREGATE_RULE);
    match values.as_slice() {
        [y, x] if y.data_type.is_numeric() && x.data_type.is_numeric() => Ok(typed(
            format!(
                "count(CASE WHEN {} IS NOT NULL AND {} IS NOT NULL THEN 1 END)",
                y.sql, x.sql
            ),
            DatabricksType::BigInt,
        )),
        _ => Err(refusal("only REGR_COUNT of two numbers converts")),
    }
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
        (Signature::SameType, [data_type])
            if name == COALESCE_NAME && *data_type != DatabricksType::Null =>
        {
            (vec![values[0].sql.clone()], *data_type)
        }
        (Signature::SameType, types) if types.len() >= MIN_LIST_ARGUMENTS => {
            match common_type(&values) {
                Some(DatabricksType::Array(_)) if ORDERING_FUNCTIONS.contains(&name) => {
                    return Err(refused_function(
                        name,
                        format!(
                            "no rule converts {name} of arrays: DuckDB orders a NULL element above a value, Databricks below"
                        ),
                    ));
                }
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

/// A refusal of a same-named function whose arguments fall outside its signature.
fn mismatched(name: &str, types: &[DatabricksType]) -> Refusal {
    let names: Vec<String> = types.iter().map(|data_type| data_type.name()).collect();
    outside_function(
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

/// `PERCENTILE_CONT(p)` or `PERCENTILE_DISC(p) WITHIN GROUP (ORDER BY x [DESC])` of an integer up to INT as DOUBLE, for a literal fraction both engines hold exactly.
pub(crate) fn within_group(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let within = understood(payload, WITHIN_GROUP_KIND, &["this", "order_by"])?;
    let refusal = |reason: &str| refused(WITHIN_GROUP_KIND, reason.to_owned());
    let Some((kind, call)) = node_kind(field(within, "this")) else {
        return Err(refusal("no rule converts this WITHIN GROUP"));
    };
    let call = understood(
        call,
        kind,
        &["name", "args", "set_quantifier", "from_separator"],
    )?;
    let name = field(call, "name")
        .as_str()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let function = match name.as_str() {
        "PERCENTILE_CONT" => "percentile_cont",
        "PERCENTILE_DISC" => "percentile_disc",
        _ => return Err(refusal(&format!("no rule converts {name} WITHIN GROUP"))),
    };
    if !site.aggregates {
        return Err(refusal(&format!(
            "no rule converts {name} where an aggregate cannot appear"
        )));
    }
    if !matches!(field(call, "set_quantifier").as_str(), None | Some("none")) {
        return Err(refusal(&format!(
            "no rule converts {name} with a set quantifier"
        )));
    }
    let ([fraction], [ordered]) = (list(call, "args"), list(within, "order_by")) else {
        return Err(refusal(&format!(
            "only {name} of one fraction and one order key converts"
        )));
    };
    if matches!(node_kind(fraction), Some(("column", _))) {
        return Err(outside(
            WITHIN_GROUP_KIND,
            format!("{name} of a column fraction, which Databricks rejects as no constant"),
        ));
    }
    let inner = site.without_aggregates();
    let fraction = expression(fraction, inner)?;
    let _rule = site.log.rule(AGGREGATE_RULE);
    let exact = match &fraction.literal {
        Some(Literal::Number(text)) => match text.parse::<f64>() {
            Ok(value) => {
                let steps = value * f64::from(EXACT_FRACTION_STEPS);
                let whole = steps.trunc() == steps
                    && (0..=i64::from(EXACT_FRACTION_STEPS)).contains(&(steps as i64));
                whole.then_some(value)
            }
            Err(_) => None,
        },
        _ => None,
    };
    let Some(exact) = exact else {
        return Err(refusal(&format!(
            "only {name} of a literal fraction in steps of 1/{EXACT_FRACTION_STEPS} converts, which both engines interpolate exactly"
        )));
    };
    let ordered = match node_kind(ordered) {
        Some(("ordered", payload)) => payload,
        _ => ordered,
    };
    let Some(ordered) = ordered.as_object() else {
        return Err(refusal(&format!("no rule reads the order key of {name}")));
    };
    let value = expression(field(ordered, "this"), inner)?;
    if !small_integer(value.data_type) {
        return Err(refusal(&format!(
            "no rule converts {name} of {}, whose interpolation may round differently",
            value.data_type.name()
        )));
    }
    let direction = match flag(ordered, "desc") {
        true => " DESC",
        false => "",
    };
    Ok(typed(
        format!(
            "CAST({function}({exact}) WITHIN GROUP (ORDER BY CAST({} AS DOUBLE){direction}) AS DOUBLE)",
            value.sql
        ),
        DatabricksType::Double,
    ))
}
