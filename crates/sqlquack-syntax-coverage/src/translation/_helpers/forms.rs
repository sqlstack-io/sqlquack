//! Each form's state from its counted cases, and the tallies by family.

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::models::{FormStatus, SyntaxCoverage};
use crate::translation::constants::NO_RECORDED_RESULT;
use crate::translation::models::{
    CaseOutcome, CaseTranslation, FormState, FormTranslation, TranslationReport, TranslationTally,
};

/// Every catalogued form still in its source, by id with its family; only `family` when given.
pub(crate) fn catalogued_forms<'a>(
    coverage: &'a SyntaxCoverage,
    family: Option<&str>,
) -> BTreeMap<&'a str, &'a str> {
    let mut catalogued: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, forms) in &coverage.catalogue {
        if family.is_some_and(|wanted| wanted != name) {
            continue;
        }
        for form in forms {
            if form.status != FormStatus::AbsentFromSource {
                catalogued.insert(form.id.as_str(), name.as_str());
            }
        }
    }
    catalogued
}

/// Whether a case counts in `family`: one of its forms is catalogued there.
fn counts_in(case: &CaseTranslation, family: &str, catalogued: &BTreeMap<&str, &str>) -> bool {
    case.forms
        .iter()
        .any(|id| catalogued.get(id.as_str()) == Some(&family))
}

/// One tally per family with a catalogued form.
fn family_tallies(
    forms: &[FormTranslation],
    cases: &[CaseTranslation],
    catalogued: &BTreeMap<&str, &str>,
) -> Vec<(String, TranslationTally)> {
    let families: BTreeSet<&str> = catalogued.values().copied().collect();
    let mut tallies: Vec<(String, TranslationTally)> = Vec::new();
    for family in families {
        let family_forms: Vec<&FormTranslation> =
            forms.iter().filter(|form| form.family == family).collect();
        let family_cases: Vec<&CaseTranslation> = cases
            .iter()
            .filter(|case| counts_in(case, family, catalogued))
            .collect();
        tallies.push((family.to_owned(), tally(&family_forms, &family_cases)));
    }
    tallies
}

/// The report on the converted cases: each catalogued form's state, the tallies and refusals.
pub(crate) fn report(
    cases: Vec<CaseTranslation>,
    catalogued: &BTreeMap<&str, &str>,
) -> TranslationReport {
    let mut by_form: BTreeMap<&str, Vec<&CaseTranslation>> = BTreeMap::new();
    for case in &cases {
        for form in &case.forms {
            by_form.entry(form.as_str()).or_default().push(case);
        }
    }
    let mut forms: Vec<FormTranslation> = Vec::new();
    for (id, family) in catalogued {
        let counted: &[&CaseTranslation] = by_form.get(id).map_or(&[][..], Vec::as_slice);
        forms.push(form_translation(id, family, counted));
    }
    let total: TranslationTally = tally(
        &forms.iter().collect::<Vec<_>>(),
        &cases.iter().collect::<Vec<_>>(),
    );
    TranslationReport {
        total,
        families: family_tallies(&forms, &cases, catalogued),
        refusals: refusal_ranking(&forms),
        results_match: NO_RECORDED_RESULT.to_owned(),
        forms,
        cases,
    }
}

