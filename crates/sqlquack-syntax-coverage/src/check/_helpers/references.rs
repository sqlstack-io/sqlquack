//! References between files that must resolve.

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::_helpers::ids::obligation_form;
use crate::formats::models::{Form, FormStatus, SyntaxCoverage};

/// Every unresolved reference: obligations of uncatalogued forms, cases citing undeclared
/// obligations, aliases and exclusions naming unknown or still-documented forms.
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
    let sources: BTreeSet<&str> = coverage
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect();
    for review in &coverage.reviews {
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
