//! WHERE-style conditions, GROUP BY, ORDER BY and LIMIT.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::main::unresolved::unresolved;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::constants::RAISE_MARKER;
use sqlquack_duckdb::blocks::main::explicit_null_order::explicit_null_order;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::operators::boolean_operand;
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::windows::{
    canonical_variant, grouping_key, order_item,
};
use crate::conversion::_helpers::queries::group_all::group_all_positions;
use crate::conversion::_helpers::support::scope::{Emitted, Scope, resolve};
use crate::conversion::_helpers::support::site::{Placement, Site};
use crate::conversion::_helpers::support::tree::{
    Payload, field, flag, identifier_name, integer_literal, list, node_construct, node_kind,
    payload_object, require_understood,
};
use crate::conversion::constants::{GROUP_BY_ALL_RULE, LIMIT, ORDER_BY, VARIANT_HELD_UNREAD};

/// A clause's condition node, such as WHERE's.
pub(crate) fn clause_condition<'a>(select: &'a Payload, clause: &str) -> Option<&'a Value> {
    select
        .get(clause)
        .filter(|value| !value.is_null())
        .map(|value| value.get("this").unwrap_or(value))
}

/// A BOOLEAN filter condition's SQL, conjuncts holding a raise gated behind the others.
pub(crate) fn condition_sql(condition: &Value, site: Site<'_>) -> Result<String, Refusal> {
    let emitted = convert_expression(condition, site)?;
    if !matches!(
        emitted.data_type,
        SnowflakeType::Boolean | SnowflakeType::Null
    ) {
        return Err(outside_conditions(
            node_construct("where"),
            format!(
                "a condition of type {} needs an implicit conversion",
                emitted.data_type
            ),
        ));
    }
    if !emitted.sql.contains(RAISE_MARKER) {
        return Ok(emitted.sql);
    }
    let conjuncts = conjuncts_of(condition);
    let mut free: Vec<String> = Vec::new();
    let mut guarded: Vec<String> = Vec::new();
    for conjunct in conjuncts {
        let sql = boolean_operand(conjunct, site)?;
        match sql.contains(RAISE_MARKER) {
            true => guarded.push(sql),
            false => free.push(sql),
        }
    }
    if free.is_empty() {
        return Ok(emitted.sql);
    }
    let free = free.join(" AND ");
    Ok(format!(
        "({free} AND CASE WHEN {free} THEN {} ELSE FALSE END)",
        guarded.join(" AND ")
    ))
}

/// The operands of nested ANDs, through parentheses.
fn conjuncts_of(condition: &Value) -> Vec<&Value> {
    match node_kind(condition) {
        Some(("and", payload)) => ["left", "right"]
            .iter()
            .filter_map(|side| payload.get(*side))
            .flat_map(conjuncts_of)
            .collect(),
        Some(("paren", payload)) if payload.get("this").is_some_and(is_and) => {
            conjuncts_of(&payload["this"])
        }
        _ => vec![condition],
    }
}

fn is_and(node: &Value) -> bool {
    matches!(node_kind(node), Some(("and", _)))
}

/// GROUP BY positions or expressions over FROM columns.
pub(crate) fn group_by(select: &Payload, site: Site<'_>, held: &[bool]) -> Result<String, Refusal> {
    let construct = node_construct("group_by");
    let group = payload_object(field(select, "group_by", &construct)?, &construct)?;
    if group.get("all").and_then(Value::as_bool) == Some(true)
        && list(group, "expressions").is_empty()
    {
        require_understood(group, &["expressions", "all"], &construct)?;
        let _all = site.context.log.rule(GROUP_BY_ALL_RULE);
        let keys = group_all_positions(select)?
            .into_iter()
            .map(|position| position_key(position, held, "GROUP BY ALL"))
            .collect::<Result<Vec<String>, Refusal>>()?;
        return Ok(format!(" GROUP BY {}", keys.join(", ")));
    }
    require_understood(group, &["expressions"], &construct)?;
    let without_aliases = Scope::new(site.scope.relations.clone(), site.scope.outer);
    let site = site.scoped(&without_aliases).placed(Placement::ROW);
    let mut keys: Vec<String> = Vec::new();
    for key in list(group, "expressions") {
        match integer_literal(key) {
            Some(position) => keys.push(position_key(position, held, "GROUP BY")?),
            None => keys.push(group_key(convert_expression(key, site)?, &construct, site)?.sql),
        }
    }
    if keys.is_empty() {
        return Err(missing(construct, "GROUP BY without keys".to_owned()));
    }
    Ok(format!(" GROUP BY {}", keys.join(", ")))
}

