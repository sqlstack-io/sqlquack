//! Window functions and their OVER clauses.

use serde_json::{Value, json};
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::arrays::constants::ARRAY_SORT_ORDER;
use sqlquack_duckdb::arrays::main::json_order_key::json_order_key;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::explicit_null_order::explicit_null_order;
use sqlquack_duckdb::blocks::main::window_frame::window_frame;
use sqlquack_duckdb::semi_structured::constants::JSON_FIDELITY;
use sqlquack_duckdb::semi_structured::main::json_output::json_output;
use sqlquack_semantics::types::main::utc_kept::utc_kept;
use sqlquack_snowflake::typing::main::ranking::ranking;
use sqlquack_snowflake::typing::main::unify::unify;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::calls::convert_aggregate;
use crate::conversion::_helpers::expressions::intervals::interval_literal;
use crate::conversion::_helpers::expressions::named_aggregates::named_aggregate;
use crate::conversion::_helpers::expressions::ordered_aggregates::within_group;
use crate::conversion::_helpers::expressions::scalar::{call_name, convert_expression};
use crate::conversion::_helpers::expressions::spellings::{as_typed, as_written};
use crate::conversion::_helpers::support::declared::merged;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Context, Emitted};
use crate::conversion::_helpers::support::shapes::type_family;
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, flag, function_construct, integer_literal, list, node_construct, node_kind,
    payload_object, require_understood,
};
use crate::conversion::_helpers::support::typed_leaves::compared_by_text;
use crate::conversion::constants::{
    CALL_KIND, DISTINCT_WINDOW_AGGREGATE, FRAME_FACET, FUNCTION_FACET, NO_PARTITION, NTILE_NAME,
    PARTITION_FACET, RATIO_TO_REPORT_NAME, SEVERAL_PARTITION_KEYS, UNORDERED_WINDOW_AGGREGATES,
    VARIANT_EQUALITY_BY_TEXT, VARIANT_ORDER_KEY, VARIANT_PARTITION, WINDOW_AGGREGATE,
    WINDOW_FIRST_LAST, WINDOW_KEYS_CONSTRUCT, WINDOW_OFFSET, WINDOW_RANKING, WINDOW_VALUE_FRAME,
    WINDOWED_NAMED_AGGREGATES,
};

/// Snowflake's frame for FIRST_VALUE and LAST_VALUE when none is written.
const WHOLE_PARTITION: &str = "ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING";
/// The frame for an ordered window aggregate when none is written.
const RUNNING_RANGE: &str = "RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW";
/// Frame kinds and bounds as the parser names them.
const ROWS: &str = "Rows";
const RANGE: &str = "Range";
/// The interval unit a RANGE frame offset over a DATE key is converted in.
const DAY_UNIT: &str = "DAY";
const UNBOUNDED_PRECEDING: &str = "UnboundedPreceding";
const UNBOUNDED_FOLLOWING: &str = "UnboundedFollowing";
const CURRENT_ROW: &str = "CurrentRow";
/// The one named aggregate whose window frame is recorded: COUNT_IF over a running ROWS frame.
const COUNT_IF_NAME: &str = "COUNT_IF";
/// LAG and LEAD as the parser names generic calls of them.
const LAG_NAME: &str = "LAG";
const LEAD_NAME: &str = "LEAD";
/// LAG and LEAD take a value, an offset and a default.
const OFFSET_ARGUMENTS: usize = 3;

/// A window's partition keys, order items and partition keys' families.
struct WindowKeys {
    partition: Vec<String>,
    order: Vec<String>,
    families: Vec<&'static str>,
    ascending_date: bool,
}

/// The converted OVER clause.
struct Over<'a> {
    partition: Vec<String>,
    order: Vec<String>,
    frame: Option<&'a Value>,
    /// Whether the window is ordered by one DATE key, ascending.
    ascending_date: bool,
}

