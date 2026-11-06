//! `relation_key`: the schema relation a written, possibly qualified, relation name reads.

use sqlquack_core::refusals::main::unresolved::unresolved;
use sqlquack_core::refusals::models::Refusal;

use crate::names::constants::{AMBIGUOUS_RELATION_SUFFIX, RELATION_KEY_SEPARATOR};
use crate::types::models::Schema;

/// The declared key agreeing with the most of `parts`, read from the name backwards; a tie refuses.
pub fn relation_key(parts: &[String], schema: &Schema) -> Result<Option<String>, Refusal> {
    let mut best: Vec<&String> = Vec::new();
    let mut best_score = (0, false);
    for key in schema.relations.keys() {
        let key_parts: Vec<&str> = key.split(RELATION_KEY_SEPARATOR).collect();
        let agrees = key_parts
            .iter()
            .rev()
            .zip(parts.iter().rev())
            .all(|(key_part, written)| key_part == written);
        if !agrees {
            continue;
        }
        let score = (
            key_parts.len().min(parts.len()),
            key_parts.len() == parts.len(),
        );
        if score > best_score {
            best_score = score;
            best.clear();
        }
        if score == best_score {
            best.push(key);
        }
    }
    match best.as_slice() {
        [] => Ok(None),
        [key] => Ok(Some((*key).clone())),
        keys => Err(unresolved(format!(
            "relation `{}` matches `{}`{AMBIGUOUS_RELATION_SUFFIX}",
            parts.join(RELATION_KEY_SEPARATOR),
            keys.iter()
                .map(|key| key.as_str())
                .collect::<Vec<_>>()
                .join("`, `"),
        ))),
    }
}
