//! Window functions over an ordered window, with Databricks's null ordering and frames written out.

use serde_json::Value;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::databricks::conversion::_helpers::expressions::dispatch::expression;
use crate::databricks::conversion::_helpers::expressions::literals::typed;
use crate::databricks::conversion::_helpers::functions::calls::{Aggregate, aggregate};
use crate::databricks::conversion::_helpers::queries::query::ordering;
use crate::databricks::conversion::_helpers::support::scope::{Scope, Site};
use crate::databricks::conversion::_helpers::support::tree::{
    digits_value, field, flag, identifier, list, node_kind, refused, refused_function, understood,
};
use crate::databricks::conversion::constants::{
    AGGREGATES, CUME_DIST_NAME, CURRENT_ROW, FRACTION_WINDOWS, INTEGER_WINDOWS, MAX_WINDOW_COUNT,
    NTH_VALUE_NAME, NTILE_NAME, OFFSET_WINDOWS, RANGE_FRAME, RANK_WINDOWS, ROWS_FRAME, WINDOW_RULE,
};
use crate::databricks::conversion::models::{DatabricksType, Emitted, Literal};

/// The parsed parts of a window call: the function, its arguments and the `OVER` clause.
struct Window<'a> {
    name: String,
    args: &'a [Value],
    ignore_nulls: bool,
    distinct: bool,
    over: &'a serde_json::Map<String, Value>,
}

/// A frame bound's DuckDB SQL and its place relative to the current row, unbounded ends outermost.
struct Bound {
    sql: String,
    place: i128,
}

/// `function(...) OVER (...)` as DuckDB SQL with the Databricks result type.
pub(crate) fn window(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let window = parts(payload)?;
    let name = window.name.as_str();
    if window.distinct && !AGGREGATES.iter().any(|(aggregate, _)| *aggregate == name) {
        return Err(refused_function(
            name,
            format!("no rule converts {name} with a set quantifier as a window function"),
        ));
    }
    if !site.windows {
        return Err(refused_function(
            name,
            format!(
                "no rule converts {name} OVER outside the projection of a block without GROUP BY"
            ),
        ));
    }
    let _rule = site.log.rule(WINDOW_RULE);
    let inner = Site {
        aggregates: false,
        windows: false,
        outputs: false,
        ..site
    };
    let frame = window.over.get("frame").filter(|frame| !frame.is_null());
    if AGGREGATES.iter().any(|(aggregate, _)| *aggregate == name) {
        return aggregate_window(&window, frame, inner);
    }
    let keys = keys(window.over, inner)?;
    if list(window.over, "order_by").is_empty() {
        return Err(refused_function(
            name,
            format!(
                "no rule converts {name} over a window without ORDER BY, whose row order neither engine fixes"
            ),
        ));
    }
    let fixed_frame = ranking(name) || OFFSET_WINDOWS.contains(&name);
    if fixed_frame && frame.is_some() {
        return Err(refused_function(
            name,
            format!("no rule converts {name} over a written frame, which Databricks rejects"),
        ));
    }
    if fixed_frame && window.ignore_nulls {
        return Err(refused_function(
            name,
            format!("no rule converts {name} IGNORE NULLS"),
        ));
    }
    let over = |frame: Option<String>| match frame {
        Some(frame) => format!("OVER ({keys} {frame})"),
        None => format!("OVER ({keys})"),
    };
    let value = |node: &Value| expression(node, inner);
    match (name, window.args) {
        (_, []) if INTEGER_WINDOWS.contains(&name) => Ok(typed(
            format!(
                "CAST({}() {} AS INTEGER)",
                name.to_ascii_lowercase(),
                over(None)
            ),
            DatabricksType::Int,
        )),
        (_, []) if FRACTION_WINDOWS.contains(&name) => Ok(typed(
            format!("{}() {}", name.to_ascii_lowercase(), over(None)),
            DatabricksType::Double,
        )),
        ("NTILE", [buckets]) => {
            let buckets = positive(&value(buckets)?, name)?;
            Ok(typed(
                format!("CAST(ntile({buckets}) {} AS INTEGER)", over(None)),
                DatabricksType::Int,
            ))
        }
        ("LAG" | "LEAD", [input, rest @ ..]) => {
            let input = valued(value(input)?, name)?;
            let mut args = vec![input.sql.clone()];
            let (offset, default) = match rest {
                [] => (None, None),
                [offset] => (Some(offset), None),
                [offset, default] => (Some(offset), Some(default)),
                _ => return Err(arity(name, window.args)),
            };
            if let Some(offset) = offset {
                args.push(integer(&value(offset)?, name)?.to_string());
            }
            if let Some(default) = default {
                let default = value(default)?;
                if default.data_type != input.data_type && default.data_type != DatabricksType::Null
                {
                    return Err(refused_function(
                        name,
                        format!(
                            "no rule converts {name} of a {} with a {} default",
                            input.data_type.name(),
                            default.data_type.name()
                        ),
                    ));
                }
                args.push(default.sql);
            }
            Ok(Emitted {
                sql: format!(
                    "{}({}) {}",
                    name.to_ascii_lowercase(),
                    args.join(", "),
                    over(None)
                ),
                literal: None,
                ..input
            })
        }
        ("FIRST_VALUE" | "LAST_VALUE", [input, rest @ ..]) => {
            let input = valued(value(input)?, name)?;
            let ignore_nulls = match rest {
                [] => window.ignore_nulls,
                [_] if window.ignore_nulls => {
                    return Err(refused_function(
                        name,
                        format!("no rule converts {name} with both IGNORE NULLS and a flag"),
                    ));
                }
                [flag] => match node_kind(flag) {
                    Some(("boolean", payload)) => {
                        payload.get("value").and_then(Value::as_bool) == Some(true)
                    }
                    _ => {
                        return Err(refused_function(
                            name,
                            format!(
                                "no rule converts {name} whose second argument is no TRUE or FALSE"
                            ),
                        ));
                    }
                },
                _ => return Err(arity(name, window.args)),
            };
            let frame = written_frame(frame, name)?;
            Ok(Emitted {
                sql: format!(
                    "{}({}{}) {}",
                    name.to_ascii_lowercase(),
                    input.sql,
                    ignoring(ignore_nulls),
                    over(Some(frame))
                ),
                literal: None,
                ..input
            })
        }
        ("NTH_VALUE", [input, position]) => {
            let input = valued(value(input)?, name)?;
            let position = positive(&value(position)?, name)?;
            let frame = written_frame(frame, name)?;
            Ok(Emitted {
                sql: format!(
                    "nth_value({}, {position}{}) {}",
                    input.sql,
                    ignoring(window.ignore_nulls),
                    over(Some(frame))
                ),
                literal: None,
                ..input
            })
        }
        _ => Err(arity(name, window.args)),
    }
}