impl Over<'_> {
    fn clause(&self, frame: Option<&str>, site: Site<'_>) -> String {
        let mut parts: Vec<String> = Vec::new();
        if !self.partition.is_empty() {
            parts.push(format!("PARTITION BY {}", self.partition.join(", ")));
        }
        if !self.order.is_empty() {
            parts.push(format!("ORDER BY {}", self.order.join(", ")));
        }
        if let Some(frame) = frame {
            parts.push(window_frame(frame, site.context.log));
        }
        parts.join(" ")
    }
}

/// A window function call with its OVER clause.
pub(crate) fn convert_window(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("window_function");
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "over"], &construct)?;
    let written = field(payload, "this", &construct)?;
    let typed = as_typed(written);
    let Some((function, function_payload)) =
        node_kind(typed.as_ref().map_or(written, |(typed, _)| typed))
    else {
        return Err(missing(
            construct,
            "a window function has an unexpected shape".to_owned(),
        ));
    };
    if matches!(function, "function" | "aggregate_function")
        && call_name(function_payload) == RATIO_TO_REPORT_NAME
    {
        return ratio_to_report(function_payload, payload, site);
    }
    let rule = match function {
        "row_number" | "rank" | "dense_rank" => WINDOW_RANKING,
        "function" if call_name(function_payload) == NTILE_NAME => WINDOW_RANKING,
        "lag" | "lead" => WINDOW_OFFSET,
        "function" | "aggregate_function" if offset_name(function_payload).is_some() => {
            WINDOW_OFFSET
        }
        "first_value" | "last_value" => WINDOW_FIRST_LAST,
        "count" | "sum" | "min" | "max" | "within_group" => WINDOW_AGGREGATE,
        "function" | "aggregate_function"
            if WINDOWED_NAMED_AGGREGATES.contains(&call_name(function_payload).as_str()) =>
        {
            WINDOW_AGGREGATE
        }
        "function" | "aggregate_function" => {
            let name = call_name(function_payload);
            return Err(missing(
                function_construct(&name),
                format!("no rule converts window function fn:{name}"),
            ));
        }
        other => {
            return Err(missing(
                node_construct(other),
                format!("no rule converts window function node:{other}"),
            ));
        }
    };
    let _rule = site.context.log.rule(rule);
    let over_payload = payload_object(field(payload, "over", &construct)?, &construct)?;
    let over = convert_over(over_payload, site.placed(Placement::GROUPED))?;
    let log = site.context.log;
    log.facet(
        FUNCTION_FACET,
        &match function {
            "function" | "aggregate_function" => call_name(function_payload),
            other => other.to_ascii_uppercase(),
        },
    );
    log.facet(
        FRAME_FACET,
        match (over.frame, over.order.is_empty()) {
            (None, true) => "unordered",
            (None, false) => "ordered",
            (Some(frame), _) if is_whole_partition(frame) => "whole_partition",
            (Some(frame), _) => match frame.get("kind").and_then(Value::as_str) {
                Some(ROWS) => "rows",
                _ => "range",
            },
        },
    );
    let site = site.placed(Placement::GROUPED);
    let converted = match function {
        "row_number" | "rank" | "dense_rank" => ranking_window(function, &over, site),
        "function" if call_name(function_payload) == NTILE_NAME => {
            ntile_window(function_payload, &over, site)
        }
        "lag" | "lead" => offset_window((function, false), function_payload, &over, site),
        "function" | "aggregate_function" if offset_name(function_payload).is_some() => {
            ignoring_nulls_offset(function_payload, &over, site)
        }
        "first_value" | "last_value" => first_last_window(function, function_payload, &over, site),
        "count" | "sum" | "min" | "max" => {
            aggregate_window(function, function_payload, &over, site)
        }
        "within_group" => ordered_window(function_payload, &over, site),
        "function" | "aggregate_function" => {
            let name = call_name(function_payload);
            if over.frame.is_some_and(|frame| {
                name != COUNT_IF_NAME || !is_running(frame) || over.order.is_empty()
            }) {
                return Err(outside_conditions(
                    function_construct(&name),
                    format!("{name} over an explicit frame, which no recording shows"),
                ));
            }
            if UNORDERED_WINDOW_AGGREGATES.contains(&name.as_str()) && !over.order.is_empty() {
                return Err(outside_conditions(
                    function_construct(&name),
                    format!("{name} over an ordered window, which Snowflake does not support"),
                ));
            }
            let clause = aggregate_clause(&over, site, &node_construct(function))?;
            named_aggregate(
                &call_name(function_payload),
                function_payload,
                site,
                Some(&clause),
            )
        }
        other => Err(missing(
            node_construct(other),
            format!("no rule converts window function node:{other}"),
        )),
    };
    match typed {
        Some((_, spelling)) => converted.map_err(|refusal| as_written(refusal, spelling)),
        None => converted,
    }
}

