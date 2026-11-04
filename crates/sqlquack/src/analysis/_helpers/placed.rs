//! `placed`: a refusal pointing into the source, so that every diagnostic has a span.

use sqlquack_core::parsing::main::node_location::node_location;
use sqlquack_core::parsing::models::SourceLines;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_syntax::Expression;

/// The refusal at its own place, else at its statement, else over the whole source.
pub(crate) fn placed(mut refusal: Refusal, statement: &Expression, lines: &SourceLines) -> Refusal {
    refusal.location = refusal
        .location
        .or_else(|| node_location(statement, lines))
        .or_else(|| Some(lines.whole()));
    refusal
}
