//! `measure_translation`: what conversion makes of every case Snowflake parsed, by form.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Mutex;

use crate::check::main::check_coverage::check_coverage;
use crate::check::models::{CheckResult, LocalVerdict};
use crate::formats::main::obligation_form::obligation_form;
use crate::formats::models::{Case, Expect, FormStatus, Phase, SyntaxCoverage};
use crate::translation::_helpers::census::{load_census, schema_json};
use crate::translation::_helpers::classify::classify;
use crate::translation::_helpers::convert::Converter;
use crate::translation::_helpers::forms::{form_translation, refusal_ranking, tally};
use crate::translation::constants::NO_RECORDED_RESULT;
use crate::translation::models::{
    CaseTranslation, CensusEntry, FormTranslation, TranslationReport, TranslationTally,
};

/// Where the inputs are and how conversion runs.
pub struct TranslationInputs<'a> {
    pub coverage: &'a SyntaxCoverage,
    /// The coverage root, holding the census sidecars.
    pub root: &'a Path,
    /// Constructs with a current by-design sign-off.
    pub by_design: &'a BTreeSet<String>,
    pub binary: &'a Path,
    pub policy: &'a Path,
    /// A scratch directory for each case's schema file.
    pub work: &'a Path,
    pub family: Option<&'a str>,
    pub jobs: usize,
}

/// Every case whose current receipt says Snowflake parsed it and that does not expect a rejection.
fn counted_cases<'a>(coverage: &'a SyntaxCoverage, check: &CheckResult) -> Vec<&'a Case> {
    let parsed: BTreeSet<&str> = check
        .forms
        .iter()
        .flat_map(|form| &form.obligations)
        .flat_map(|obligation| &obligation.cases)
        .filter(|case| {
            matches!(
                case.phase,
                Some(Phase::Accepted | Phase::SemanticError | Phase::RuntimeError)
            )
        })
        .map(|case| case.case_id.as_str())
        .collect();
    coverage
        .cases
        .values()
        .flatten()
        .filter(|case| {
            parsed.contains(case.case_id.as_str()) && case.expect != Some(Expect::Reject)
        })
        .collect()
}

/// The forms a case's obligations belong to, sorted and distinct.
fn case_forms(case: &Case, families: &BTreeMap<&str, &str>) -> Vec<String> {
    case.obligations
        .iter()
        .filter_map(|obligation| {
            families
                .iter()
                .find_map(|(family, _)| obligation_form(obligation, family))
        })
        .map(str::to_owned)
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// Converts one case, with its census schema written under `work` when it has one.
fn translate(
    case: &Case,
    census: Option<&CensusEntry>,
    (inputs, converter): (&TranslationInputs, &Converter),
    forms: Vec<String>,
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
        forms,
        census_schema: census.is_some(),
        empty_setup: case.setup.is_empty(),
        outcome: classify(
            output,
            inputs.by_design,
            (census.is_some(), case.setup.is_empty()),
        ),
    }
}

/// The report, or every problem with the census sidecars.
pub fn measure_translation(inputs: &TranslationInputs) -> Result<TranslationReport, Vec<String>> {
    let coverage: &SyntaxCoverage = inputs.coverage;
    let check: CheckResult = check_coverage(coverage, None, &|_| LocalVerdict::Rejected);
    let known: BTreeSet<&str> = coverage
        .cases
        .values()
        .flatten()
        .map(|case| case.case_id.as_str())
        .collect();
    let census: BTreeMap<String, CensusEntry> = load_census(inputs.root, &known)?;
    let families: BTreeMap<&str, &str> = coverage
        .catalogue
        .keys()
        .map(|family| (family.as_str(), family.as_str()))
        .collect();
    let catalogued: BTreeMap<&str, &str> = coverage
        .catalogue
        .iter()
        .flat_map(|(family, forms)| {
            forms
                .iter()
                .filter(|form| form.status != FormStatus::AbsentFromSource)
                .map(move |form| (form.id.as_str(), family.as_str()))
        })
        .filter(|(_, family)| inputs.family.is_none_or(|wanted| wanted == *family))
        .collect();
    let queue: Vec<(&Case, Vec<String>)> = counted_cases(coverage, &check)
        .into_iter()
        .map(|case| (case, case_forms(case, &families)))
        .filter(|(_, forms)| {
            forms
                .iter()
                .any(|form| catalogued.contains_key(form.as_str()))
        })
        .collect();
    let converter = Converter {
        binary: inputs.binary,
        policy: inputs.policy,
        bound: crate::translation::constants::CONVERSION_BOUND,
    };
    let next: Mutex<std::vec::IntoIter<(&Case, Vec<String>)>> = Mutex::new(queue.into_iter());
    let done: Mutex<Vec<CaseTranslation>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..inputs.jobs.max(1) {
            scope.spawn(|| {
                loop {
                    let item = next.lock().map(|mut queue| queue.next()).ok().flatten();
                    let Some((case, forms)) = item else {
                        break;
                    };
                    let result =
                        translate(case, census.get(&case.case_id), (inputs, &converter), forms);
                    if let Ok(mut done) = done.lock() {
                        done.push(result);
                    }
                }
            });
        }
    });
    let mut cases: Vec<CaseTranslation> = done.into_inner().unwrap_or_default();
    cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    let mut by_form: BTreeMap<&str, Vec<&CaseTranslation>> = BTreeMap::new();
    for case in &cases {
        for form in &case.forms {
            by_form.entry(form.as_str()).or_default().push(case);
        }
    }
    let forms: Vec<FormTranslation> = catalogued
        .iter()
        .map(|(id, family)| {
            form_translation(id, family, by_form.get(id).map_or(&[][..], Vec::as_slice))
        })
        .collect();
    let mut families_tally: Vec<(String, TranslationTally)> = Vec::new();
    for family in catalogued.values().collect::<BTreeSet<&&str>>() {
        let family_forms: Vec<&FormTranslation> = forms
            .iter()
            .filter(|form| form.family == **family)
            .collect();
        let family_cases: Vec<&CaseTranslation> = cases
            .iter()
            .filter(|case| {
                case.forms
                    .iter()
                    .any(|id| catalogued.get(id.as_str()) == Some(family))
            })
            .collect();
        families_tally.push(((*family).to_string(), tally(&family_forms, &family_cases)));
    }
    Ok(TranslationReport {
        total: tally(
            &forms.iter().collect::<Vec<_>>(),
            &cases.iter().collect::<Vec<_>>(),
        ),
        families: families_tally,
        refusals: refusal_ranking(&forms),
        results_match: NO_RECORDED_RESULT.to_owned(),
        forms,
        cases,
    })
}
