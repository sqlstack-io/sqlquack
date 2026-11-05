//! `results_baseline`: the outcomes a report holds, as `results-baseline.json` records them.

use crate::results::constants::BASELINE_FORMAT;
use crate::results::models::{ResultsBaseline, ResultsReport};

/// Every statement's outcome and every form's state in `report`.
pub fn results_baseline(report: &ResultsReport) -> ResultsBaseline {
    ResultsBaseline {
        format: BASELINE_FORMAT.to_owned(),
        statements: report.statement_outcomes.clone(),
        forms: report
            .form_results
            .iter()
            .map(|form| (form.id.clone(), form.state))
            .collect(),
    }
}
