//! TIMESTAMP_TZ source columns whose schema declares their offset, and the assumption they carry.

use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::conversion::_helpers::support::scope::{Context, Emitted};
use sqlquack_policy::policy::constants::DECLARED_OFFSET;

/// The declared columns any of `values` relies on, sorted and without repeats.
pub(crate) fn declared_of<'a>(values: impl Iterator<Item = &'a Emitted>) -> Vec<String> {
    merged(
        values
            .flat_map(|value| value.declared.iter().cloned())
            .collect(),
    )
}

/// `labels` sorted and without repeats.
pub(crate) fn merged(labels: Vec<String>) -> Vec<String> {
    let mut labels = labels;
    labels.sort();
    labels.dedup();
    labels
}

/// Relies on the declared offsets behind a UTC mark, each as `declared_offset:<column>`.
pub(crate) fn rely_on_declared(
    declared: &[String],
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(), Refusal> {
    if declared.is_empty() {
        return Ok(());
    }
    if !context.options.divergences.contains_key(DECLARED_OFFSET) {
        return Err(outside_conditions(
            construct.clone(),
            format!(
                "{} relies on the offset the schema declares for {}, assumption `{DECLARED_OFFSET}`, \
                 which the policy does not allow in this mode",
                construct.key(),
                declared.join(", ")
            ),
        ));
    }
    for label in declared {
        context
            .log
            .divergence(&format!("{DECLARED_OFFSET}:{label}"), &construct.key());
    }
    Ok(())
}
