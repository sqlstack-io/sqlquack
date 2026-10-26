//! `builtin_failure_modes`: the failure modes each built-in rule declares.

use sqlquack_evidence::verify::types::FailureModes;
use sqlquack_rules::conversion::constants::RULES;

/// For each rule declaring failure modes, those modes.
pub fn builtin_failure_modes() -> FailureModes {
    RULES
        .iter()
        .filter(|rule| !rule.failures.is_empty())
        .map(|rule| (rule.id.to_owned(), rule.failures.to_vec()))
        .collect()
}