/// An aggregate over a window: the whole partition, or with ORDER BY a frame written out.
fn aggregate_window(
    window: &Window<'_>,
    frame: Option<&Value>,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let name = window.name.as_str();
    if window.ignore_nulls || window.distinct {
        return Err(refused_function(
            name,
            format!("no rule converts {name} with DISTINCT or IGNORE NULLS as a window function"),
        ));
    }
    let keys = keys(window.over, site)?;
    let over = match (list(window.over, "order_by").is_empty(), frame) {
        (true, None) => format!(" OVER ({keys})"),
        (true, Some(_)) => {
            return Err(refused_function(
                name,
                format!(
                    "no rule converts {name} over a frame without ORDER BY, whose row order neither engine fixes"
                ),
            ));
        }
        (false, frame) => format!(" OVER ({keys} {})", written_frame(frame, name)?),
    };
    let aggregating = Site {
        aggregates: true,
        ..site
    };
    let call = Aggregate {
        name,
        args: window.args,
        distinct: false,
        over: Some(&over),
    };
    aggregate(&call, aggregating)
}

/// The refusal of a window call no rule converts with these arguments.
fn arity(name: &str, args: &[Value]) -> Refusal {
    refused_function(
        name,
        format!(
            "no rule converts {name} of {} arguments as a window function",
            args.len()
        ),
    )
}

