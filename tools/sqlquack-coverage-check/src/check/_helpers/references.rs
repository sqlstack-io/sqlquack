//! References between files that must resolve.

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::{Case, Form, FormStatus, SyntaxCoverage, UnobservableCause};

/// Every unresolved reference between the catalogue, obligations, cases, aliases and reviews.
pub(crate) fn reference_problems(
    coverage: &SyntaxCoverage,
    forms: &BTreeMap<&str, &Form>,
) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    let mut obligations: BTreeSet<&str> = BTreeSet::new();
    for (family, family_obligations) in &coverage.obligations {
        for obligation in family_obligations {
            obligations.insert(obligation.id.as_str());
            let form: &str = obligation_form(&obligation.id, family).unwrap_or_default();
            if !forms.contains_key(form) {
                problems.push(format!(
                    "obligation {}: form {form} is not catalogued",
                    obligation.id
                ));
            }
        }
    }
    for case in coverage.cases.values().flatten() {
        for obligation in &case.obligations {
            if !obligations.contains(obligation.as_str()) {
                problems.push(format!(
                    "case {}: obligation {obligation} is not declared",
                    case.case_id
                ));
            }
        }
    }
    for alias in &coverage.aliases {
        if forms
            .get(alias.from.as_str())
            .is_none_or(|form| form.status != FormStatus::AbsentFromSource)
        {
            problems.push(format!(
                "alias {}: not catalogued as absent_from_source",
                alias.from
            ));
        }
        for to in alias
            .to
            .iter()
            .filter(|to| !forms.contains_key(to.as_str()))
        {
            problems.push(format!("alias {}: {to} is not catalogued", alias.from));
        }
    }
    let cases: BTreeMap<&str, &Case> = coverage
        .cases
        .values()
        .flatten()
        .map(|case| (case.case_id.as_str(), case))
        .collect();
    let receipted: BTreeSet<(&str, &str)> = coverage
        .receipts
        .iter()
        .filter(|receipt| receipt.measure.is_none())
        .map(|receipt| (receipt.case_id.as_str(), receipt.sql_sha256.as_str()))
        .collect();
    for entry in &coverage.unobservable {
        if entry.cause == Some(UnobservableCause::NotSubmitted) {
            for listed in entry.cases.iter().filter(|listed| {
                receipted.contains(&(listed.case_id.as_str(), listed.sql_sha256.as_str()))
            }) {
                problems.push(format!(
                    "unobservable {}: not_submitted, but case {} has a receipt; remove the entry",
                    entry.obligation, listed.case_id
                ));
            }
        }
        if !obligations.contains(entry.obligation.as_str()) {
            problems.push(format!(
                "unobservable {}: obligation is not declared",
                entry.obligation
            ));
        }
        for listed in &entry.cases {
            let resolved: bool = cases.get(listed.case_id.as_str()).is_some_and(|case| {
                case.sql_sha256 == listed.sql_sha256 && case.obligations.contains(&entry.obligation)
            });
            if !resolved {
                problems.push(format!(
                    "unobservable {}: case {} is not a case with that sql_sha256 citing it",
                    entry.obligation, listed.case_id
                ));
            }
        }
    }
    for reading in &coverage.readings {
        if !forms.contains_key(reading.form.as_str()) {
            problems.push(format!("reading {}: form is not catalogued", reading.form));
        }
    }
    let sources: BTreeSet<&str> = coverage
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect();
    for review in &coverage.reviews {
        for form in review.forms.iter().flatten() {
            if !forms.contains_key(form.as_str()) {
                problems.push(format!(
                    "review {}: form {form} is not catalogued",
                    review.id
                ));
            }
        }
        for exclusion in &review.exclusions {
            let known: bool = match (&exclusion.form, &exclusion.source) {
                (Some(form), _) => forms.contains_key(form.as_str()),
                (_, Some(source)) => sources.contains(source.as_str()),
                _ => false,
            };
            if !known {
                problems.push(format!(
                    "review {}: excludes an unknown form or page",
                    review.id
                ));
            }
        }
    }
    problems
}