fn convert_over<'a>(over: &'a Payload, site: Site<'_>) -> Result<Over<'a>, Refusal> {
    let construct = node_construct("window_function");
    require_understood(over, &["partition_by", "order_by", "frame"], &construct)?;
    let log = site.context.log;
    log.enter(WINDOW_KEYS_CONSTRUCT, false);
    let keys = window_keys(over, site, &construct);
    log.leave(None, (0, None));
    let WindowKeys {
        partition,
        order,
        families,
        ascending_date,
    } = keys?;
    log.facet(
        PARTITION_FACET,
        match families.as_slice() {
            [] => NO_PARTITION,
            [family] => family,
            _ => SEVERAL_PARTITION_KEYS,
        },
    );
    Ok(Over {
        partition,
        order,
        frame: over.get("frame").filter(|frame| !frame.is_null()),
        ascending_date,
    })
}

/// The partition keys, the order items and the partition keys' families, outside the window's shape.
fn window_keys(
    over: &Payload,
    site: Site<'_>,
    construct: &Construct,
) -> Result<WindowKeys, Refusal> {
    let mut partition: Vec<String> = Vec::new();
    let mut families: Vec<&'static str> = Vec::new();
    for key in list(over, "partition_by") {
        let key = convert_expression(key, site)?;
        families.push(type_family(key.data_type));
        let key = partition_key(key, construct, site.context)?;
        partition.push(key.sql);
    }
    let mut order: Vec<String> = Vec::new();
    let mut types: Vec<SnowflakeType> = Vec::new();
    for item in list(over, "order_by") {
        let (sql, data_type) = typed_order_item(item, site)?;
        order.push(sql);
        types.push(data_type);
    }
    let ascending = list(over, "order_by").iter().all(|item| {
        item.get("desc")
            .is_none_or(|desc| desc != &Value::Bool(true))
    });
    Ok(WindowKeys {
        partition,
        order,
        families,
        ascending_date: ascending && types == [SnowflakeType::Date],
    })
}

fn ranking_window(function: &str, over: &Over<'_>, site: Site<'_>) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let refuse = |reason: String| outside_conditions(node_construct(function), reason);
    if over.order.is_empty() || over.frame.is_some() {
        return Err(refuse("ranking needs an ORDER BY and no frame".to_owned()));
    }
    let data_type = ranking(log);
    let sql = cast(
        &format!("{function}() OVER ({})", over.clause(None, site)),
        data_type,
        log,
    )
    .map_err(refuse)?;
    Ok(Emitted::new(sql, data_type))
}

/// NTILE(n) of a positive integer literal: the ORDER BY rows split into n buckets, as ranking is.
fn ntile_window(payload: &Value, over: &Over<'_>, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = function_construct(NTILE_NAME);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let call = payload_object(payload, &construct)?;
    require_understood(call, &["name", "args"], &construct)?;
    let buckets = match list(call, "args") {
        [count] => integer_literal(count).filter(|count| *count >= 1),
        _ => None,
    }
    .ok_or_else(|| refuse("NTILE of other than one positive integer literal".to_owned()))?;
    if over.order.is_empty() || over.frame.is_some() {
        return Err(refuse("ranking needs an ORDER BY and no frame".to_owned()));
    }
    let log = site.context.log;
    let data_type = ranking(log);
    let sql = cast(
        &format!("ntile({buckets}) OVER ({})", over.clause(None, site)),
        data_type,
        log,
    )
    .map_err(refuse)?;
    Ok(Emitted::new(sql, data_type))
}

