//! `caller_choices`: conversion options with what the caller confirms and refuses.

use crate::snowflake::conversion::models::ConversionOptions;
use sqlquack_policy::policy::constants::{DECLARED_OFFSET, PROJECT_UDF};
use sqlquack_policy::policy::models::Policy;

use crate::snowflake::analysis::_helpers::refuse_value_divergences::refuse_value_divergences;
use crate::snowflake::analysis::main::confirm_assumption::confirm_assumption;
use crate::snowflake::analysis::models::ConversionChoices;

/// `options` with the caller's confirmed assumptions, less value divergences if only loud ones.
pub fn caller_choices(
    options: ConversionOptions,
    policy: &Policy,
    choices: &ConversionChoices,
) -> ConversionOptions {
    let options = match choices.confirm_declared_offsets {
        true => confirm_assumption(options, policy, DECLARED_OFFSET),
        false => options,
    };
    let options = match choices.python_runtime {
        true => confirm_assumption(options, policy, PROJECT_UDF),
        false => options,
    };
    match choices.loud_divergences_only {
        true => refuse_value_divergences(options, policy),
        false => options,
    }
}
