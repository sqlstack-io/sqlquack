//! Call spellings with recorded evidence, handed to their rules in the shape each reads.

use serde_json::{Map, Value, json};

use sqlquack_diagnostics::refusals::models::Refusal;

use crate::conversion::_helpers::support::tree::{
    Payload, function_construct, is_inert, node_construct, node_kind,
};
use crate::conversion::constants::{
    AGGREGATE_CALL_KIND, CALL_KIND, CALL_SPELLINGS, REGEXP_LIKE_NAME,
};
use crate::conversion::models::{CallShape, SpellingRule};

/// The node a call is converted as when its spelling has evidence, with that spelling.
pub(crate) fn as_typed(node: &Value) -> Option<(Value, &'static SpellingRule)> {
    let (kind, payload) = node_kind(node)?;
    let payload = payload.as_object()?;
    let aggregate = match kind {
        CALL_KIND => false,
        AGGREGATE_CALL_KIND => true,
        _ => return None,
    };
    let name = payload.get("name")?.as_str()?;
    let spelling = CALL_SPELLINGS
        .iter()
        .find(|spelling| spelling.name.eq_ignore_ascii_case(name))?;
    if !plain(payload, aggregate, spelling.shape) {
        return None;
    }
    let arguments = payload.get("args")?.as_array()?;
    let typed = shaped(spelling, arguments, payload)?;
    Some((json!({ spelling.node: typed }), spelling))
}

/// A refusal of the rule node `spelling` was converted as, named after the spelling as written.
pub(crate) fn as_written(refusal: Refusal, spelling: &SpellingRule) -> Refusal {
    let typed = node_construct(spelling.node);
    if refusal.construct.as_ref() != Some(&typed) {
        return refusal;
    }
    let written = function_construct(spelling.name);
    Refusal {
        reason: refusal.reason.replace(&typed.key(), &written.key()),
        construct: Some(written),
        ..refusal
    }
}

/// A call without a quoted name, brackets, a missing argument list, or modifiers its rule ignores.
fn plain(payload: &Payload, aggregate: bool, shape: CallShape) -> bool {
    let unquoted = ["quoted", "use_bracket_syntax", "no_parens"]
        .iter()
        .all(|field| payload.get(*field).is_none_or(is_inert));
    let unmodified = ["order_by", "limit"]
        .iter()
        .all(|field| payload.get(*field).is_none_or(is_inert));
    let scalar_only = aggregate
        || ["distinct", "filter"]
            .iter()
            .all(|field| payload.get(*field).is_none_or(is_inert));
    let nulls = shape == CallShape::FirstLast || payload.get("ignore_nulls").is_none_or(is_inert);
    unquoted && unmodified && scalar_only && nulls
}

/// The payload `spelling`'s rule reads, or `None` for an arity it does not accept.
fn shaped(spelling: &SpellingRule, arguments: &[Value], payload: &Payload) -> Option<Value> {
    let flag = |field: &str| payload.get(field).cloned().unwrap_or(Value::Bool(false));
    let filter = || payload.get("filter").cloned().unwrap_or(Value::Null);
    let typed = match (spelling.shape, arguments) {
        (CallShape::Expressions, [_, ..]) => json!({ "expressions": arguments }),
        (CallShape::Renamed, [_, ..]) => {
            json!({ "expressions": arguments, "original_name": spelling.name })
        }
        (CallShape::Fallback, [this, expression]) => {
            json!({ "this": this, "expression": expression, "original_name": spelling.name })
        }
        (CallShape::Conditional, [condition, when_true, when_false]) => json!({
            "condition": condition,
            "true_value": when_true,
            "false_value": when_false,
            "original_name": spelling.name,
        }),
        (CallShape::Unary, [this]) => json!({ "this": this }),
        (CallShape::Trim, [this, rest @ ..]) if rest.len() <= 1 => json!({
            "this": this,
            "characters": rest.first(),
            "position": "Both",
            "position_explicit": false,
            "sql_standard_syntax": false,
        }),
        (CallShape::Affix, [this, expression]) => json!({ "this": this, "expression": expression }),
        (CallShape::Substring, [this, start, rest @ ..]) if rest.len() <= 1 => json!({
            "this": this,
            "start": start,
            "length": rest.first(),
            "from_for_syntax": false,
        }),
        (CallShape::Difference, [unit, start, end]) => {
            json!({ "this": end, "expression": start, "unit": unit_var(unit)? })
        }
        (CallShape::Count, [argument]) => {
            let star = argument.get("star").is_some_and(|star| {
                ["table", "ilike", "except", "replace", "rename"]
                    .iter()
                    .all(|field| star.get(*field).is_none_or(Value::is_null))
            });
            json!({
                "this": if star { Value::Null } else { argument.clone() },
                "star": star,
                "distinct": flag("distinct"),
                "filter": filter(),
                "original_name": spelling.name,
            })
        }
        (CallShape::Aggregate, [this]) => json!({
            "this": this,
            "distinct": flag("distinct"),
            "filter": filter(),
            "name": spelling.name,
            "order_by": [],
        }),
        (CallShape::Ranking, []) => Value::Null,
        (CallShape::Offset, [this]) => json!({ "this": this }),
        (CallShape::Offset, [this, offset]) => json!({ "this": this, "offset": offset }),
        (CallShape::Offset, [this, offset, default]) => {
            json!({ "this": this, "offset": offset, "default": default })
        }
        (CallShape::FirstLast, [this]) if is_inert(&flag("distinct")) && filter().is_null() => {
            let mut typed: Map<String, Value> = Map::new();
            typed.insert("this".to_owned(), this.clone());
            if let Some(ignore) = payload.get("ignore_nulls").filter(|value| !value.is_null()) {
                typed.insert("ignore_nulls".to_owned(), ignore.clone());
            }
            Value::Object(typed)
        }
        _ => return None,
    };
    Some(typed)
}

/// A unit keyword written as an unqualified name, as the unit `var` date functions read.
fn unit_var(unit: &Value) -> Option<Value> {
    let (kind, payload) = node_kind(unit)?;
    match kind {
        "var" => Some(unit.clone()),
        "column" if payload.get("table").is_none_or(Value::is_null) => {
            let name = payload.get("name")?;
            let quoted = name.get("quoted").and_then(Value::as_bool) == Some(true);
            let text = name.get("name")?.as_str()?;
            (!quoted).then(|| json!({ "var": { "this": text } }))
        }
        _ => None,
    }
}

/// `s RLIKE p` as the call `REGEXP_LIKE(s, p)` the regular-expression rule reads, without flags.
pub(crate) fn rlike_as_call(payload: &Value) -> Option<Value> {
    let payload = payload.as_object()?;
    let understood = payload
        .iter()
        .all(|(field, value)| matches!(field.as_str(), "this" | "pattern") || is_inert(value));
    let (text, pattern) = (payload.get("this")?, payload.get("pattern")?);
    understood.then(|| {
        json!({ CALL_KIND: {
            "name": REGEXP_LIKE_NAME,
            "args": [text, pattern],
            "distinct": false,
        }})
    })
}
