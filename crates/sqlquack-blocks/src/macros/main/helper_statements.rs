//! `helper_statements`: the statements defining the helper macros some SQL calls.

use std::collections::BTreeSet;

use crate::macros::_helpers::registry::{definition, helper_macros};
use crate::macros::constants::{CREATE_HELPER_SCHEMA, HELPER_CALL_PREFIX};

/// The schema and every helper macro `sql` calls, dependencies first; empty when it calls none.
pub fn helper_statements(sql: &[&str]) -> Vec<String> {
    let macros = helper_macros();
    let mut used: BTreeSet<&str> = BTreeSet::new();
    let mut pending: Vec<String> = sql.iter().map(|text| (*text).to_owned()).collect();
    while let Some(text) = pending.pop() {
        for helper in &macros {
            if !used.contains(helper.name) && calls(&text, helper.name) {
                used.insert(helper.name);
                pending.push((helper.body)());
            }
        }
    }
    if used.is_empty() {
        return Vec::new();
    }
    std::iter::once(CREATE_HELPER_SCHEMA.to_owned())
        .chain(
            macros
                .iter()
                .filter(|helper| used.contains(helper.name))
                .map(definition),
        )
        .collect()
}

/// Whether `text` calls the helper `name`, with any whitespace before its parenthesis.
fn calls(text: &str, name: &str) -> bool {
    let called = format!("{HELPER_CALL_PREFIX}{name}");
    text.match_indices(&called)
        .any(|(at, _)| text[at + called.len()..].trim_start().starts_with('('))
}
