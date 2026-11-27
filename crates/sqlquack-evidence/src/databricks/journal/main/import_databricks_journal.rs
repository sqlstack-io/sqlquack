//! `import_databricks_journal`: a Databricks recording journal's answers as results of pending cases.

use std::path::{Path, PathBuf};

use crate::databricks::journal::_helpers::outcomes::{create_new, imported, pending_by_sql};
use crate::databricks::journal::models::{ImportSummary, JournalRow, RowImport};
use crate::errors::EvidenceError;
use crate::pending::models::PendingCase;

/// Writes the result of every pending case a statement in `journals` answers, matched by its SQL.
pub fn import_databricks_journal(
    journals: &[PathBuf],
    (pending, results): (&Path, &Path),
    warehouse_version: &str,
) -> Result<ImportSummary, EvidenceError> {
    let cases = pending_by_sql(pending)?;
    std::fs::create_dir_all(results)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", results.display())))?;
    let mut summary = ImportSummary::default();
    for journal in journals {
        let text = std::fs::read_to_string(journal)
            .map_err(|error| EvidenceError::Io(format!("{}: {error}", journal.display())))?;
        for (number, line) in text.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let row: JournalRow = serde_json::from_str(line).map_err(|error| {
                EvidenceError::Io(format!("{}:{}: {error}", journal.display(), number + 1))
            })?;
            let Some(case) = cases.get(&row.sql_sha256) else {
                summary.unmatched += 1;
                continue;
            };
            match import_row((case, &row), (results, warehouse_version))? {
                RowImport::Rows => summary.rows += 1,
                RowImport::Error => summary.errors += 1,
                RowImport::AlreadyRecorded => summary.already_recorded += 1,
                RowImport::NotRecordable(reason) => summary
                    .not_recordable
                    .push((case.case_reference.clone(), reason)),
            }
        }
    }
    Ok(summary)
}

/// Writes `case`'s result from `row` unless it has one, or says why `row` records none.
fn import_row(
    (case, row): (&PendingCase, &JournalRow),
    (results, warehouse_version): (&Path, &str),
) -> Result<RowImport, EvidenceError> {
    let metadata_path = results.join(format!("{}.json", case.case_hash));
    if metadata_path.exists() {
        return Ok(RowImport::AlreadyRecorded);
    }
    let result = match imported((case, row), warehouse_version) {
        Ok(result) => result,
        Err(reason) => return Ok(RowImport::NotRecordable(reason)),
    };
    let written = match &result.arrow {
        Some(arrow) => {
            create_new(&results.join(format!("{}.arrow", case.case_hash)), arrow)?;
            RowImport::Rows
        }
        None => RowImport::Error,
    };
    let mut text = serde_json::to_string_pretty(&result.metadata)
        .map_err(|error| EvidenceError::Io(error.to_string()))?;
    text.push('\n');
    create_new(&metadata_path, text.as_bytes())?;
    Ok(written)
}
