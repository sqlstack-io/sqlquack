//! GROUP BY ALL: the positions of the SELECT items that hold no aggregate.

use serde_json::Value;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::snowflake::conversion::_helpers::expressions::named_aggregates::is_named_aggregate;
use crate::snowflake::conversion::_helpers::expressions::scalar::call_name;
use crate::snowflake::conversion::_helpers::support::tree::{
    Payload, list, node_construct, node_kind,
};
use crate::snowflake::conversion::constants::{
    AGGREGATE_KINDS, GROUP_BY_ALL, GROUP_BY_ALL_REFUSED_KINDS, STAR_KIND,
};

/// The 1-based positions GROUP BY ALL groups by: every SELECT item without an aggregate.
pub(crate) fn group_all_positions(select: &Payload) -> Result<Vec<u64>, Refusal> {
    let refuse = |reason: &str| outside_conditions(node_construct("group_by"), reason.to_owned());
    let mut positions: Vec<u64> = Vec::new();
    for (item, position) in list(select, "expressions").iter().zip(1_u64..) {
        let found = visit(item, Found::default());
        if node_kind(item).is_some_and(|(kind, _)| kind == STAR_KIND) {
            return Err(refuse(
                "GROUP BY ALL beside a star, whose columns shift positions",
            ));
        }
        if found.refused {
            return Err(refuse(
                "GROUP BY ALL of an item holding a window function or subquery",
            ));
        }
        if !found.aggregate {
            positions.push(position);
        }
    }
    if positions.is_empty() {
        return Err(refuse(GROUP_BY_ALL));
    }
    Ok(positions)
}

#[derive(Default)]
struct Found {
    aggregate: bool,
    refused: bool,
}

/// Records what `node` holds, at any depth.
fn visit(node: &Value, found: Found) -> Found {
    let Some((kind, payload)) = node_kind(node) else {
        return visit_children(node, found);
    };
    let found = match kind {
        _ if AGGREGATE_KINDS.contains(&kind) => Found {
            aggregate: true,
            ..found
        },
        _ if GROUP_BY_ALL_REFUSED_KINDS.contains(&kind) => Found {
            refused: true,
            ..found
        },
        "function" if is_named_aggregate(&call_name(payload)) => Found {
            aggregate: true,
            ..found
        },
        _ => found,
    };
    visit_children(payload, found)
}

fn visit_children(value: &Value, found: Found) -> Found {
    match value {
        Value::Array(items) => items.iter().fold(found, |found, item| visit(item, found)),
        Value::Object(fields) => fields
            .values()
            .fold(found, |found, field| visit(field, found)),
        _ => found,
    }
}
