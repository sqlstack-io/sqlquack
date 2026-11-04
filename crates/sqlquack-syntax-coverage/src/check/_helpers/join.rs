//! Joining forms, obligations, cases and receipts, and judging every case once.

use std::collections::BTreeMap;

use crate::check::_helpers::receipts::case_result;
use crate::check::models::{CaseResult, Joined, LocalVerdict};
use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::{Form, Obligation, Receipt, SyntaxCoverage};

fn forms_by_id(coverage: &SyntaxCoverage) -> BTreeMap<&str, &Form> {
    coverage
        .catalogue
        .values()
        .flatten()
        .map(|form| (form.id.as_str(), form))
        .collect()
}

fn obligations_by_form(coverage: &SyntaxCoverage) -> BTreeMap<&str, Vec<&Obligation>> {
    let mut by_form: BTreeMap<&str, Vec<&Obligation>> = BTreeMap::new();
    for (family, obligations) in &coverage.obligations {
        for obligation in obligations {
            let form: &str = obligation_form(&obligation.id, family).unwrap_or_default();
            by_form.entry(form).or_default().push(obligation);
        }
    }
    by_form
}

fn cases_by_obligation(coverage: &SyntaxCoverage) -> BTreeMap<&str, Vec<&str>> {
    let mut citing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for case in coverage.cases.values().flatten() {
        for obligation in &case.obligations {
            citing
                .entry(obligation.as_str())
                .or_default()
                .push(case.case_id.as_str());
        }
    }
    citing
}

fn receipts_by_case(coverage: &SyntaxCoverage) -> BTreeMap<&str, Vec<&Receipt>> {
    let mut receipts: BTreeMap<&str, Vec<&Receipt>> = BTreeMap::new();
    for receipt in &coverage.receipts {
        receipts
            .entry(receipt.case_id.as_str())
            .or_default()
            .push(receipt);
    }
    receipts
}

/// Every join the checker needs, with each case judged against `parse`.
pub(crate) fn join<'a>(
    coverage: &'a SyntaxCoverage,
    parse: &dyn Fn(&str) -> LocalVerdict,
) -> Joined<'a> {
    let receipts: BTreeMap<&str, Vec<&Receipt>> = receipts_by_case(coverage);
    let mut cases: BTreeMap<&str, CaseResult> = BTreeMap::new();
    for case in coverage.cases.values().flatten() {
        let found: &[&Receipt] = receipts
            .get(case.case_id.as_str())
            .map_or(&[], Vec::as_slice);
        cases.insert(
            case.case_id.as_str(),
            case_result(case, found, parse(&case.sql)),
        );
    }
    Joined {
        forms: forms_by_id(coverage),
        obligations: obligations_by_form(coverage),
        citing: cases_by_obligation(coverage),
        cases,
        unobservable: coverage
            .unobservable
            .iter()
            .map(|entry| entry.obligation.as_str())
            .collect(),
    }
}
