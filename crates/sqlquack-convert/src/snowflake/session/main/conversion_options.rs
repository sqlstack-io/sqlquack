//! `conversion_options`: the known divergences recorded cases replay with, and the budget.

use crate::snowflake::conversion::models::{
    AllowedDivergence, ConversionBudget, ConversionOptions,
};
use sqlquack_policy::policy::models::{DivergenceMode, Policy};

/// The divergences `policy` replays its recorded cases with; none in strict mode.
pub fn conversion_options(policy: &Policy, strict: bool) -> ConversionOptions {
    let mut options = ConversionOptions::default();
    if let Some(budget) = policy.conversion_budget {
        options.budget = ConversionBudget {
            milliseconds: budget.milliseconds,
            size_ratio: budget.size_ratio,
            minimum_bytes: budget.minimum_bytes,
        };
    }
    if strict {
        return options;
    }
    for (id, divergence) in &policy.divergences {
        if divergence.replay == DivergenceMode::Warn {
            options.divergences.insert(
                id.clone(),
                AllowedDivergence {
                    constructs: divergence.constructs.clone(),
                    reason: divergence.reason.clone(),
                    assumption: divergence.assumption,
                },
            );
        }
    }
    options
}
