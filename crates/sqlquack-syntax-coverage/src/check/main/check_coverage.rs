//! `check_coverage`: every form's verdict, failing closed with a named reason.

use std::collections::BTreeMap;

use crate::check::_helpers::receipts::case_outcome;
use crate::check::_helpers::references::reference_problems;
use crate::check::_helpers::reviews::{current_exclusions, review_reason};
use crate::check::models::{
    CheckResult, FormResult, FormState, LocalVerdict, ObligationResult, Reason,
};
use crate::formats::_helpers::ids::obligation_form;
use crate::formats::models::{
    Case, Form, FormStatus, Obligation, Receipt, SourceStatus, SyntaxCoverage,
};

/// Checks every catalogued form against `parse`, the current parser, built from `parser_tree`.
pub fn check_coverage(
    coverage: &SyntaxCoverage,
    parser_tree: Option<&str>,
    parse: &dyn Fn(&str) -> LocalVerdict,
) -> CheckResult {
    let forms: BTreeMap<&str, &Form> = coverage
        .catalogue
        .values()
        .flatten()
        .map(|form| (form.id.as_str(), form))
        .collect();
    let problems: Vec<String> = reference_problems(coverage, &forms);
    let mut by_form: BTreeMap<&str, Vec<&Obligation>> = BTreeMap::new();
    for (family, obligations) in &coverage.obligations {
        for obligation in obligations {
            let form: &str = obligation_form(&obligation.id, family).unwrap_or_default();
            by_form.entry(form).or_default().push(obligation);
        }
    }
    let mut cases: BTreeMap<&str, Vec<&Case>> = BTreeMap::new();
    for case in coverage.cases.values().flatten() {
        for obligation in &case.obligations {
            cases.entry(obligation.as_str()).or_default().push(case);
        }
    }
    let mut receipts: BTreeMap<&str, Vec<&Receipt>> = BTreeMap::new();
    for receipt in &coverage.receipts {
        receipts
            .entry(receipt.case_id.as_str())
            .or_default()
            .push(receipt);
    }
    let outcomes: BTreeMap<&str, Result<bool, Reason>> = coverage
        .cases
        .values()
        .flatten()
        .map(|case| {
            let found: &[&Receipt] = receipts
                .get(case.case_id.as_str())
                .map_or(&[], Vec::as_slice);
            (
                case.case_id.as_str(),
                case_outcome(case, found, parse(&case.sql)),
            )
        })
        .collect();
    let (excluded_forms, excluded_sources) =
        current_exclusions(&coverage.reviews, &coverage.file_hashes, parser_tree);
    let obligation_result = |obligation: &Obligation| -> ObligationResult {
        let citing: &[&Case] = cases.get(obligation.id.as_str()).map_or(&[], Vec::as_slice);
        let results: Vec<Result<bool, Reason>> = citing
            .iter()
            .map(|case| outcomes[case.case_id.as_str()])
            .collect();
        let reason: Option<Reason> = match results.is_empty() {
            true => Some(Reason::MissingCase),
            false => results.iter().filter_map(|result| result.err()).min(),
        };
        ObligationResult {
            id: obligation.id.clone(),
            required: obligation.required,
            reason,
            semantic_only: reason.is_none() && results.contains(&Ok(true)),
        }
    };
    let mut results: Vec<FormResult> = Vec::new();
    for (family, family_forms) in &coverage.catalogue {
        let review: Option<Reason> = review_reason(
            family,
            &coverage.reviews,
            &coverage.file_hashes,
            parser_tree,
        );
        for form in family_forms {
            let obligations: Vec<ObligationResult> = by_form
                .get(form.id.as_str())
                .map_or(&[][..], Vec::as_slice)
                .iter()
                .map(|obligation| obligation_result(obligation))
                .collect();
            let required: Vec<&ObligationResult> = obligations
                .iter()
                .filter(|result| result.required)
                .collect();
            let mut reasons: Vec<Reason> =
                required.iter().filter_map(|result| result.reason).collect();
            if required.is_empty() {
                reasons.push(Reason::NoObligations);
            }
            reasons.extend(review);
            reasons.sort_unstable();
            reasons.dedup();
            let state: FormState = match () {
                () if form.status == FormStatus::AbsentFromSource => FormState::Absent,
                () if excluded_forms.contains(&form.id) => FormState::Excluded,
                () if reasons.is_empty() => FormState::Verified,
                () => FormState::Unverified,
            };
            results.push(FormResult {
                id: form.id.clone(),
                family: family.clone(),
                source: form.source.clone(),
                state,
                satisfied_obligations: required
                    .iter()
                    .filter(|result| result.reason.is_none())
                    .count(),
                required_obligations: required.len(),
                reasons,
                obligations,
            });
        }
    }
    let unexcluded_pages: Vec<String> = coverage
        .sources
        .iter()
        .filter(|source| {
            source.status != SourceStatus::Forms && !excluded_sources.contains(&source.id)
        })
        .map(|source| source.id.clone())
        .collect();
    CheckResult {
        parser_tree: parser_tree.map(str::to_owned),
        forms: results,
        unexcluded_pages,
        problems,
    }
}
