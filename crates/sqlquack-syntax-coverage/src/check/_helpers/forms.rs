//! One form's result from its obligations' cases and its family review.

use std::collections::BTreeSet;

use crate::check::models::{CaseResult, FormResult, FormState, Joined, ObligationResult, Reason};
use crate::formats::models::{Form, FormStatus, Obligation};

/// Satisfied only when at least one case cites it and every citing case agrees.
fn obligation_result(obligation: &Obligation, joined: &Joined) -> ObligationResult {
    let cases: Vec<CaseResult> = joined
        .citing
        .get(obligation.id.as_str())
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(|case_id| joined.cases.get(case_id).cloned())
        .collect();
    let reason: Option<Reason> = match cases.is_empty() {
        true => Some(Reason::MissingCase),
        false => cases.iter().filter_map(|case| case.reason).min(),
    };
    ObligationResult {
        id: obligation.id.clone(),
        kind: obligation.kind,
        target: obligation.target.clone(),
        required: obligation.required,
        reason,
        semantic_only: reason.is_none() && cases.iter().any(|case| case.semantic_only),
        cases,
    }
}

/// Every reason the form is not verified, sorted and distinct.
fn form_reasons(obligations: &[ObligationResult], review: Option<Reason>) -> Vec<Reason> {
    let required: Vec<&ObligationResult> = obligations
        .iter()
        .filter(|result| result.required)
        .collect();
    let mut reasons: Vec<Reason> = required.iter().filter_map(|result| result.reason).collect();
    if required.is_empty() {
        reasons.push(Reason::NoObligations);
    }
    reasons.extend(review);
    reasons.sort_unstable();
    reasons.dedup();
    reasons
}

/// The form's state: absent, excluded by a current review, verified, or unverified.
pub(crate) fn form_result(
    form: &Form,
    family: &str,
    joined: &Joined,
    review: Option<Reason>,
    excluded: &BTreeSet<String>,
) -> FormResult {
    let obligations: Vec<ObligationResult> = joined
        .obligations
        .get(form.id.as_str())
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|obligation| obligation_result(obligation, joined))
        .collect();
    let reasons: Vec<Reason> = form_reasons(&obligations, review);
    let state: FormState = match () {
        () if form.status == FormStatus::AbsentFromSource => FormState::Absent,
        () if excluded.contains(&form.id) => FormState::Excluded,
        () if reasons.is_empty() => FormState::Verified,
        () => FormState::Unverified,
    };
    let required: Vec<&ObligationResult> = obligations.iter().filter(|o| o.required).collect();
    FormResult {
        id: form.id.clone(),
        family: family.to_owned(),
        source: form.source.clone(),
        also_sources: form.also_sources.clone(),
        state,
        satisfied_obligations: required.iter().filter(|o| o.reason.is_none()).count(),
        required_obligations: required.len(),
        reasons,
        obligations,
    }
}