/// The name Databricks gives an unaliased window column, when its parts are columns and integers.
pub(crate) fn spelled(node: &Value, scope: &Scope) -> Result<Option<String>, Refusal> {
    let Some((_, payload)) = node_kind(node) else {
        return Ok(None);
    };
    let window = parts(payload)?;
    let name = window.name.as_str();
    let frame = match window.over.get("frame").filter(|frame| !frame.is_null()) {
        Some(_) => return Ok(None),
        None if name == CUME_DIST_NAME || name == NTH_VALUE_NAME => RANGE_FRAME,
        None if ranking(name) => ROWS_FRAME,
        None => return Ok(None),
    };
    let mut args: Vec<String> = Vec::new();
    for arg in window.args {
        let Some(arg) = spelled_part(arg, scope)? else {
            return Ok(None);
        };
        args.push(arg);
    }
    let mut spec: Vec<String> = Vec::new();
    let mut partitions: Vec<String> = Vec::new();
    for key in list(window.over, "partition_by") {
        let Some(key) = spelled_part(key, scope)? else {
            return Ok(None);
        };
        partitions.push(key);
    }
    if !partitions.is_empty() {
        spec.push(format!("PARTITION BY {}", partitions.join(", ")));
    }
    let mut orders: Vec<String> = Vec::new();
    for ordered in list(window.over, "order_by") {
        let ordered = understood(ordered, "ordered", &["this", "desc", "nulls_first"])?;
        let Some(key) = spelled_part(field(ordered, "this"), scope)? else {
            return Ok(None);
        };
        let desc = flag(ordered, "desc");
        let nulls_first = field(ordered, "nulls_first").as_bool().unwrap_or(!desc);
        orders.push(format!(
            "{key} {} {}",
            if desc { "DESC" } else { "ASC" },
            if nulls_first {
                "NULLS FIRST"
            } else {
                "NULLS LAST"
            }
        ));
    }
    if !orders.is_empty() {
        spec.push(format!("ORDER BY {}", orders.join(", ")));
    }
    spec.push(format!(
        "{frame} BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW"
    ));
    let ignore = match window.ignore_nulls {
        true => " ignore nulls",
        false => "",
    };
    let spelling = match RANK_WINDOWS.contains(&name) {
        true => name.to_owned(),
        false => name.to_ascii_lowercase(),
    };
    Ok(Some(format!(
        "{spelling}({}){ignore} OVER ({})",
        args.join(", "),
        spec.join(" ")
    )))
}

/// Whether `name` is a ranking function, which takes no frame and no IGNORE NULLS.
fn ranking(name: &str) -> bool {
    INTEGER_WINDOWS.contains(&name) || FRACTION_WINDOWS.contains(&name) || name == NTILE_NAME
}

/// A column spelled as defined, or an integer literal, as Databricks spells them in a column name.
fn spelled_part(node: &Value, scope: &Scope) -> Result<Option<String>, Refusal> {
    match node_kind(node) {
        Some(("column", payload)) => {
            let column = understood(payload, "column", &["name", "table"])?;
            if !field(column, "table").is_null() {
                return Ok(None);
            }
            let written = identifier(field(column, "name"))?;
            let plain = scope
                .column(&written)
                .is_some_and(|(defined, _)| *defined == written)
                && written
                    .chars()
                    .next()
                    .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
                && written
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_');
            Ok(plain.then_some(written))
        }
        Some(("literal", payload)) => {
            let literal = understood(payload, "literal", &["literal_type", "value"])?;
            let digits = field(literal, "value").as_str().unwrap_or_default();
            let integer = field(literal, "literal_type").as_str() == Some("number")
                && digits_value(digits)
                    .is_some_and(|value| value <= MAX_WINDOW_COUNT && value.to_string() == digits);
            Ok(integer.then(|| digits.to_owned()))
        }
        _ => Ok(None),
    }
}

/// The function, its arguments, whether it ignores NULLs, and its `OVER` clause.
fn parts(payload: &Value) -> Result<Window<'_>, Refusal> {
    let window = understood(payload, "window_function", &["this", "over"])?;
    let over = understood(
        field(window, "over"),
        "over",
        &["partition_by", "order_by", "frame"],
    )?;
    let Some((kind @ ("function" | "aggregate_function"), call)) = node_kind(field(window, "this"))
    else {
        return Err(refused(
            "window_function",
            "no rule converts a window over anything but a function call".to_owned(),
        ));
    };
    let call = understood(
        call,
        kind,
        &["name", "args", "set_quantifier", "ignore_nulls"],
    )?;
    let name = field(call, "name")
        .as_str()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let distinct = !matches!(field(call, "set_quantifier").as_str(), None | Some("none"));
    Ok(Window {
        distinct,
        args: list(call, "args"),
        ignore_nulls: flag(call, "ignore_nulls"),
        over,
        name,
    })
}

/// `PARTITION BY ... ORDER BY ...`, the null ordering written out.
fn keys(over: &serde_json::Map<String, Value>, site: Site<'_>) -> Result<String, Refusal> {
    let mut parts: Vec<String> = Vec::new();
    let mut partitions: Vec<String> = Vec::new();
    for key in list(over, "partition_by") {
        let key = expression(key, site)?;
        if matches!(
            key.data_type,
            DatabricksType::Array(_) | DatabricksType::Float | DatabricksType::Double
        ) {
            return Err(refused(
                "window_function",
                format!(
                    "no rule partitions by a {} as Databricks does",
                    key.data_type.name()
                ),
            ));
        }
        partitions.push(key.sql);
    }
    if !partitions.is_empty() {
        parts.push(format!("PARTITION BY {}", partitions.join(", ")));
    }
    let orders = list(over, "order_by");
    if !orders.is_empty() {
        parts.push(format!("ORDER BY {}", ordering(orders, site)?));
    }
    Ok(parts.join(" "))
}

