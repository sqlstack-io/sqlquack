//! `conversion_options`: the known divergences the policy allows in one mode, and its budget.

use sqlquack_policy::policy::models::{DivergenceMode, Policy};
use sqlquack_rules::conversion::models::{AllowedDivergence, ConversionBudget, ConversionOptions};

/// The divergences `policy` converts with a warning, in strict mode or by default.
pub fn conversion_options(policy: &Policy, strict: bool) -> ConversionOptions {
    let mut options = ConversionOptions::default();
    if let Some(budget) = policy.conversion_budget {
        options.budget = ConversionBudget {
            milliseconds: budget.milliseconds,
            size_ratio: budget.size_ratio,
            minimum_bytes: budget.minimum_bytes,
        };
    }
    for (id, divergence) in &policy.divergences {
        let mode = match strict {
            true => divergence.strict,
            false => divergence.default,
        };
        if mode == DivergenceMode::Warn {
            options.divergences.insert(
                id.clone(),
                AllowedDivergence {
                    constructs: divergence.constructs.clone(),
                    reason: divergence.reason.clone(),
                },
            );
        }
    }
    options
}
