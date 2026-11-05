//! The counted cases: those Snowflake parsed, each with its forms, converted in parallel.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{CheckResult, LocalVerdict};
use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::{Case, Expect, Phase, SyntaxCoverage};
use crate::translation::_helpers::census::schema_json;
use crate::translation::_helpers::classify::classify;
use crate::translation::_helpers::convert::Converter;
use crate::translation::constants::CONVERSION_BOUND;
use crate::translation::models::{CaseTranslation, CensusEntry, TranslationInputs};

/// The ids of the cases whose current receipt says Snowflake parsed them.
fn parsed_case_ids(check: &CheckResult) -> BTreeSet<&str> {
    let mut parsed: BTreeSet<&str> = BTreeSet::new();
    for obligation in check.forms.iter().flat_map(|form| &form.obligations) {
        for case in &obligation.cases {
            if matches!(
                case.phase,
                Some(Phase::Accepted | Phase::SemanticError | Phase::RuntimeError)
            ) {
                parsed.insert(case.case_id.as_str());
            }
        }
    }
    parsed
}

/// The forms a case's obligations belong to, sorted and distinct.
fn case_forms(case: &Case, coverage: &SyntaxCoverage) -> Vec<String> {
    let mut forms: BTreeSet<String> = BTreeSet::new();
    for obligation in &case.obligations {
        let form: Option<&str> = coverage
            .catalogue
            .keys()
            .find_map(|family| obligation_form(obligation, family));
        forms.extend(form.map(str::to_owned));
    }
    forms.into_iter().collect()
}

/// Every case Snowflake parsed that does not expect a rejection and has a catalogued form.
pub(crate) fn queued_cases<'a>(
    coverage: &'a SyntaxCoverage,
    catalogued: &BTreeMap<&str, &str>,
) -> Vec<(&'a Case, Vec<String>)> {
    let check: CheckResult = check_coverage(coverage, None, &|_| LocalVerdict::Rejected);
    let parsed: BTreeSet<&str> = parsed_case_ids(&check);
    let mut queue: Vec<(&Case, Vec<String>)> = Vec::new();
    for case in coverage.cases.values().flatten() {
        if !parsed.contains(case.case_id.as_str()) || case.expect == Some(Expect::Reject) {
            continue;
        }
        let forms: Vec<String> = case_forms(case, coverage);
        if forms
            .iter()
            .any(|form| catalogued.contains_key(form.as_str()))
        {
            queue.push((case, forms));
        }
    }
    queue
}

/// Converts one case, with its census schema written under `work` when it has one.
fn translate(
    (case, forms): &(&Case, Vec<String>),
    census: Option<&CensusEntry>,
    (inputs, converter): (&TranslationInputs, &Converter),
) -> CaseTranslation {
    let schema = census.map(|entry| {
        let path = inputs.work.join(format!("{}.json", case.sql_sha256));
        let written = std::fs::write(&path, schema_json(entry).to_string());
        (path, written)
    });
    let output = match &schema {
        Some((_, Err(error))) => Err(format!("cannot write the census schema: {error}")),
        Some((path, Ok(()))) => converter.convert(&case.sql, Some(path)),
        None => converter.convert(&case.sql, None),
    };
    if let Some((path, _)) = &schema {
        let _ = std::fs::remove_file(path);
    }
    CaseTranslation {
        case_id: case.case_id.clone(),
        forms: forms.clone(),
        census_schema: census.is_some(),
        empty_setup: case.setup.is_empty(),
        outcome: classify(
            output,
            inputs.by_design,
            (census.is_some(), case.setup.is_empty()),
        ),
    }
}

/// Every queued case converted on `inputs.jobs` threads, sorted by case id.
pub(crate) fn convert_cases(
    queue: &[(&Case, Vec<String>)],
    census: &BTreeMap<String, CensusEntry>,
    inputs: &TranslationInputs,
) -> Vec<CaseTranslation> {
    let converter = Converter {
        binary: inputs.binary,
        policy: inputs.policy,
        bound: CONVERSION_BOUND,
    };
    let next: AtomicUsize = AtomicUsize::new(0);
    let work = || {
        let mut done: Vec<CaseTranslation> = Vec::new();
        while let Some(item) = queue.get(next.fetch_add(1, Ordering::Relaxed)) {
            done.push(translate(
                item,
                census.get(&item.0.case_id),
                (inputs, &converter),
            ));
        }
        done
    };
    let mut cases: Vec<CaseTranslation> = std::thread::scope(|scope| {
        let mut workers: Vec<std::thread::ScopedJoinHandle<Vec<CaseTranslation>>> = Vec::new();
        for _ in 0..inputs.jobs.max(1) {
            workers.push(scope.spawn(work));
        }
        let mut cases: Vec<CaseTranslation> = Vec::new();
        for worker in workers {
            match worker.join() {
                Ok(done) => cases.extend(done),
                Err(panic) => std::panic::resume_unwind(panic),
            }
        }
        cases
    });
    cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    cases
}
