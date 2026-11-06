//! Aggregates the parser keeps as named calls, such as ANY_VALUE, COUNT_IF and OBJECT_AGG.

use crate::conversion::_helpers::expressions::windows::variant_order_key;
use crate::conversion::_helpers::support::scope::Context;
use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::aggregates::constants::EXTREME_KEY_TIES;
use sqlquack_duckdb::aggregates::main::list_median::list_median;
use sqlquack_duckdb::aggregates::main::value_at_extreme::value_at_extreme;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::column_type::column_type;
use sqlquack_duckdb::checking::constants::SEMI_STRUCTURED_HOLDING_TYPE;
use sqlquack_duckdb::raises::main::raised::raised;
use sqlquack_duckdb::semi_structured::constants::JSON_FIDELITY;
use sqlquack_duckdb::variants::main::json_object_agg::json_object_agg;
use sqlquack_snowflake::typing::constants::COUNT_IF_TYPE;
use sqlquack_snowflake::typing::constants::MAX_PRECISION;
use sqlquack_snowflake::typing::main::count::count;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::variants::to_variant;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    flag, function_construct, list, payload_object, require_understood,
};
use crate::conversion::_helpers::support::typed_leaves::compared_value;
use crate::conversion::constants::{
    MAX_BY_FUNCTION, MEDIAN_EXACT_DIGITS, MEDIAN_EXTRA_DIGITS, MEDIAN_RULE, MODE_RULE, MULTI_COUNT,
    NAMED_AGGREGATE_NAMES, NAMED_AGGREGATES, OBJECT_AGG_VALUE_PAIR, TO_VARIANT_RULE,
    VALUE_AT_EXTREME_KEY, WINDOWED_NAMED_AGGREGATES,
};
use sqlquack_policy::policy::constants::{
    ANY_VALUE_CHOICE, MODE_OF_NO_ROWS, MODE_TIES, PERCENTILE_INTERPOLATION,
};

/// Whether `name` is an aggregate converted here.
pub(crate) fn is_named_aggregate(name: &str) -> bool {
    NAMED_AGGREGATE_NAMES.contains(&name)
}

