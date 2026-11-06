//! Volatile Python function calls, which converted SQL must not repeat.

use serde_json::Value;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_policy::policy::constants::PROJECT_UDF;
use sqlquack_rules::conversion::models::Converted;
use sqlquack_semantics::types::models::{Schema, Volatility};
use sqlquack_syntax::Expression;

use crate::analysis::constants::CALL_NODE_KINDS;

/// `converted`, or a refusal where its SQL calls a volatile function more often than `statement`.
pub(crate) fn volatile_calls_kept(
    converted: Converted,
    statement: &Expression,
    schema: &Schema,
) -> Result<Converted, Refusal> {
    let tree = std::cell::OnceCell::new();
    let prefix = format!("{PROJECT_UDF}:");
    let repeated = converted
        .provenance
        .divergences
        .keys()
        .filter_map(|label| label.strip_prefix(&prefix))
        .filter(|name| {
            schema
                .functions
                .get(*name)
                .is_some_and(|spec| spec.traits.volatility == Volatility::Volatile)
        })
        .find(|name| {
            let emitted = converted.sql.matches(&format!("\"{name}\"(")).count();
            let tree = tree.get_or_init(|| serde_json::to_value(statement).unwrap_or_default());
            let written = call_nodes(tree, name);
            emitted > written
        });
    match repeated {
        None => Ok(converted),
        Some(name) => Err(Refusal {
            construct: None,
            location: None,
            category: RefusalCategory::TypeCondition,
            reason: format!(
                "the volatile Python function {name}, which the DuckDB SQL would call more often \
                 than the statement does"
            ),
        }),
    }
}

/// How many call nodes in `tree` name `name`, ignoring case.
pub(crate) fn call_nodes(tree: &Value, name: &str) -> usize {
    match tree {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| {
                let call = CALL_NODE_KINDS.contains(&key.as_str())
                    && value
                        .get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|called| called.eq_ignore_ascii_case(name));
                usize::from(call) + call_nodes(value, name)
            })
            .sum(),
        Value::Array(items) => items.iter().map(|item| call_nodes(item, name)).sum(),
        _ => 0,
    }
}
