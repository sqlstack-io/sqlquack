//! Statements linked to forms, each form's result, and the report built from them.

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::models::SyntaxCoverage;
use crate::results::_helpers::files::{group_sha256, signed, subjects};
use crate::results::_helpers::forms::{family_tallies, form_result, statement_tally};
use crate::results::_helpers::links::{FunctionIndex, link_statement};
use crate::results::_helpers::obligations::uncited_cases;
use crate::results::_helpers::worklist::worklist;
use crate::results::constants::{NOT_TRANSLATABLE_SUBJECT, QUERY_FAMILIES};
use crate::results::models::{
    FormResult, ObligationState, ObligationTally, RecordedStatement, ResultState, ResultsFiles,
    ResultsReport, StatementOutcome, TranslationObligation,
};
use crate::translation::main::catalogued_forms::catalogued_forms;

/// Each statement reference and the forms it counts for.
pub(crate) type Links<'a> = BTreeMap<&'a str, BTreeSet<String>>;

/// The forms of every current not-translatable sign-off.
fn listed_forms(files: &ResultsFiles) -> BTreeSet<&str> {
    let mut listed: BTreeSet<&str> = BTreeSet::new();
    for group in &files.not_translatable.groups {
        let subject = format!("{NOT_TRANSLATABLE_SUBJECT}{}", group.id);
        if signed(&files.review, &subject, &group_sha256(group)) {
            listed.extend(group.forms.iter().map(String::as_str));
        }
    }
    listed
}

/// The problem naming an unmapped construct and the statements using it.
fn unmapped(construct: &str, references: &[&str]) -> String {
    format!(
        "unmapped construct {construct} in {} statements, such as {}: link it to forms or list it \
         under no_form",
        references.len(),
        references.first().copied().unwrap_or_default()
    )
}

/// The forms each statement counts for: a refused one only for the forms its refusal names.
pub(crate) fn link_all<'a>(
    statements: &'a [RecordedStatement],
    functions: &FunctionIndex,
    files: &ResultsFiles,
) -> Result<Links<'a>, Vec<String>> {
    let mut links: Links = BTreeMap::new();
    let mut unaccounted: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for statement in statements {
        let linked = link_statement(statement, functions, &files.links);
        for construct in linked.unaccounted {
            unaccounted
                .entry(construct)
                .or_default()
                .push(&statement.reference);
        }
        let forms = match statement.outcome.refused() {
            true => linked.refused_forms,
            false => linked.forms,
        };
        if links.insert(&statement.reference, forms).is_some() {
            return Err(vec![format!(
                "statement {} listed twice",
                statement.reference
            )]);
        }
    }
    let problems: Vec<String> = unaccounted
        .iter()
        .map(|(construct, references)| unmapped(construct, references))
        .collect();
    match problems.is_empty() {
        true => Ok(links),
        false => Err(problems),
    }
}

/// Every catalogued form's result, or each signed-off not-translatable form a statement matches.
pub(crate) fn form_results(
    coverage: &SyntaxCoverage,
    files: &ResultsFiles,
    (statements, links): (&[RecordedStatement], &Links),
    states: &BTreeMap<String, ObligationState>,
) -> Result<Vec<FormResult>, Vec<String>> {
    let mut unmet: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut declared: BTreeSet<&str> = BTreeSet::new();
    for obligation in files
        .obligations
        .iter()
        .filter(|obligation| obligation.required)
    {
        declared.insert(obligation.form.as_str());
        if states.get(&obligation.id) != Some(&ObligationState::Met) {
            unmet
                .entry(obligation.form.as_str())
                .or_default()
                .push(obligation.id.clone());
        }
    }
    let mut linked: BTreeMap<&str, Vec<&RecordedStatement>> = BTreeMap::new();
    for statement in statements {
        let forms = links.get(statement.reference.as_str());
        for form in forms.into_iter().flatten() {
            linked.entry(form).or_default().push(statement);
        }
    }
    let listed = listed_forms(files);
    let mut problems: Vec<String> = Vec::new();
    let mut results: Vec<FormResult> = Vec::new();
    for (id, family) in catalogued_forms(coverage, None) {
        let statements: &[&RecordedStatement] = linked.get(id).map_or(&[], Vec::as_slice);
        let is_listed = listed.contains(id);
        let matching = statements
            .iter()
            .find(|statement| statement.outcome.matches());
        if let Some(matching) = matching.filter(|_| is_listed) {
            problems.push(format!(
                "{id} is listed as not translatable, but {} translates with matching results",
                matching.reference
            ));
        }
        let unmet_here = unmet.get(id).cloned().unwrap_or_default();
        results.push(form_result(
            (id, family),
            is_listed,
            statements,
            (unmet_here, declared.contains(id)),
        ));
    }
    match problems.is_empty() {
        true => Ok(results),
        false => Err(problems),
    }
}

