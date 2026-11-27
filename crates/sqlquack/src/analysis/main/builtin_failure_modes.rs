//! `builtin_failure_modes`: the failure modes each built-in rule declares.

use sqlquack_convert::databricks::conversion::constants::FAILURE_MODES;
use sqlquack_convert::snowflake::catalogue::constants::RULES;
use sqlquack_evidence::verify::types::FailureModes;

/// For each rule declaring failure modes, those modes.
pub fn builtin_failure_modes() -> FailureModes {
    RULES
        .iter()
        .filter(|rule| !rule.failures.is_empty())
        .map(|rule| (rule.id.to_owned(), rule.failures.to_vec()))
        .chain(
            FAILURE_MODES
                .iter()
                .map(|(rule, failures)| ((*rule).to_owned(), failures.to_vec())),
        )
        .collect()
}
