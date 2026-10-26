//! `export_review`: the review records of every case, one file per case file.

use std::collections::BTreeMap;
use std::path::Path;

use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::results::main::load_result::load_result;
use sqlquack_evidence::verify::models::{CaseVerdict, VerifyReport};
use sqlquack_evidence::verify::types::CaseConverter;

use crate::review::_helpers::render::{CaseReview, render_case};
use crate::review::constants::EXPORT_EXTENSION;
use crate::review::models::ReviewFile;

/// Review files for `cases`, with verdicts and provenance from `report` and results from `results`.
pub fn export_review(
    cases: &[LoadedCase],
    report: &VerifyReport,
    results: &Path,
    converter: &dyn CaseConverter,
) -> Vec<ReviewFile> {
    let verdicts: BTreeMap<&str, &CaseVerdict> = report
        .cases
        .iter()
        .map(|verdict| (verdict.reference.as_str(), verdict))
        .collect();
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    for case in cases {
        let file = case.reference.split('#').next().unwrap_or_default();
        let path = format!(
            "{}.{EXPORT_EXTENSION}",
            file.strip_suffix(".toml").unwrap_or(file)
        );
        let review = CaseReview {
            case,
            verdict: verdicts.get(case.reference.as_str()).copied(),
            conversion: converter.convert(case),
            stored: load_result(results, &case.hash).map_err(|error| error.to_string()),
        };
        let text = files.entry(path).or_insert_with(|| {
            format!("# sqlquack review export of {file}: Snowflake results as recorded\n")
        });
        text.push('\n');
        text.push_str(&render_case(&review));
    }
    files
        .into_iter()
        .map(|(path, text)| ReviewFile { path, text })
        .collect()
}