/// RATIO_TO_REPORT(x) as x / NULLIFZERO(SUM(x)) over the same partitions, typed and rounded as that
/// division and NULL for a zero sum, as recorded.
fn ratio_to_report(call: &Value, window: &Payload, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = function_construct(RATIO_TO_REPORT_NAME);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let call = payload_object(call, &construct)?;
    require_understood(call, &["name", "args"], &construct)?;
    let [value] = list(call, "args") else {
        return Err(refuse("RATIO_TO_REPORT of other than one argument"));
    };
    let over = field(window, "over", &construct)?;
    if !list(payload_object(over, &construct)?, "order_by").is_empty()
        || over.get("frame").is_some_and(|frame| !frame.is_null())
    {
        return Err(refuse(
            "RATIO_TO_REPORT over an ordered or framed window, which Snowflake rejects",
        ));
    }
    let _rule = site.context.log.rule(WINDOW_AGGREGATE);
    let total =
        json!({ "window_function": { "this": { "sum": { "this": value } }, "over": over } });
    let divisor =
        json!({ CALL_KIND: { "name": "NULLIFZERO", "args": [total], "distinct": false } });
    convert_expression(&json!({ "div": { "left": value, "right": divisor } }), site)
}

/// `lag` or `lead` for a LAG or LEAD call the parser keeps generic, as it does with IGNORE NULLS.
fn offset_name(payload: &Value) -> Option<&'static str> {
    match call_name(payload).as_str() {
        LAG_NAME => Some("lag"),
        LEAD_NAME => Some("lead"),
        _ => None,
    }
}

/// LAG or LEAD with IGNORE NULLS: the offset counts only rows whose value is not NULL.
fn ignoring_nulls_offset(
    payload: &Value,
    over: &Over<'_>,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let function = offset_name(payload).unwrap_or("lag");
    let construct = function_construct(&call_name(payload));
    let call = payload_object(payload, &construct)?;
    require_understood(call, &["name", "args", "ignore_nulls"], &construct)?;
    if !flag(call, "ignore_nulls") {
        return Err(outside_conditions(
            construct,
            format!(
                "{} as a generic call without IGNORE NULLS",
                call_name(payload)
            ),
        ));
    }
    let arguments = list(call, "args");
    if arguments.is_empty() || arguments.len() > OFFSET_ARGUMENTS {
        return Err(outside_conditions(
            construct,
            format!("{} with these arguments", call_name(payload)),
        ));
    }
    let inner = json!({
        "this": arguments[0],
        "offset": arguments.get(1).cloned().unwrap_or(Value::Null),
        "default": arguments.get(2).cloned().unwrap_or(Value::Null),
    });
    offset_window((function, true), &inner, over, site)
}

fn offset_window(
    (function, ignore_nulls): (&str, bool),
    payload: &Value,
    over: &Over<'_>,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(function);
    let log = site.context.log;
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let inner = payload_object(payload, &construct)?;
    require_understood(inner, &["this", "offset", "default"], &construct)?;
    if over.order.is_empty() || over.frame.is_some() {
        return Err(refuse(
            "LAG and LEAD need an ORDER BY and no frame".to_owned(),
        ));
    }
    let value = convert_expression(field(inner, "this", &construct)?, site)?;
    let offset = match inner.get("offset").filter(|offset| !offset.is_null()) {
        Some(offset) => integer_literal(offset)
            .ok_or_else(|| refuse("the offset must be an integer literal".to_owned()))?,
        None => 1,
    };
    if ignore_nulls && offset < 1 {
        return Err(refuse(
            "IGNORE NULLS is recorded only with an offset of 1 or more".to_owned(),
        ));
    }
    let mut arguments = vec![value.sql.clone(), offset.to_string()];
    let mut values = vec![(value.data_type, value.utc_offset)];
    let mut declared = value.declared.clone();
    if let Some(default) = inner.get("default").filter(|default| !default.is_null()) {
        let default = convert_expression(default, site)?;
        values.push((default.data_type, default.utc_offset));
        declared.extend(default.declared.iter().cloned());
        let common = unify(&[value.data_type, default.data_type], log).map_err(refuse)?;
        if common != value.data_type {
            return Err(refuse(format!(
                "a default of type {} changes the result type from {}",
                default.data_type, value.data_type
            )));
        }
        arguments.push(cast(&default.sql, value.data_type, log).map_err(refuse)?);
    }
    Ok(Emitted {
        utc_offset: utc_kept(&values, value.data_type),
        declared: merged(declared),
        ..Emitted::new(
            format!(
                "{function}({}{}) OVER ({})",
                arguments.join(", "),
                match ignore_nulls {
                    true => " IGNORE NULLS",
                    false => "",
                },
                over.clause(None, site)
            ),
            value.data_type,
        )
    })
}