/// The form's state: measured cases decide it, and a form with none is not measured.
pub(crate) fn form_translation(
    id: &str,
    family: &str,
    cases: &[&CaseTranslation],
) -> FormTranslation {
    let measured: Vec<&CaseOutcome> = cases
        .iter()
        .map(|case| &case.outcome)
        .filter(|outcome| outcome.measured())
        .collect();
    let translated: usize = measured
        .iter()
        .filter(|outcome| matches!(outcome, CaseOutcome::Translated { .. }))
        .count();
    let rest: Vec<&&CaseOutcome> = measured
        .iter()
        .filter(|outcome| !matches!(outcome, CaseOutcome::Translated { .. }))
        .collect();
    let all_rest =
        |test: fn(&CaseOutcome) -> bool| !rest.is_empty() && rest.iter().all(|o| test(o));
    let state: FormState = match () {
        () if cases.is_empty() => FormState::NotMeasurable,
        () if measured.is_empty()
            && cases
                .iter()
                .any(|case| case.outcome == CaseOutcome::NeedsSchema) =>
        {
            FormState::NeedsSchema
        }
        () if measured.is_empty() => FormState::NoRelationInScope,
        () if rest.is_empty() => FormState::Translates,
        () if all_rest(|o| matches!(o, CaseOutcome::ByDesign { .. })) => FormState::ByDesign,
        () if all_rest(|o| {
            matches!(
                o,
                CaseOutcome::ByDesign { .. } | CaseOutcome::DeclaredValueDivergence { .. }
            )
        }) =>
        {
            FormState::DeclaredValueDivergence
        }
        () => FormState::NotYetTranslated,
    };
    let bound: Vec<bool> = cases
        .iter()
        .filter(|case| case.empty_setup && case.outcome.measured())
        .map(|case| case.outcome == CaseOutcome::Translated { binds: Some(true) })
        .collect();
    let refusals: BTreeSet<String> = rest
        .iter()
        .flat_map(|outcome| match outcome {
            CaseOutcome::NotYetTranslated { refusals } => refusals.clone(),
            CaseOutcome::DidNotFinish { .. } => vec!["conversion did not finish".to_owned()],
            _ => Vec::new(),
        })
        .collect();
    FormTranslation {
        id: id.to_owned(),
        family: family.to_owned(),
        state,
        counted_cases: cases.len(),
        measured_cases: measured.len(),
        translated_cases: translated,
        binds: (!bound.is_empty()).then(|| bound.iter().all(|binds| *binds)),
        refusals: refusals.into_iter().collect(),
    }
}

/// The tally of `forms` and the cases they count.
pub(crate) fn tally(forms: &[&FormTranslation], cases: &[&CaseTranslation]) -> TranslationTally {
    let count = |state: FormState| forms.iter().filter(|form| form.state == state).count();
    let not_measurable: usize = count(FormState::NotMeasurable);
    let needs_schema: usize = count(FormState::NeedsSchema);
    let no_relation: usize = count(FormState::NoRelationInScope);
    let counted: usize = forms.len() - not_measurable;
    let case_count =
        |test: fn(&CaseOutcome) -> bool| cases.iter().filter(|c| test(&c.outcome)).count();
    TranslationTally {
        forms: forms.len(),
        not_measurable_forms: not_measurable,
        counted_forms: counted,
        needs_schema_forms: needs_schema,
        no_relation_in_scope_forms: no_relation,
        measurable_forms: counted - needs_schema - no_relation,
        translated_forms: count(FormState::Translates),
        by_design_forms: count(FormState::ByDesign),
        declared_value_divergence_forms: count(FormState::DeclaredValueDivergence),
        not_yet_translated_forms: count(FormState::NotYetTranslated),
        binding_forms: forms.iter().filter(|form| form.binds == Some(true)).count(),
        bind_measured_forms: forms.iter().filter(|form| form.binds.is_some()).count(),
        counted_cases: cases.len(),
        translated_cases: case_count(|o| matches!(o, CaseOutcome::Translated { .. })),
        needs_schema_cases: case_count(|o| *o == CaseOutcome::NeedsSchema),
        no_relation_in_scope_cases: case_count(|o| *o == CaseOutcome::NoRelationInScope),
        did_not_finish_cases: case_count(|o| matches!(o, CaseOutcome::DidNotFinish { .. })),
    }
}

/// Not-yet-translated refusals by the number of forms each blocks, most first, then by name.
fn refusal_ranking(forms: &[FormTranslation]) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for refusal in forms.iter().flat_map(|form| &form.refusals) {
        *counts.entry(refusal.as_str()).or_default() += 1;
    }
    let mut ranked: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(refusal, count)| (refusal.to_owned(), count))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
}
