//! `write_translation_statements`: every case's outcome and constructs, as a JSON file.

use std::collections::BTreeMap;
use std::path::Path;

use sqlquack_evidence::cases::models::LoadedCase;
use sqlquack_evidence::verify::models::VerifyReport;
use sqlquack_evidence::verify::types::CaseConverter;

use crate::statements::_helpers::constructs::{constructs, own_functions, project_key};
use crate::statements::_helpers::outcomes::outcome;
use crate::statements::constants::STATEMENTS_FORMAT;
use crate::statements::models::{Statement, StatementsFile};

/// Writes every case's statement to `path`, and how many, or why it could not.
pub fn write_translation_statements(
    path: &Path,
    cases: &[LoadedCase],
    report: &VerifyReport,
    converter: &dyn CaseConverter,
) -> Result<usize, String> {
    let cases: BTreeMap<&str, &LoadedCase> = cases
        .iter()
        .map(|case| (case.reference.as_str(), case))
        .collect();
    let mut statements: Vec<Statement> = Vec::new();
    for verdict in &report.cases {
        let Some(case) = cases.get(verdict.reference.as_str()) else {
            return Err(format!("{}: no such case", verdict.reference));
        };
        let (outcome, codes, refused) = outcome(case, verdict, converter);
        let own = own_functions(case);
        statements.push(Statement {
            reference: verdict.reference.clone(),
            hash: verdict.hash.clone(),
            outcome,
            codes,
            refused: refused
                .into_iter()
                .map(|key| project_key(key, &own))
                .collect(),
            constructs: constructs(case, &own)?,
        });
    }
    let count = statements.len();
    let file = StatementsFile {
        format: STATEMENTS_FORMAT,
        statements,
    };
    let text = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())? + "\n";
    std::fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(count)
}
