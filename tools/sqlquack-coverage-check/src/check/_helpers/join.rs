//! Joining forms, obligations, cases and receipts, and judging every case once.

use std::collections::BTreeMap;

use crate::check::_helpers::receipts::case_result;
use crate::check::models::{CaseResult, Joined, LocalVerdict, Reason};
use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::{Form, Obligation, Receipt, SyntaxCoverage, UnobservableCause};

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

/// The receipts of each statement, by its SQL, setup and session hashes, whatever the case id.
fn receipts_by_statement(coverage: &SyntaxCoverage) -> BTreeMap<(&str, &str, &str), Vec<&Receipt>> {
    let mut receipts: BTreeMap<(&str, &str, &str), Vec<&Receipt>> = BTreeMap::new();
    for receipt in &coverage.receipts {
        receipts
            .entry((
                receipt.sql_sha256.as_str(),
                receipt.setup_sha256.as_str(),
                receipt.session_sha256.as_str(),
            ))
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
    let statements: BTreeMap<(&str, &str, &str), Vec<&Receipt>> = receipts_by_statement(coverage);
    let mut cases: BTreeMap<&str, CaseResult> = BTreeMap::new();
    for case in coverage.cases.values().flatten() {
        let found: &[&Receipt] = receipts
            .get(case.case_id.as_str())
            .map_or(&[], Vec::as_slice);
        let statement: &[&Receipt] = statements
            .get(&(
                case.sql_sha256.as_str(),
                case.setup_sha256.as_str(),
                case.session_sha256.as_str(),
            ))
            .map_or(&[], Vec::as_slice);
        cases.insert(
            case.case_id.as_str(),
            case_result(case, found, statement, parse(&case.sql)),
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
            .map(|entry| {
                let reason: Reason = match entry.cause {
                    Some(UnobservableCause::NotSubmitted) => Reason::UnverifiableNotSubmitted,
                    None => Reason::UnverifiableInternalError,
                };
                (entry.obligation.as_str(), reason)
            })
            .collect(),
        readings: coverage
            .readings
            .iter()
            .map(|reading| (reading.form.as_str(), reading))
            .collect(),
    }
}
