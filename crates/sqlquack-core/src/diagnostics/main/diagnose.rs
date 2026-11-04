//! `diagnose`: refusals as coded diagnostics pointing into their source.

use crate::diagnostics::_helpers::code_entry::code_entry;
use crate::diagnostics::_helpers::coded::coded;
use crate::diagnostics::_helpers::line_index::LineIndex;
use crate::diagnostics::constants::{
    CASE_ONLY_MATCH, CASE_ONLY_MATCH_HELP, CONVERSION_BUDGET, DIVERGENCE_REFUSED, GUARD_PREFIX,
    MISSING_RULE, NOT_EXPRESSIBLE, PARSE_FIDELITY, TYPE_CONDITION, UNPARSED_PREFIX,
    UNRESOLVED_NAME,
};
use crate::diagnostics::models::Diagnostic;
use crate::refusals::models::{Refusal, RefusalCategory};

/// Each refusal of `source` as a diagnostic, in the refusals' order.
pub fn diagnose(refusals: &[Refusal], source: &str) -> Vec<Diagnostic> {
    let lines = LineIndex::new(source);
    let mut diagnostics = Vec::with_capacity(refusals.len());
    for refusal in refusals {
        let (code, label) = code_and_label(refusal);
        let mut diagnostic = coded(code, label);
        diagnostic.span = refusal.location.map(|location| lines.span(location));
        if refusal.category == RefusalCategory::UnresolvedName
            && refusal.reason.contains(CASE_ONLY_MATCH)
        {
            diagnostic.help = Some(CASE_ONLY_MATCH_HELP.to_owned());
        }
        if let Some(construct) = refusal
            .construct
            .as_ref()
            .filter(|_| refusal.category != RefusalCategory::SnowflakeRejects)
        {
            diagnostic.message = format!("{}: `{construct}`", diagnostic.message);
        }
        diagnostics.push(diagnostic);
    }
    diagnostics
}

/// The code a refusal maps to, and the label left once its code's own wording is removed.
fn code_and_label(refusal: &Refusal) -> (&'static str, String) {
    let code = match refusal.category {
        RefusalCategory::ParseFidelity => PARSE_FIDELITY,
        RefusalCategory::MissingRule => MISSING_RULE,
        RefusalCategory::TypeCondition => TYPE_CONDITION,
        RefusalCategory::UnresolvedName => UNRESOLVED_NAME,
        RefusalCategory::NotExpressible => NOT_EXPRESSIBLE,
        RefusalCategory::ConversionBudget => CONVERSION_BUDGET,
        RefusalCategory::DivergenceRefused => DIVERGENCE_REFUSED,
        RefusalCategory::SnowflakeRejects => refusal
            .construct
            .as_ref()
            .and_then(|construct| code_entry(&construct.name))
            .map_or(MISSING_RULE, |entry| entry.code),
    };
    let Some(message) = refusal.reason.strip_prefix(UNPARSED_PREFIX) else {
        return (code, refusal.reason.clone());
    };
    let guard = message
        .split_once(':')
        .filter(|(name, _)| name.starts_with(GUARD_PREFIX))
        .and_then(|(name, rest)| code_entry(name).map(|entry| (entry.code, rest.trim())));
    match guard {
        Some((guard, rest)) => (guard, rest.to_owned()),
        None => (code, message.to_owned()),
    }
}