/// Forms with no matching or refused statement: the capture queue, and the effect-receipt gap.
fn needing_forms(forms: &[FormResult]) -> (Vec<String>, Vec<String>) {
    let rank = |family: &str| QUERY_FAMILIES.iter().position(|name| *name == family);
    let mut needing: Vec<&FormResult> = forms
        .iter()
        .filter(|form| {
            matches!(
                form.state,
                ResultState::Unmeasured | ResultState::SyntaxOnly
            )
        })
        .collect();
    needing.sort_by_key(|form| (rank(&form.family), form.id.as_str()));
    let (queue, effect): (Vec<&FormResult>, Vec<&FormResult>) = needing
        .into_iter()
        .partition(|form| rank(&form.family).is_some());
    let ids = |forms: Vec<&FormResult>| forms.into_iter().map(|form| form.id.clone()).collect();
    (ids(queue), ids(effect))
}

/// The references of `statements` that pass `keep`.
fn references(
    statements: &[RecordedStatement],
    keep: impl Fn(&RecordedStatement) -> bool,
) -> Vec<String> {
    statements
        .iter()
        .filter(|statement| keep(statement))
        .map(|statement| statement.reference.clone())
        .collect()
}

/// The report over the linked statements and the forms' results.
pub(crate) fn assemble(
    files: &ResultsFiles,
    functions: FunctionIndex,
    (statements, links): (&[RecordedStatement], &Links),
    (form_results, obligation_states): (Vec<FormResult>, BTreeMap<String, ObligationState>),
) -> ResultsReport {
    let mut obligations = ObligationTally::default();
    for state in obligation_states.values() {
        obligations.declared += 1;
        *match state {
            ObligationState::Met => &mut obligations.met,
            ObligationState::Unmet => &mut obligations.unmet,
            ObligationState::Held => &mut obligations.held,
            ObligationState::Unmeasured => &mut obligations.unmeasured,
        } += 1;
    }
    let with_forms: BTreeSet<&str> = links
        .iter()
        .filter(|(_, forms)| !forms.is_empty())
        .map(|(reference, _)| *reference)
        .collect();
    let subjects: Vec<(String, bool)> = subjects(files, &functions.sha256);
    let named = |current: bool| -> Vec<String> {
        subjects
            .iter()
            .filter(|(_, signed)| *signed == current)
            .map(|(subject, _)| subject.clone())
            .collect()
    };
    let (forms, families) = family_tallies(&form_results);
    let (capture_forms, effect_receipt_forms) = needing_forms(&form_results);
    ResultsReport {
        forms,
        families,
        statements: statement_tally(statements, &with_forms),
        signed_off: named(true),
        unsigned: named(false),
        not_translatable: files.not_translatable.groups.clone(),
        worklist: worklist(statements, links, &form_results),
        capture_forms,
        effect_receipt_forms,
        capture_statements: references(statements, |statement| {
            statement.outcome == StatementOutcome::UnrecordedTranslates
        }),
        unlinked_statements: references(statements, |statement| {
            statement.outcome.recorded() && !with_forms.contains(statement.reference.as_str())
        }),
        function_links_sha256: functions.sha256,
        statement_outcomes: statements
            .iter()
            .map(|statement| (statement.reference.clone(), statement.outcome))
            .collect(),
        form_results,
        obligations,
        held: reasons(&files.obligations, |obligation| obligation.held.as_ref()),
        unmeasured: reasons(&files.obligations, |obligation| {
            obligation.reason.as_ref().filter(|_| !obligation.required)
        }),
        uncited_cases: uncited_cases(&files.obligations, statements),
        retired_obligations: files.retired.clone(),
        obligation_states,
    }
}

/// Each obligation `pick` gives a reason for, by id.
fn reasons(
    obligations: &[TranslationObligation],
    pick: fn(&TranslationObligation) -> Option<&String>,
) -> BTreeMap<String, String> {
    obligations
        .iter()
        .filter_map(|obligation| reason(obligation, pick))
        .collect()
}

/// The obligation's id and the reason `pick` gives, if any.
fn reason(
    obligation: &TranslationObligation,
    pick: fn(&TranslationObligation) -> Option<&String>,
) -> Option<(String, String)> {
    pick(obligation).map(|why| (obligation.id.clone(), why.clone()))
}