/// A GROUP BY key; a JSON-held VARIANT groups by the text Snowflake writes for it.
fn group_key(key: Emitted, construct: &Construct, site: Site<'_>) -> Result<Emitted, Refusal> {
    match key.data_type == SnowflakeType::SemiStructured && !key.is_variant_held() {
        true => canonical_variant(key, construct, site.context),
        false => grouping_key(key, construct),
    }
}

/// Each GROUP BY expression that is a JSON-held VARIANT, as converted and as grouped.
pub(crate) fn variant_group_keys(
    select: &Payload,
    site: Site<'_>,
) -> Result<Vec<(String, String)>, Refusal> {
    let construct = node_construct("group_by");
    let Some(group) = select.get("group_by").filter(|group| !group.is_null()) else {
        return Ok(Vec::new());
    };
    let _muted = site.context.log.muted();
    let without_aliases = Scope::new(site.scope.relations.clone(), site.scope.outer);
    let site = site.scoped(&without_aliases).placed(Placement::ROW);
    let mut keys: Vec<(String, String)> = Vec::new();
    for key in list(payload_object(group, &construct)?, "expressions") {
        if integer_literal(key).is_some() {
            continue;
        }
        let converted = convert_expression(key, site)?;
        if converted.data_type == SnowflakeType::SemiStructured && !converted.is_variant_held() {
            let sql = converted.sql.clone();
            keys.push((
                sql,
                canonical_variant(converted, &construct, site.context)?.sql,
            ));
        }
    }
    Ok(keys)
}

/// The output positions GROUP BY names.
pub(crate) fn group_positions(select: &Payload) -> Vec<u64> {
    let group = select.get("group_by").and_then(Value::as_object);
    group
        .map(|group| list(group, "expressions"))
        .unwrap_or_default()
        .iter()
        .filter_map(integer_literal)
        .collect()
}

/// A key by output position; an output held as a DuckDB VARIANT is refused, as its order is.
fn position_key(position: u64, held: &[bool], clause: &str) -> Result<String, Refusal> {
    let output = held
        .iter()
        .zip(1_u64..)
        .find(|(_, output)| *output == position);
    match output.map(|(held, _)| held) {
        Some(false) => Ok(position.to_string()),
        Some(true) => Err(outside_conditions(
            node_construct("ordered"),
            VARIANT_HELD_UNREAD.to_owned(),
        )),
        None => Err(unresolved(format!(
            "{clause} position {position} is out of range"
        ))),
    }
}

/// ORDER BY with Snowflake's NULL placement, then LIMIT and OFFSET.
pub(crate) fn order_and_limit(
    query: &Payload,
    site: Site<'_>,
    outputs: &[(String, Emitted)],
) -> Result<String, Refusal> {
    let mut sql = String::new();
    if let Some(order) = query.get("order_by").filter(|order| !order.is_null()) {
        let _muted = fixture_ordering(order, site).then(|| site.context.log.muted());
        let _rule = site.context.log.rule(ORDER_BY);
        let construct = node_construct("order_by");
        let order = payload_object(order, &construct)?;
        require_understood(order, &["expressions"], &construct)?;
        let grouped = query.get("group_by").is_some_and(|group| !group.is_null());
        let mut items: Vec<String> = Vec::new();
        for item in list(order, "expressions") {
            items.push(order_key(
                item,
                site.placed(Placement::PROJECTION),
                (outputs, grouped),
            )?);
        }
        sql.push_str(&format!(" ORDER BY {}", items.join(", ")));
    }
    for (clause, keyword) in [("limit", "LIMIT"), ("offset", "OFFSET")] {
        if let Some(value) = query.get(clause).filter(|value| !value.is_null()) {
            let _rule = site.context.log.rule(LIMIT);
            let count = value.get("this").and_then(integer_literal).ok_or_else(|| {
                outside_conditions(
                    node_construct(clause),
                    format!("{keyword} must be an integer literal"),
                )
            })?;
            sql.push_str(&format!(" {keyword} {count}"));
        }
    }
    Ok(sql)
}