fn first_last_window(
    function: &str,
    payload: &Value,
    over: &Over<'_>,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(function);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let inner = payload_object(payload, &construct)?;
    require_understood(inner, &["this", "ignore_nulls"], &construct)?;
    if over.order.is_empty() {
        return Err(refuse(format!(
            "{} needs an ORDER BY",
            function.to_ascii_uppercase()
        )));
    }
    let frame = match over.frame {
        Some(frame) if !is_whole_partition(frame) => {
            let _rule = site.context.log.rule(WINDOW_VALUE_FRAME);
            explicit_frame(frame).ok_or_else(|| {
                refuse("only ROWS frames with literal bounds are converted".to_owned())
            })?
        }
        _ => WHOLE_PARTITION.to_owned(),
    };
    let value = convert_expression(field(inner, "this", &construct)?, site)?;
    let nulls = match flag(inner, "ignore_nulls") {
        true => " IGNORE NULLS",
        false => "",
    };
    Ok(Emitted {
        utc_offset: value.utc_offset,
        declared: value.declared.clone(),
        ..Emitted::new(
            format!(
                "{function}({}{nulls}) OVER ({})",
                value.sql,
                over.clause(Some(&frame), site)
            ),
            value.data_type,
        )
    })
}

fn aggregate_window(
    function: &str,
    payload: &Value,
    over: &Over<'_>,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(function);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    if flag(payload_object(payload, &construct)?, "distinct")
        && (function != DISTINCT_WINDOW_AGGREGATE || !over.order.is_empty() || over.frame.is_some())
    {
        return Err(refuse(
            "DISTINCT in a window aggregate other than COUNT over whole partitions".to_owned(),
        ));
    }
    let clause = aggregate_clause(over, site, &construct)?;
    convert_aggregate(function, payload, site, Some(&clause))
}

/// An aggregate's OVER clause: an explicit literal ROWS frame, or a running one when ordered.
fn aggregate_clause(
    over: &Over<'_>,
    site: Site<'_>,
    construct: &Construct,
) -> Result<String, Refusal> {
    let frame = match over.frame {
        Some(frame) => Some(
            match explicit_frame(frame) {
                Some(rows) => Some(rows),
                None => day_range_frame(frame, over, site)?,
            }
            .ok_or_else(|| {
                outside_conditions(
                    construct.clone(),
                    "only ROWS frames with literal bounds, and RANGE frames of whole days over \
                     one ascending DATE key, are converted"
                        .to_owned(),
                )
            })?,
        ),
        None if !over.order.is_empty() => Some(RUNNING_RANGE.to_owned()),
        None => None,
    };
    Ok(over.clause(frame.as_deref(), site))
}

/// A WITHIN GROUP aggregate over whole partitions: OVER without ORDER BY or a frame.
fn ordered_window(payload: &Value, over: &Over<'_>, site: Site<'_>) -> Result<Emitted, Refusal> {
    if !over.order.is_empty() || over.frame.is_some() {
        return Err(outside_conditions(
            node_construct("within_group"),
            "a WITHIN GROUP aggregate over an ordered or framed window".to_owned(),
        ));
    }
    within_group(payload, site, Some(&over.clause(None, site)))
}

