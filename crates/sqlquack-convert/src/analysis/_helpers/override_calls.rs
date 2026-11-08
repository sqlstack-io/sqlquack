//! User overrides, which converted SQL must apply to every call of the function they replace.

use serde_json::Value;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::{Refusal, RefusalCategory};
use sqlquack_duckdb_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_rules::conversion::main::override_macro_name::override_macro_name;
use sqlquack_rules::conversion::models::{ConversionOptions, Converted};
use sqlquack_syntax::Expression;

use crate::analysis::_helpers::volatile_calls::call_nodes;

/// `converted`, or a refusal where a call of an overridden function does not reach the override.
pub(crate) fn overrides_applied(
    converted: Converted,
    statement: &Expression,
    options: &ConversionOptions,
) -> Result<Converted, Refusal> {
    if options.overrides.is_empty() {
        return Ok(converted);
    }
    let tree = serde_json::to_value(statement).unwrap_or_default();
    let log = ProvenanceLog::default();
    let skipped = options.overrides.iter().find(|(replaces, replacement)| {
        let written = call_nodes(&tree, replaces) + dedicated_nodes(&tree, replaces);
        let macro_call = format!(
            "{}(",
            quoted_identifier(&override_macro_name(&replacement.name), &log)
        );
        converted.sql.matches(&macro_call).count() < written
    });
    match skipped {
        None => Ok(converted),
        Some((replaces, replacement)) => Err(Refusal {
            construct: None,
            location: None,
            category: RefusalCategory::MissingRule,
            reason: format!(
                "a call of {replaces} the user override {} does not reach here, which SQLQuack \
                 never converts past an override",
                replacement.name
            ),
        }),
    }
}

/// How many nodes the parser keeps as `name`'s own node kind, keyed by its lower-case name.
fn dedicated_nodes(tree: &Value, name: &str) -> usize {
    let key = name.to_ascii_lowercase();
    match tree {
        Value::Object(fields) => fields
            .iter()
            .map(|(field, value)| usize::from(*field == key) + dedicated_nodes(value, name))
            .sum(),
        Value::Array(items) => items.iter().map(|item| dedicated_nodes(item, name)).sum(),
        _ => 0,
    }
}
