//! `configured_options`: the built-in defaults with the user's `[allow]` and Python runtime.

use crate::snowflake::analysis::main::confirm_assumption::confirm_assumption;
use crate::snowflake::analysis::models::ConversionChoices;
use crate::snowflake::conversion::models::{ConversionBudget, ConversionOptions};
use crate::snowflake::session::main::builtin_conversion_options::builtin_conversion_options;
use sqlquack_policy::policy::constants::PROJECT_UDF;
use sqlquack_policy::policy::main::evidence_policy::evidence_policy;

/// The defaults, with the user's `[allow]` and Python runtime outside strict mode.
pub fn configured_options(choices: &ConversionChoices) -> ConversionOptions {
    let evidence = evidence_policy();
    let mut options = builtin_conversion_options(choices.strict);
    if let Some(budget) = evidence.conversion_budget {
        options.budget = ConversionBudget {
            milliseconds: budget.milliseconds,
            size_ratio: budget.size_ratio,
            minimum_bytes: budget.minimum_bytes,
        };
    }
    if choices.strict {
        return options;
    }
    for (id, allowed) in &choices.allow {
        options = match allowed {
            true => confirm_assumption(options, evidence, id),
            false => {
                options.divergences.remove(id);
                options
            }
        };
    }
    match choices.python_runtime {
        true => confirm_assumption(options, evidence, PROJECT_UDF),
        false => options,
    }
}
