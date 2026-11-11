//! `diagnose`: refusals as coded diagnostics pointing into their source.

use crate::classification::models::ConstructFamily;
use crate::diagnostics::_helpers::code_entry::code_entry;
use crate::diagnostics::_helpers::coded::coded;
use crate::diagnostics::_helpers::jinja::holds_jinja;
use crate::diagnostics::_helpers::line_index::LineIndex;
use crate::diagnostics::_helpers::plain_keys::plain_keys;
use crate::diagnostics::constants::{
    CASE_ONLY_MATCH, CASE_ONLY_MATCH_HELP, CONVERSION_BUDGET, DIVERGENCE_REFUSED,
    FUNCTION_OPERAND_HELP, GUARD_PREFIX, JINJA_HELP, MISSING_RULE, NOT_EXPRESSIBLE, PARSE_FIDELITY,
    REPEATED_RESULT_NAME, REPEATED_RESULT_NAME_HELP, SUGGESTED_NAME_MAX_CHARS, TYPE_CONDITION,
    UNALIASED_OUTPUT_NAME, UNALIASED_QUOTED_NAME, UNALIASED_QUOTED_NAME_HELP, UNPARSED_PREFIX,
    UNRESOLVED_NAME, ZONED_LOCAL_TIME, ZONED_LOCAL_TIME_HELP, ZONED_LOCAL_TIME_NOTE,
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
        if refusal.category == RefusalCategory::UnresolvedName
            && refusal.reason.contains(REPEATED_RESULT_NAME)
        {
            diagnostic.help = Some(REPEATED_RESULT_NAME_HELP.to_owned());
        }
        if refusal.category == RefusalCategory::ParseFidelity && holds_jinja(refusal, source) {
            diagnostic.help = Some(JINJA_HELP.to_owned());
        }
        if refusal.category == RefusalCategory::TypeCondition
            && refusal.reason.contains(ZONED_LOCAL_TIME)
        {
            diagnostic.notes.push(ZONED_LOCAL_TIME_NOTE.to_owned());
            diagnostic.help = Some(ZONED_LOCAL_TIME_HELP.to_owned());
        }
        if code == TYPE_CONDITION
            && refusal
                .construct
                .as_ref()
                .is_some_and(|construct| construct.family == ConstructFamily::Function)
            && diagnostic.help.as_deref() != Some(ZONED_LOCAL_TIME_HELP)
        {
            diagnostic.help = Some(FUNCTION_OPERAND_HELP.to_owned());
        }
        if code == UNALIASED_OUTPUT_NAME && refusal.reason.contains(UNALIASED_QUOTED_NAME) {
            diagnostic.help = Some(UNALIASED_QUOTED_NAME_HELP.to_owned());
        } else if code == UNALIASED_OUTPUT_NAME
            && let Some(name) = diagnostic
                .span
                .and_then(|span| suggested_name(source, span.start, span.end))
        {
            diagnostic.help = Some(format!(
                "add an alias, such as `AS {name}`, which names the column in both engines"
            ));
        }
        if let Some(construct) = refusal
            .construct
            .as_ref()
            .filter(|_| refusal.category != RefusalCategory::SnowflakeRejects)
            .filter(|_| code == MISSING_RULE || refusal.category != RefusalCategory::MissingRule)
        {
            let plain = construct.plain_name();
            diagnostic.message = match code {
                MISSING_RULE => format!("SQLQuack can't convert {plain} yet"),
                TYPE_CONDITION => format!("SQLQuack can't convert {plain} for these types"),
                NOT_EXPRESSIBLE => format!("DuckDB can't do what Snowflake does with {plain}"),
                _ => format!("{}: {plain}", diagnostic.message),
            };
        }
        diagnostic.label = plain_keys(&diagnostic.label);
        diagnostics.push(diagnostic);
    }
    diagnostics
}

/// The code a refusal maps to, and the label left once its code's own wording is removed.
fn code_and_label(refusal: &Refusal) -> (&'static str, String) {
    let code = match refusal.category {
        RefusalCategory::ParseFidelity => PARSE_FIDELITY,
        RefusalCategory::MissingRule => refusal
            .construct
            .as_ref()
            .filter(|construct| construct.family == ConstructFamily::Syntax)
            .and_then(|construct| code_entry(&construct.name))
            .map_or(MISSING_RULE, |entry| entry.code),
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

/// The item text at `start..end` upper-cased and quoted as an alias, if short printable ASCII.
fn suggested_name(source: &str, start: usize, end: usize) -> Option<String> {
    let text: String = source
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();
    let printable = !text.is_empty()
        && text.chars().count() <= SUGGESTED_NAME_MAX_CHARS
        && text.bytes().all(|byte| (b' '..=b'~').contains(&byte));
    printable.then(|| format!("\"{}\"", text.to_ascii_uppercase().replace('"', "\"\"")))
}