/// Whether a frame is the rows from the partition's start to the current row.
fn is_running(frame: &Value) -> bool {
    frame.get("kind").and_then(Value::as_str) == Some(ROWS)
        && frame.get("start").and_then(Value::as_str) == Some(UNBOUNDED_PRECEDING)
        && frame
            .get("end")
            .is_none_or(|end| end.is_null() || end.as_str() == Some(CURRENT_ROW))
        && frame.get("exclude").is_none_or(Value::is_null)
}

fn is_whole_partition(frame: &Value) -> bool {
    frame.get("kind").and_then(Value::as_str) == Some(ROWS)
        && frame.get("start").and_then(Value::as_str) == Some(UNBOUNDED_PRECEDING)
        && frame.get("end").and_then(Value::as_str) == Some(UNBOUNDED_FOLLOWING)
        && frame.get("exclude").is_none_or(Value::is_null)
}

/// A frame bound's SQL and its place relative to the current row, unbounded ends outermost.
fn frame_bound(bound: &Value) -> Option<(String, i128)> {
    match bound {
        Value::String(name) => match name.as_str() {
            UNBOUNDED_PRECEDING => Some(("UNBOUNDED PRECEDING".to_owned(), i128::MIN)),
            UNBOUNDED_FOLLOWING => Some(("UNBOUNDED FOLLOWING".to_owned(), i128::MAX)),
            CURRENT_ROW => Some(("CURRENT ROW".to_owned(), 0)),
            _ => None,
        },
        Value::Object(_) => {
            let (side, offset) = node_kind(bound)?;
            let offset = integer_literal(offset)?;
            match side {
                "Preceding" => Some((format!("{offset} PRECEDING"), -i128::from(offset))),
                "Following" => Some((format!("{offset} FOLLOWING"), i128::from(offset))),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A RANGE frame whose offsets are literal intervals of whole days, over one ascending DATE key.
fn day_range_frame(
    frame: &Value,
    over: &Over<'_>,
    site: Site<'_>,
) -> Result<Option<String>, Refusal> {
    let range = frame.get("kind").and_then(Value::as_str) == Some(RANGE);
    if !range || !over.ascending_date || !frame.get("exclude").is_none_or(Value::is_null) {
        return Ok(None);
    }
    let bound = |bound: &Value| -> Result<Option<(String, i128)>, Refusal> {
        let Some((side, offset)) = node_kind(bound).filter(|_| bound.is_object()) else {
            return Ok(frame_bound(bound));
        };
        let days = match interval_literal(offset, site)? {
            Some((days, DAY_UNIT)) => i128::from(days),
            _ => return Ok(None),
        };
        Ok(match side {
            "Preceding" => Some((format!("INTERVAL {days} DAY PRECEDING"), -days)),
            "Following" => Some((format!("INTERVAL {days} DAY FOLLOWING"), days)),
            _ => None,
        })
    };
    let Some((start, from)) = frame.get("start").map(bound).transpose()?.flatten() else {
        return Ok(None);
    };
    let (end, to) = match frame.get("end") {
        Some(Value::Null) | None => ("CURRENT ROW".to_owned(), 0),
        Some(end) => match bound(end)? {
            Some(end) => end,
            None => return Ok(None),
        },
    };
    let inverted = from > to || from == i128::MAX || to == i128::MIN;
    Ok((!inverted).then(|| format!("RANGE BETWEEN {start} AND {end}")))
}

fn explicit_frame(frame: &Value) -> Option<String> {
    let rows = frame.get("kind").and_then(Value::as_str) == Some(ROWS);
    if !rows || !frame.get("exclude").is_none_or(Value::is_null) {
        return None;
    }
    let (start, from) = frame_bound(frame.get("start")?)?;
    let (end, to) = match frame.get("end") {
        Some(Value::Null) | None => ("CURRENT ROW".to_owned(), 0),
        Some(end) => frame_bound(end)?,
    };
    let inverted = from > to || from == i128::MAX || to == i128::MIN;
    (!inverted).then(|| format!("ROWS BETWEEN {start} AND {end}"))
}

/// A sort, grouping or partition key; DuckDB orders and groups VARIANT keys differently.
pub(crate) fn grouping_key(key: Emitted, construct: &Construct) -> Result<Emitted, Refusal> {
    match key.data_type {
        SnowflakeType::SemiStructured => Err(outside_conditions(
            construct.clone(),
            "a VARIANT key, which DuckDB orders and groups differently".to_owned(),
        )),
        _ => Ok(key),
    }
}

/// A VARIANT as Snowflake writes it, re-held as JSON, so DuckDB's equality of its text is VARIANT equality.
pub(crate) fn canonical_variant(
    value: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = context.log.rule(VARIANT_EQUALITY_BY_TEXT);
    diverge(context, JSON_FIDELITY, construct)?;
    compared_by_text((value.untainted, value.sourced), construct, context)?;
    Ok(Emitted {
        sql: json_output(&value.sql, context.log),
        ..value
    })
}

/// A partition key; a VARIANT partitions by Snowflake's text of it, keys in byte order.
fn partition_key(
    key: Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    match key.data_type {
        SnowflakeType::SemiStructured => {
            let _rule = context.log.rule(VARIANT_PARTITION);
            diverge(context, JSON_FIDELITY, construct)?;
            compared_by_text((key.untainted, key.sourced), construct, context)?;
            Ok(Emitted::new(
                format!("CAST({} AS VARCHAR)", json_output(&key.sql, context.log)),
                SnowflakeType::Varchar,
            ))
        }
        _ => Ok(key),
    }
}

/// One ORDER BY item's key, as it is compared when sorting.
pub(crate) fn order_key(item: &Value, site: Site<'_>) -> Result<String, Refusal> {
    Ok(typed_order_key(item, site)?.0)
}

/// An ORDER BY item's sort key and the type of the key as written.
fn typed_order_key(item: &Value, site: Site<'_>) -> Result<(String, SnowflakeType), Refusal> {
    let construct = node_construct("ordered");
    let object = payload_object(item, &construct)?;
    let key = convert_expression(field(object, "this", &construct)?, site)?;
    let data_type = key.data_type;
    if data_type == SnowflakeType::SemiStructured && !key.is_variant_held() {
        let sql = variant_order_key(&key, &construct, site.context)?.sql;
        return Ok((sql, data_type));
    }
    Ok((grouping_key(key, &construct)?.sql, data_type))
}

/// A JSON-held VARIANT's sort key in ARRAY_SORT's recorded order of kinds.
pub(crate) fn variant_order_key(
    key: &Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = context.log.rule(VARIANT_ORDER_KEY);
    diverge(context, ARRAY_SORT_ORDER, construct)?;
    compared_by_text((key.untainted, key.sourced), construct, context)?;
    Ok(Emitted::new(
        json_order_key(&key.sql, false, context.log),
        SnowflakeType::SemiStructured,
    ))
}

/// One ORDER BY item with Snowflake's NULL placement spelled out.
pub(crate) fn order_item(item: &Value, site: Site<'_>) -> Result<String, Refusal> {
    Ok(typed_order_item(item, site)?.0)
}

/// One ORDER BY item, and the type of its key as written.
fn typed_order_item(item: &Value, site: Site<'_>) -> Result<(String, SnowflakeType), Refusal> {
    let construct = node_construct("ordered");
    let object = payload_object(item, &construct)?;
    require_understood(object, &["this", "desc", "nulls_first"], &construct)?;
    let (key, data_type) = typed_order_key(item, site)?;
    let sql = explicit_null_order(
        &key,
        flag(object, "desc"),
        object.get("nulls_first").and_then(Value::as_bool),
        site.context.log,
    );
    Ok((sql, data_type))
}
