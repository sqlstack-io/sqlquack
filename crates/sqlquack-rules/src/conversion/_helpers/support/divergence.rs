//! Relying on a known divergence the policy allows, or refusing.

use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::refused_divergence::refused_divergence;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_snowflake_types::typing::constants::MAX_PRECISION;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::support::scope::Context;

use sqlquack_policy::policy::constants::NUMERIC_RANGE;

/// Records the divergence at `construct` when the policy allows it here; refuses otherwise.
pub(crate) fn diverge(
    context: &Context<'_>,
    divergence: &str,
    construct: &Construct,
) -> Result<(), Refusal> {
    let key = construct.key();
    match context.options.divergences.get(divergence) {
        Some(allowed) if allowed.constructs.contains(&key) => {
            context.log.divergence(divergence, &key);
            Ok(())
        }
        _ => Err(refused_divergence(construct.clone(), divergence)),
    }
}

/// Whether the policy allows the divergence at `construct`, recording it when it does.
pub(crate) fn allows(context: &Context<'_>, divergence: &str, construct: &Construct) -> bool {
    let key = construct.key();
    let allowed = context
        .options
        .divergences
        .get(divergence)
        .is_some_and(|allowed| allowed.constructs.contains(&key));
    if allowed {
        context.log.divergence(divergence, &key);
    }
    allowed
}

/// NUMBERs compared or unified as one DECIMAL of over 38 digits rely on `numeric_range`.
pub(crate) fn require_widening_range(
    types: &[SnowflakeType],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(), Refusal> {
    if types.contains(&SnowflakeType::Float) {
        return Ok(());
    }
    let numbers: Vec<(u8, u8)> = types
        .iter()
        .filter_map(|data_type| match data_type {
            SnowflakeType::Number { precision, scale } => Some((precision - scale, *scale)),
            _ => None,
        })
        .collect();
    let whole = numbers.iter().map(|(whole, _)| *whole).max().unwrap_or(0);
    let scale = numbers.iter().map(|(_, scale)| *scale).max().unwrap_or(0);
    match numbers.len() > 1 && u16::from(whole) + u16::from(scale) > u16::from(MAX_PRECISION) {
        true => diverge(context, NUMERIC_RANGE, construct),
        false => Ok(()),
    }
}