/// Whether an ORDER BY only keeps the input fixture's rows in order, by its ordering column.
fn fixture_ordering(order: &Value, site: Site<'_>) -> bool {
    let Some(fixture) = &site.context.options.fixture else {
        return false;
    };
    let Some(order) = order.as_object() else {
        return false;
    };
    let keys = list(order, "expressions");
    !keys.is_empty()
        && keys.iter().all(|item| {
            let key = item.get("this").unwrap_or(&Value::Null);
            matches!(node_kind(key), Some(("column", column))
            if column.get("name").and_then(identifier_name).is_some_and(|name| {
                name.eq_ignore_ascii_case(&fixture.ordering_column)
            }))
        })
}

/// An ORDER BY key; a name both an alias and a FROM column means the alias, as batch 11 recorded.
fn order_key(
    item: &Value,
    site: Site<'_>,
    (outputs, grouped): (&[(String, Emitted)], bool),
) -> Result<String, Refusal> {
    let key = item.get("this").unwrap_or(&Value::Null);
    let object = payload_object(item, &node_construct("ordered"))?;
    if let Some(position) = integer_literal(key) {
        let held: Vec<bool> = outputs
            .iter()
            .map(|(_, emitted)| emitted.is_variant_held())
            .collect();
        let position_sql = position_key(position, &held, "ORDER BY")?;
        let ordered = outputs.iter().zip(1_u64..).find(|(_, at)| *at == position);
        if let Some(((_, output), _)) = ordered {
            grouping_key(output.clone(), &node_construct("ordered"))?;
        }
        let position = position_sql;
        return Ok(explicit_null_order(
            &position,
            flag(object, "desc"),
            object.get("nulls_first").and_then(Value::as_bool),
            site.context.log,
        ));
    }
    let shadowing = shadowing_names(key, (site, outputs));
    let Some(name) = shadowing.first() else {
        return order_item(item, site);
    };
    if grouped || needs_unrecorded_resolution(key) {
        return Err(unresolved(format!(
            "ORDER BY `{name}` names an alias shadowing a FROM column in an aggregating query \
             or beside an aggregate, window or subquery, whose resolution no recording shows"
        )));
    }
    let mut aliases_first = site.scope.clone();
    aliases_first.aliases_first = true;
    order_item(item, site.scoped(&aliases_first))
}

/// Whether `expression` holds an aggregate, a window call or a subquery.
fn needs_unrecorded_resolution(expression: &Value) -> bool {
    match node_kind(expression) {
        Some((
            "subquery" | "select" | "exists" | "count" | "sum" | "avg" | "min" | "max"
            | "window_function" | "within_group" | "aggregate_function",
            _,
        )) => true,
        _ => match expression {
            Value::Object(fields) => fields.values().any(needs_unrecorded_resolution),
            Value::Array(items) => items.iter().any(needs_unrecorded_resolution),
            _ => false,
        },
    }
}

/// Unqualified names in `expression`, outside subqueries, both an alias and a differing FROM column.
fn shadowing_names(
    expression: &Value,
    (site, outputs): (Site<'_>, &[(String, Emitted)]),
) -> Vec<String> {
    match node_kind(expression) {
        Some(("subquery" | "select" | "exists", _)) => Vec::new(),
        Some(("column", column)) => shadowing_column(column, (site, outputs))
            .into_iter()
            .collect(),
        _ => match expression {
            Value::Object(fields) => fields
                .values()
                .flat_map(|value| shadowing_names(value, (site, outputs)))
                .collect(),
            Value::Array(items) => items
                .iter()
                .flat_map(|value| shadowing_names(value, (site, outputs)))
                .collect(),
            _ => Vec::new(),
        },
    }
}

fn shadowing_column(
    column: &Value,
    (site, outputs): (Site<'_>, &[(String, Emitted)]),
) -> Option<String> {
    if !column.get("table").is_none_or(Value::is_null) {
        return None;
    }
    let name = column.get("name").and_then(identifier_name)?;
    let (_, emitted) = outputs.iter().find(|(output, _)| *output == name)?;
    let from_only = Scope::new(site.scope.relations.clone(), None);
    match resolve(&from_only, None, &name, site.context.log) {
        Ok(source) if emitted.sql != source.sql => Some(name),
        Ok(_) | Err(_) => None,
    }
}