/// The value a value function returns, which must have a type.
fn valued(input: Emitted, name: &str) -> Result<Emitted, Refusal> {
    match input.data_type {
        DatabricksType::Null => Err(refused_function(
            name,
            format!("no rule converts {name} of an untyped NULL"),
        )),
        _ => Ok(input),
    }
}

/// An INT literal's value, as offsets and bucket counts must be.
fn integer(emitted: &Emitted, name: &str) -> Result<i32, Refusal> {
    let refusal = || {
        refused_function(
            name,
            format!("no rule converts {name} of an offset or count that is no INT literal"),
        )
    };
    let (Some(Literal::Number(digits)), DatabricksType::Int) =
        (&emitted.literal, emitted.data_type)
    else {
        return Err(refusal());
    };
    digits.parse::<i32>().map_err(|_| refusal())
}

/// A positive INT literal's value, as NTILE's buckets and NTH_VALUE's position must be.
fn positive(emitted: &Emitted, name: &str) -> Result<i32, Refusal> {
    match integer(emitted, name)? {
        value if value > 0 => Ok(value),
        _ => Err(refused_function(
            name,
            format!("no rule converts {name} of a count below 1, which Databricks rejects"),
        )),
    }
}

/// The SQL that ignores NULLs inside a value function's parentheses, as DuckDB writes it.
fn ignoring(ignore_nulls: bool) -> &'static str {
    match ignore_nulls {
        true => " IGNORE NULLS",
        false => "",
    }
}

/// The frame written out: Databricks's default over an ordered window, or a written ROWS or RANGE frame.
fn written_frame(frame: Option<&Value>, name: &str) -> Result<String, Refusal> {
    let Some(frame) = frame else {
        return Ok(format!(
            "{RANGE_FRAME} BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW"
        ));
    };
    let unsupported = || {
        refused_function(
            name,
            format!(
                "no rule converts {name} over a frame other than ROWS with literal bounds or RANGE \
                 without offsets, from no later than it ends"
            ),
        )
    };
    let frame = understood(frame, "frame", &["kind", "kind_text", "start", "end"])?;
    let kind = match field(frame, "kind").as_str() {
        Some("Rows") => ROWS_FRAME,
        Some("Range") => RANGE_FRAME,
        _ => return Err(unsupported()),
    };
    let start = bound(field(frame, "start")).ok_or_else(unsupported)?;
    let end = match field(frame, "end") {
        Value::Null => Bound {
            sql: CURRENT_ROW.to_owned(),
            place: 0,
        },
        end => bound(end).ok_or_else(unsupported)?,
    };
    let offset = |bound: &Bound| bound.place.abs() != i128::MAX && bound.sql != CURRENT_ROW;
    if start.place > end.place
        || start.place == i128::MAX
        || end.place == -i128::MAX
        || (kind == RANGE_FRAME && (offset(&start) || offset(&end)))
    {
        return Err(unsupported());
    }
    Ok(format!("{kind} BETWEEN {} AND {}", start.sql, end.sql))
}

/// One frame bound: unbounded, the current row, or a non-negative integer literal before or after it.
fn bound(bound: &Value) -> Option<Bound> {
    match bound {
        Value::String(text) => match text.as_str() {
            "UnboundedPreceding" => Some(Bound {
                sql: "UNBOUNDED PRECEDING".to_owned(),
                place: -i128::MAX,
            }),
            "UnboundedFollowing" => Some(Bound {
                sql: "UNBOUNDED FOLLOWING".to_owned(),
                place: i128::MAX,
            }),
            "CurrentRow" => Some(Bound {
                sql: CURRENT_ROW.to_owned(),
                place: 0,
            }),
            _ => None,
        },
        Value::Object(fields) => {
            let [(direction, offset)] = fields.iter().collect::<Vec<_>>()[..] else {
                return None;
            };
            let Some(("literal", literal)) = node_kind(offset) else {
                return None;
            };
            let Ok(literal) = understood(literal, "literal", &["literal_type", "value"]) else {
                return None;
            };
            let rows = digits_value(field(literal, "value").as_str()?)
                .filter(|rows| *rows <= MAX_WINDOW_COUNT)?;
            let integer = field(literal, "literal_type").as_str() == Some("number");
            match (integer, direction.as_str()) {
                (true, "Preceding") => Some(Bound {
                    sql: format!("{rows} PRECEDING"),
                    place: -i128::from(rows),
                }),
                (true, "Following") => Some(Bound {
                    sql: format!("{rows} FOLLOWING"),
                    place: i128::from(rows),
                }),
                _ => None,
            }
        }
        _ => None,
    }
}
