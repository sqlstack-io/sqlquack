//! `declared_refusals`: what refuses a statement before conversion is tried.

use sqlquack_diagnostics::classification::models::Occurrence;
use sqlquack_diagnostics::refusals::main::refuse_constructs::refuse_constructs;
use sqlquack_diagnostics::refusals::main::snowflake_rejections::snowflake_rejections;
use sqlquack_diagnostics::refusals::models::{Refusal, RefusalCategory};
use sqlquack_diagnostics::refusals::types::ConstructPolicy;
use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::SourceLines;

/// The constructs the policy declares inexpressible, then the nodes of `source` Snowflake rejects.
pub(crate) fn declared_refusals(
    statement: &Expression,
    occurrences: &[Occurrence],
    policy: &dyn ConstructPolicy,
    (source, lines): (&str, &SourceLines),
) -> Vec<Refusal> {
    refuse_constructs(occurrences, policy)
        .into_iter()
        .filter(|refusal| refusal.category == RefusalCategory::NotExpressible)
        .chain(snowflake_rejections(statement, (source, lines)))
        .collect()
}