/// One named aggregate; `window` is the OVER clause when used as a window function.
pub(crate) fn named_aggregate(
    name: &str,
    payload: &Value,
    site: Site<'_>,
    window: Option<&str>,
) -> Result<Emitted, Refusal> {
    let construct = function_construct(name);
    let _rule = site.context.log.rule(NAMED_AGGREGATES);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["name", "args", "distinct"], &construct)?;
    let distinct = flag(payload, "distinct");
    if distinct != (name == MULTI_COUNT) {
        return Err(refuse(format!(
            "{name} with or without DISTINCT as written"
        )));
    }
    let over = window.map_or_else(String::new, |window| format!(" OVER ({window})"));
    if window.is_some() && !WINDOWED_NAMED_AGGREGATES.contains(&name) {
        return Err(refuse(format!("{name} over a window")));
    }
    let site = site.placed(Placement::ROW);
    let arguments: Vec<Emitted> = list(payload, "args")
        .iter()
        .map(|argument| convert_expression(argument, site))
        .collect::<Result<_, _>>()?;
    let log = site.context.log;
    match (name, arguments.as_slice()) {
        ("ANY_VALUE", [value]) if value.data_type != SnowflakeType::Null => {
            diverge(site.context, ANY_VALUE_CHOICE, &construct)?;
            compared_value(value, &construct, site.context)?;
            Ok(Emitted::new(
                single_value(&value.sql, value.data_type, log).map_err(refuse)?,
                value.data_type,
            ))
        }
        ("COUNT_IF", [condition]) if boolean(condition) => {
            let data_type = COUNT_IF_TYPE;
            Ok(Emitted::new(
                cast(
                    &format!(
                        "(CASE WHEN count(*){over} = 0 THEN NULL \
                         ELSE count(CASE WHEN {} THEN 1 END){over} END)",
                        condition.sql
                    ),
                    data_type,
                    log,
                )
                .map_err(refuse)?,
                data_type,
            ))
        }
        ("BOOLOR_AGG" | "BOOLAND_AGG", [condition]) if boolean(condition) => {
            let function = match name {
                "BOOLOR_AGG" => "bool_or",
                _ => "bool_and",
            };
            Ok(Emitted::new(
                format!("{function}({}){over}", condition.sql),
                SnowflakeType::Boolean,
            ))
        }
        (MULTI_COUNT, [_, _, ..]) => {
            let present: Vec<String> = arguments
                .iter()
                .map(|argument| format!("{} IS NOT NULL", argument.sql))
                .collect();
            let values: Vec<&str> = arguments
                .iter()
                .map(|argument| argument.sql.as_str())
                .collect();
            let data_type = count(log);
            Ok(Emitted::new(
                cast(
                    &format!(
                        "count(DISTINCT CASE WHEN {} THEN ({}) END){over}",
                        present.join(" AND "),
                        values.join(", ")
                    ),
                    data_type,
                    log,
                )
                .map_err(refuse)?,
                data_type,
            ))
        }
        ("MAX_BY" | "MIN_BY", [value, key])
            if ordered_key(key.data_type)
                || key.data_type == SnowflakeType::SemiStructured && !key.is_variant_held() =>
        {
            let _rule = log.rule(VALUE_AT_EXTREME_KEY);
            diverge(site.context, EXTREME_KEY_TIES, &construct)?;
            let key = match key.data_type {
                SnowflakeType::SemiStructured => variant_order_key(key, &construct, site.context)?,
                _ => key.clone(),
            };
            compared_value(value, &construct, site.context)?;
            let value_type = match value.data_type {
                SnowflakeType::SemiStructured => SEMI_STRUCTURED_HOLDING_TYPE.to_owned(),
                other => column_type(other, log).map_err(refuse)?,
            };
            Ok(Emitted::new(
                value_at_extreme(
                    (&value.sql, &key.sql),
                    name == MAX_BY_FUNCTION,
                    (&value_type, value.data_type == SnowflakeType::Float),
                    log,
                ),
                value.data_type,
            ))
        }
        ("OBJECT_AGG", [key, value])
            if matches!(key.data_type, SnowflakeType::Varchar)
                && value.data_type == SnowflakeType::SemiStructured =>
        {
            diverge(site.context, JSON_FIDELITY, &construct)?;
            Ok(Emitted::new(
                json_object_agg((&key.sql, &value.sql), &over, log),
                SnowflakeType::SemiStructured,
            ))
        }
        ("OBJECT_AGG", [key, value])
            if matches!(key.data_type, SnowflakeType::Varchar)
                && matches!(value.data_type, SnowflakeType::Number { .. }) =>
        {
            log.equivalence(OBJECT_AGG_VALUE_PAIR);
            let _rule = log.rule(TO_VARIANT_RULE);
            let value = to_variant(value.clone(), &construct, site.context)?;
            diverge(site.context, JSON_FIDELITY, &construct)?;
            Ok(Emitted::new(
                json_object_agg((&key.sql, &value.sql), &over, log),
                SnowflakeType::SemiStructured,
            ))
        }
        ("MEDIAN", [value]) if window.is_none() => median(value, &construct, site.context),
        ("MODE", [value]) if window.is_none() && mode_value(value.data_type) => {
            let _rule = log.rule(MODE_RULE);
            diverge(site.context, MODE_TIES, &construct)?;
            if !site.scope.grouped {
                diverge(site.context, MODE_OF_NO_ROWS, &construct)?;
            }
            let mode = most_frequent(&value.sql, value.data_type, log).map_err(refuse)?;
            let sql = match site.scope.grouped {
                true => mode,
                false => {
                    let empty = mode_of_no_rows(value.data_type, log).map_err(refuse)?;
                    format!("(CASE WHEN count(*) = 0 THEN {empty} ELSE {mode} END)")
                }
            };
            Ok(Emitted::new(sql, value.data_type))
        }
        (_, arguments) => Err(refuse(format!(
            "{name} of {}",
            arguments
                .iter()
                .map(|argument| argument.data_type.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// MEDIAN of a NUMBER, exact where its middles are narrow enough, or of a FLOAT, as recorded.
fn median(
    value: &Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    let log = context.log;
    let _rule = log.rule(MEDIAN_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    match value.data_type {
        SnowflakeType::Number { precision, scale }
            if scale + MEDIAN_EXTRA_DIGITS < MEDIAN_EXACT_DIGITS =>
        {
            diverge(context, PERCENTILE_INTERPOLATION, construct)?;
            let data_type = SnowflakeType::Number {
                precision: (precision + MEDIAN_EXTRA_DIGITS).min(MAX_PRECISION),
                scale: scale + MEDIAN_EXTRA_DIGITS,
            };
            let held = column_type(data_type, log).map_err(refuse)?;
            let digits = MEDIAN_EXACT_DIGITS - scale - MEDIAN_EXTRA_DIGITS;
            let bound = format!("1e{digits}");
            Ok(Emitted::new(
                list_median(&value.sql, (&held, Some(&bound)), log),
                data_type,
            ))
        }
        SnowflakeType::Float => Ok(Emitted::new(
            list_median(&value.sql, ("DOUBLE", None), log),
            SnowflakeType::Float,
        )),
        other => Err(refuse(format!("MEDIAN of {other}"))),
    }
}

fn boolean(emitted: &Emitted) -> bool {
    matches!(
        emitted.data_type,
        SnowflakeType::Boolean | SnowflakeType::Null
    )
}

/// The group's one value; different values, -0 beside 0, or NULL beside a value, raise `any_value_choice`.
fn single_value(
    value: &str,
    data_type: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    let raise = raised(&format!(
        "{ANY_VALUE_CHOICE}: ANY_VALUE of a group holding different values"
    ));
    let raise = match data_type {
        SnowflakeType::SemiStructured => format!("CAST({raise} AS JSON)"),
        _ => cast(&raise, data_type, log)?,
    };
    let signs = match data_type {
        SnowflakeType::Float => format!(" OR count(DISTINCT signbit({value})) > 1"),
        _ => String::new(),
    };
    Ok(format!(
        "(CASE WHEN count(DISTINCT {value}) > 1{signs} \
         OR (count({value}) > 0 AND count({value}) < count(*)) \
         THEN {raise} ELSE any_value({value}) END)"
    ))
}

/// MODE of no rows raises: Snowflake gives a zero value in some plans and NULL in others.
fn mode_of_no_rows(data_type: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
    let empty = raised(&format!(
        "{MODE_OF_NO_ROWS}: MODE over no rows, which Snowflake answers differently by plan"
    ));
    cast(&empty, data_type, log)
}

/// Whether MODE's equality of `data_type` values is DuckDB's, so both engines count them alike.
fn mode_value(data_type: SnowflakeType) -> bool {
    matches!(
        data_type,
        SnowflakeType::Number { .. }
            | SnowflakeType::Float
            | SnowflakeType::Varchar
            | SnowflakeType::Boolean
            | SnowflakeType::Date
            | SnowflakeType::TimestampNtz { .. }
    )
}

/// The group's most frequent value; values tied for most frequent, or -0 beside 0, raise `mode_ties`.
fn most_frequent(
    value: &str,
    data_type: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    let raise = cast(
        &raised(&format!(
            "{MODE_TIES}: MODE of a group whose most frequent values tie"
        )),
        data_type,
        log,
    )?;
    let counts = format!("map_values(histogram({value}))");
    let signs = match data_type {
        SnowflakeType::Float => format!(
            " OR (count(CASE WHEN {value} = 0 AND signbit({value}) THEN 1 END) > 0 \
             AND count(CASE WHEN {value} = 0 AND NOT signbit({value}) THEN 1 END) > 0)"
        ),
        _ => String::new(),
    };
    Ok(format!(
        "(CASE WHEN length(list_filter({counts}, lambda n: n = list_max({counts}))) > 1{signs} \
         THEN {raise} ELSE mode({value}) END)"
    ))
}

/// Whether MIN and MAX order `key` as Snowflake does, so MAX_BY and MIN_BY can find its extreme.
fn ordered_key(key: SnowflakeType) -> bool {
    matches!(
        key,
        SnowflakeType::Number { .. }
            | SnowflakeType::Float
            | SnowflakeType::Varchar
            | SnowflakeType::Boolean
            | SnowflakeType::Date
            | SnowflakeType::Time { .. }
            | SnowflakeType::TimestampNtz { .. }
            | SnowflakeType::TimestampTz { .. }
            | SnowflakeType::TimestampLtz { .. }
    )
}
