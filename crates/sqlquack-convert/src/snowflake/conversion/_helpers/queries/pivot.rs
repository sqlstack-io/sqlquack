//! PIVOT in FROM: each listed key's aggregate, grouped by the source's other columns.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::types::models::ColumnSpec;

use crate::conversion::_helpers::support::tree::{
    LITERAL_STRING, Payload, field, identifier_name, list, node_construct, node_kind,
    payload_object, require_understood,
};
use crate::conversion::constants::PIVOT_AGGREGATES;

/// `pivot` over a source of `columns` as a subquery grouping its other columns, one aggregate per key.
pub(crate) fn pivot_subquery(pivot: &Payload, columns: &[ColumnSpec]) -> Result<Value, Refusal> {
    let construct = node_construct("pivot");
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    require_understood(
        pivot,
        &["this", "expressions", "fields", "alias", "alias_columns"],
        &construct,
    )?;
    let [aggregate] = list(pivot, "expressions") else {
        return Err(refuse("a PIVOT of other than one aggregate"));
    };
    let call = match node_kind(aggregate) {
        Some(("aggregate_function", call)) => payload_object(call, &construct)?,
        _ => return Err(refuse("a PIVOT aggregate other than SUM or MAX")),
    };
    require_understood(call, &["name", "args"], &construct)?;
    let name = call.get("name").and_then(Value::as_str).unwrap_or_default();
    if !PIVOT_AGGREGATES.contains(&name.to_ascii_uppercase().as_str()) {
        return Err(refuse("a PIVOT aggregate other than SUM or MAX"));
    }
    let [measured] = list(call, "args") else {
        return Err(refuse("a PIVOT aggregate of other than one column"));
    };
    let measured_name = unqualified_column(measured)
        .ok_or_else(|| refuse("a PIVOT aggregate of other than an unqualified column"))?;
    let [spread] = list(pivot, "fields") else {
        return Err(refuse("a PIVOT with other than one FOR ... IN"));
    };
    let spread = match node_kind(spread) {
        Some(("in", spread)) => payload_object(spread, &construct)?,
        _ => return Err(refuse("a PIVOT FOR other than a column IN a list")),
    };
    require_understood(spread, &["this", "expressions"], &construct)?;
    let key = field(spread, "this", &construct)?;
    let key_name = unqualified_column(key)
        .ok_or_else(|| refuse("a PIVOT FOR other than an unqualified column"))?;
    let keys = list(spread, "expressions");
    let texts: BTreeSet<&str> = keys.iter().filter_map(text_literal).collect();
    if keys.is_empty() || texts.len() != keys.len() {
        return Err(refuse(
            "a PIVOT IN list of other than distinct text literals",
        ));
    }
    let groups: Vec<&ColumnSpec> = columns
        .iter()
        .filter(|column| column.name != key_name && column.name != measured_name)
        .collect();
    if groups.len() + 2 != columns.len() {
        return Err(refuse(
            "a PIVOT whose FOR or aggregated column is not a distinct column of its source",
        ));
    }
    if groups.is_empty() {
        return Err(refuse("a PIVOT without a column to group by"));
    }
    let names = list(pivot, "alias_columns");
    if names.len() != groups.len() + keys.len() {
        return Err(refuse(
            "a PIVOT without an alias column list naming each of its columns",
        ));
    }
    let mut expressions: Vec<Value> = Vec::with_capacity(names.len());
    let mut grouping: Vec<Value> = Vec::with_capacity(groups.len());
    for (group, name) in groups.iter().zip(names) {
        let column = json!({ "column": { "name": { "name": group.name, "quoted": true } } });
        expressions.push(json!({ "alias": { "this": column, "alias": name } }));
        grouping.push(column);
    }
    for (literal, name) in keys.iter().zip(&names[groups.len()..]) {
        let case = json!({ "case": { "whens": [[{ "eq": { "left": key, "right": literal } }, measured]] } });
        let mut keyed = call.clone();
        keyed.insert("args".to_owned(), json!([case]));
        expressions
            .push(json!({ "alias": { "this": { "aggregate_function": keyed }, "alias": name } }));
    }
    let source = field(pivot, "this", &construct)?;
    let select = json!({ "select": {
        "expressions": expressions,
        "from": { "expressions": [source] },
        "group_by": { "expressions": grouping },
    } });
    let mut subquery = Map::from_iter([("this".to_owned(), select)]);
    if let Some(alias) = pivot.get("alias").filter(|alias| !alias.is_null()) {
        subquery.insert("alias".to_owned(), alias.clone());
    }
    Ok(Value::Object(subquery))
}

/// The name of a column node without a table.
fn unqualified_column(node: &Value) -> Option<String> {
    let ("column", column) = node_kind(node)? else {
        return None;
    };
    if column.get("table").is_some_and(|table| !table.is_null()) {
        return None;
    }
    identifier_name(column.get("name")?)
}

/// The text of a string literal node.
fn text_literal(node: &Value) -> Option<&str> {
    let ("literal", literal) = node_kind(node)? else {
        return None;
    };
    match literal.get("literal_type")?.as_str()? {
        LITERAL_STRING => literal.get("value")?.as_str(),
        _ => None,
    }
}
