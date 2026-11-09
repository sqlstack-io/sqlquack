//! IS NULL, IN, BETWEEN and EXISTS.

use serde_json::Value;
use sqlquack_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_blocks::semi_structured::main::json_text::json_text;
use sqlquack_diagnostics::refusals::main::missing::missing;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::operators::{
    comparable, same_family, temporal_text_literal,
};
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::queries::query::convert_query;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, flag, list, node_construct, node_kind, payload_object, require_understood,
};
use crate::conversion::constants::{
    BETWEEN, EXISTS, IN_LIST, IN_SUBQUERY, IS_NULL, NOT_IN_SUBQUERY, VARIANT_EQUALITY,
    VARIANT_HELD_UNREAD,
};

fn negated(payload: &serde_json::Map<String, Value>, keyword: &str) -> String {
    match flag(payload, "not") {
        true => format!("NOT {keyword}"),
        false => keyword.to_owned(),
    }
}

/// IS [NOT] NULL.
pub(crate) fn null_test(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("is_null");
    let _rule = site.context.log.rule(IS_NULL);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "not"], &construct)?;
    let inner = convert_expression(field(payload, "this", &construct)?, site)?;
    let test = match flag(payload, "not") {
        true => "IS NOT NULL",
        false => "IS NULL",
    };
    Ok(Emitted::new(
        format!("({} {test})", inner.sql),
        SnowflakeType::Boolean,
    ))
}

/// [NOT] BETWEEN.
pub(crate) fn between(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("between");
    let _rule = site.context.log.rule(BETWEEN);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "low", "high", "not"], &construct)?;
    let value = convert_expression(field(payload, "this", &construct)?, site)?;
    let low = convert_expression(field(payload, "low", &construct)?, site)?;
    let high = convert_expression(field(payload, "high", &construct)?, site)?;
    let (low_value, high_value, low, high) = match (
        same_family(value.data_type, low.data_type, &construct),
        same_family(value.data_type, high.data_type, &construct),
    ) {
        (Ok(()), Ok(())) => (value.clone(), value, low, high),
        _ => {
            let (low_value, low) = comparable(value.clone(), low, &construct, site.context)?;
            let (high_value, high) = comparable(value, high, &construct, site.context)?;
            (low_value, high_value, low, high)
        }
    };
    let sql = match low_value.sql == high_value.sql {
        true => format!(
            "({} {} {} AND {})",
            low_value.sql,
            negated(payload, "BETWEEN"),
            low.sql,
            high.sql
        ),
        false => format!(
            "({}({} >= {} AND {} <= {}))",
            negated(payload, ""),
            low_value.sql,
            low.sql,
            high_value.sql,
            high.sql
        ),
    };
    Ok(Emitted::new(sql, SnowflakeType::Boolean))
}

/// [NOT] EXISTS.
pub(crate) fn exists(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("exists");
    let _rule = site.context.log.rule(EXISTS);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "not"], &construct)?;
    let (sql, _) = convert_query(
        field(payload, "this", &construct)?,
        Some(site.scope),
        &site.context.projecting(false),
        false,
    )?;
    let sql = format!("({} ({sql}))", negated(payload, "EXISTS"));
    Ok(Emitted::new(sql, SnowflakeType::Boolean))
}

/// [NOT] IN with a list, or IN with a subquery.
pub(crate) fn in_predicate(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("in");
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &["this", "expressions", "query", "not"],
        &construct,
    )?;
    let value = convert_expression(field(payload, "this", &construct)?, site)?;
    if let Some(query) = payload.get("query").filter(|query| !query.is_null()) {
        return in_subquery(value, (query, flag(payload, "not")), site);
    }
    let _rule = site.context.log.rule(IN_LIST);
    let items: Vec<Emitted> = list(payload, "expressions")
        .iter()
        .map(|item| convert_expression(item, site))
        .collect::<Result<_, _>>()?;
    let value = match value.data_type {
        SnowflakeType::SemiStructured
            if !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.data_type == SnowflakeType::Varchar) =>
        {
            let _variant = site.context.log.rule(VARIANT_EQUALITY);
            diverge(site.context, JSON_FIDELITY, &construct)?;
            Emitted::new(
                json_text(&value.sql, site.context.log),
                SnowflakeType::Varchar,
            )
        }
        _ => value,
    };
    let mut items_sql: Vec<String> = Vec::new();
    for item in items {
        let item = temporal_text_literal(item, value.data_type, &construct, site.context)?;
        same_family(value.data_type, item.data_type, &construct)?;
        items_sql.push(item.sql);
    }
    let items = items_sql;
    if items.is_empty() {
        return Err(missing(construct, "IN with no items".to_owned()));
    }
    Ok(Emitted::new(
        format!(
            "({} {} ({}))",
            value.sql,
            negated(payload, "IN"),
            items.join(", ")
        ),
        SnowflakeType::Boolean,
    ))
}

fn in_subquery(
    value: Emitted,
    (query, negated): (&Value, bool),
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct("in");
    let _rule = site.context.log.rule(IN_SUBQUERY);
    let _negated = negated.then(|| site.context.log.rule(NOT_IN_SUBQUERY));
    let query = match node_kind(query) {
        Some(("subquery", inner)) => inner.get("this").unwrap_or(query),
        _ => query,
    };
    if negated && has_constant_branch(query) {
        return Err(outside_conditions(
            construct,
            "NOT IN a set operation with a branch without FROM, which batches 9 and 10 recorded \
             Snowflake answering as if its NULLs were absent"
                .to_owned(),
        ));
    }
    let (sql, columns) = convert_query(
        query,
        Some(site.scope),
        &site.context.projecting(false),
        false,
    )?;
    let [column] = columns.as_slice() else {
        return Err(outside_conditions(
            construct,
            "IN needs a one-column subquery".to_owned(),
        ));
    };
    if negated && !site.placement.filter && !column.not_null {
        return Err(outside_conditions(
            construct,
            "NOT IN with a subquery outside a WHERE condition, which Snowflake rejects (error 2031) \
             unless it proves the subquery holds no NULL"
                .to_owned(),
        ));
    }
    if !column.holding.is_json() {
        return Err(outside_conditions(
            construct,
            VARIANT_HELD_UNREAD.to_owned(),
        ));
    }
    same_family(value.data_type, column.data_type, &construct)?;
    let probe = Emitted::new(String::new(), column.data_type);
    let (value, _) = comparable(value, probe, &construct, site.context)?;
    let sql = format!(
        "({} {} ({sql}))",
        value.sql,
        if negated { "NOT IN" } else { "IN" }
    );
    Ok(Emitted::new(sql, SnowflakeType::Boolean))
}

/// Whether `query` is a set operation with a branch selecting without FROM.
fn has_constant_branch(query: &Value) -> bool {
    match node_kind(query) {
        Some(("union" | "intersect" | "except", payload)) => ["left", "right"]
            .iter()
            .filter_map(|side| payload.get(*side))
            .any(|branch| has_constant_branch(branch) || is_constant_select(branch)),
        _ => false,
    }
}

/// Whether `branch` is a SELECT without FROM.
fn is_constant_select(branch: &Value) -> bool {
    match node_kind(branch) {
        Some(("select", select)) => select.get("from").is_none_or(Value::is_null),
        Some(("subquery", inner)) => inner.get("this").is_some_and(is_constant_select),
        _ => false,
    }
}
