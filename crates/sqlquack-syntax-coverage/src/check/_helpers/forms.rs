//! One form's result from its obligations' cases and its family review.

use crate::check::_helpers::fingerprints::{form_definition, form_fingerprint};
use crate::check::_helpers::reviews::review_reason;
use crate::check::models::{
    CaseResult, FamilyContext, FormResult, FormState, Joined, ObligationResult, Reason,
};
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
    let unobservable: Option<Reason> = joined.unobservable.get(obligation.id.as_str()).copied();
    let reason: Option<Reason> = match () {
        () if cases.is_empty() => Some(Reason::MissingCase),
        () if unobservable.is_some() => unobservable,
        () => cases.iter().filter_map(|case| case.reason).min(),
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
pub(crate) fn form_result(form: &Form, family: &str, context: &FamilyContext) -> FormResult {
    let declared: &[&Obligation] = context
        .joined
        .obligations
        .get(form.id.as_str())
        .map_or(&[][..], Vec::as_slice);
    let obligations: Vec<ObligationResult> = declared
        .iter()
        .map(|obligation| obligation_result(obligation, context.joined))
        .collect();
    let fingerprint: String = form_fingerprint(&obligations);
    let definition_sha256: String = form_definition(form, declared);
    let review: Option<Reason> =
        review_reason(&form.id, (&fingerprint, &definition_sha256), context);
    let reasons: Vec<Reason> = form_reasons(&obligations, review);
    let state: FormState = match () {
        () if form.status == FormStatus::AbsentFromSource => FormState::Absent,
        () if context.excluded.contains(&form.id) => FormState::Excluded,
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
        fingerprint,
        definition_sha256,
        obligations,
    }
}
