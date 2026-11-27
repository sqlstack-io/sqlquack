//! One journal statement as a result file pair, its cells kept as the text recorded.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use arrow::array::{ArrayRef, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::FileWriter;
use arrow::record_batch::RecordBatch;
use sqlquack_compare::values::models::ResultSet;
use sqlquack_policy::policy::models::EvidenceDialect;

use crate::constants::{
    EVIDENCE_FORMAT, METADATA_EXTENSION, RECORDABLE_SQLSTATE_CLASSES, SESSION_DEPENDENT_SQLSTATES,
};
use crate::databricks::journal::constants::{
    DATABRICKS_CONNECTOR, FAILED, IMPORTER, SQLSTATE_CLASS_LENGTH, SQLSTATE_LENGTH,
    SQLSTATE_MARKER, SUCCEEDED,
};
use crate::databricks::journal::models::{JournalColumn, JournalRow};
use crate::databricks::profile::main::databricks_evidence::databricks_evidence;
use crate::dialects::types::EvidenceProfile;
use crate::errors::EvidenceError;
use crate::pending::models::PendingCase;
use crate::results::main::content_sha256::content_sha256;
use crate::results::models::{RecordedFailure, RecordedOutcome, ResultMetadata, WarehouseColumn};

/// A result ready to write: its metadata and, for rows, its Arrow file.
pub(crate) struct Imported {
    /// The metadata.
    pub(crate) metadata: ResultMetadata,
    /// The Arrow IPC file of text cells, absent for an error.
    pub(crate) arrow: Option<Vec<u8>>,
}

/// Every Databricks pending case in `pending`, by the SHA-256 of its source SQL.
pub(crate) fn pending_by_sql(
    pending: &Path,
) -> Result<HashMap<String, PendingCase>, EvidenceError> {
    let io = |error: std::io::Error| EvidenceError::Io(format!("{}: {error}", pending.display()));
    let mut cases: HashMap<String, PendingCase> = HashMap::new();
    for entry in std::fs::read_dir(pending).map_err(io)? {
        let path = entry.map_err(io)?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some(METADATA_EXTENSION) {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(io)?;
        let case: PendingCase = serde_json::from_str(&text)
            .map_err(|error| EvidenceError::Io(format!("{}: {error}", path.display())))?;
        if case.source_dialect == EvidenceDialect::Databricks.name() {
            cases.insert(content_sha256(case.source_sql.as_bytes()), case);
        }
    }
    Ok(cases)
}

/// `case`'s result from `row`, checked to read back; or why `row` records no case outcome.
pub(crate) fn imported(
    (case, row): (&PendingCase, &JournalRow),
    warehouse_version: &str,
) -> Result<Imported, String> {
    match (row.state.as_str(), &row.error) {
        (SUCCEEDED, None) => {
            if row.truncated || row.row_count != Some(row.rows.len() as u64) {
                return Err(format!(
                    "the journal holds {} of {:?} rows",
                    row.rows.len(),
                    row.row_count
                ));
            }
            let (arrow, rows) = text_arrow(row)?;
            let digest = Some(content_sha256(&arrow));
            let metadata = metadata((case, row), None, (digest, warehouse_version));
            if let Some(Err(error)) = databricks_evidence().recorded_result(&metadata, &rows) {
                return Err(error.to_string());
            }
            Ok(Imported {
                metadata,
                arrow: Some(arrow),
            })
        }
        (FAILED, Some(error)) => {
            let sqlstate = sqlstate(&error.message);
            let recordable = sqlstate.as_deref().is_some_and(|state| {
                RECORDABLE_SQLSTATE_CLASSES.contains(&&state[..SQLSTATE_CLASS_LENGTH])
                    && !SESSION_DEPENDENT_SQLSTATES.contains(&state)
            });
            if !recordable {
                return Err(format!(
                    "Databricks error (SQLSTATE {}) is not a case outcome: {}",
                    sqlstate.as_deref().unwrap_or("unknown"),
                    error.message.lines().next().unwrap_or_default()
                ));
            }
            let failure = RecordedFailure {
                errno: None,
                sqlstate,
                message: Some(error.message.clone()),
            };
            Ok(Imported {
                metadata: metadata((case, row), Some(failure), (None, warehouse_version)),
                arrow: None,
            })
        }
        (state, _) => Err(format!("the statement ended {state}")),
    }
}

/// Writes `bytes` to `path`, which must not exist: results are append-only.
pub(crate) fn create_new(path: &Path, bytes: &[u8]) -> Result<(), EvidenceError> {
    let io = |error: std::io::Error| EvidenceError::Io(format!("{}: {error}", path.display()));
    let mut file = std::fs::File::create_new(path).map_err(io)?;
    file.write_all(bytes).map_err(io)
}

/// The metadata of `case`'s result from `row`, its Arrow file hashing to `arrow_sha256`.
fn metadata(
    (case, row): (&PendingCase, &JournalRow),
    error: Option<RecordedFailure>,
    (arrow_sha256, warehouse_version): (Option<String>, &str),
) -> ResultMetadata {
    let rows = error.is_none();
    let columns: Vec<WarehouseColumn> = match rows {
        true => row.columns.iter().map(warehouse_column).collect(),
        false => Vec::new(),
    };
    ResultMetadata {
        format: EVIDENCE_FORMAT,
        case_hash: case.case_hash.clone(),
        outcome: [RecordedOutcome::Error, RecordedOutcome::Rows][usize::from(rows)],
        recorder: format!("{IMPORTER} {}", env!("CARGO_PKG_VERSION")),
        connector: DATABRICKS_CONNECTOR.to_owned(),
        query_id_sha256: Some(row.statement_id_sha256.clone()),
        warehouse_version: warehouse_version.to_owned(),
        duckdb_version: case.duckdb_version.clone(),
        recorded_at: row.recorded_at.clone(),
        session_parameters: case.session_parameters.clone(),
        effective_parameters: None,
        parameters_sha256: None,
        columns,
        row_count: rows.then_some(row.rows.len() as u64),
        arrow_sha256,
        error,
        source_dialect: EvidenceDialect::Databricks,
    }
}

fn warehouse_column(column: &JournalColumn) -> WarehouseColumn {
    WarehouseColumn {
        name: column.name.clone(),
        type_code: None,
        type_name: column.type_text.clone(),
        precision: column.type_precision,
        scale: column.type_scale,
        internal_size: None,
        nullable: None,
        sub_microsecond_checked: false,
    }
}

/// The SQLSTATE a Databricks message names after its error class.
fn sqlstate(message: &str) -> Option<String> {
    let (_, rest) = message.split_once(SQLSTATE_MARKER)?;
    let state: String = rest.chars().take(SQLSTATE_LENGTH).collect();
    let valid = state.len() == SQLSTATE_LENGTH && state.chars().all(|c| c.is_ascii_alphanumeric());
    valid.then_some(state)
}

/// An Arrow IPC file of `row`'s cells, one text column per result column, and the rows it holds.
fn text_arrow(row: &JournalRow) -> Result<(Vec<u8>, ResultSet), String> {
    let width = row.columns.len();
    if row.rows.iter().any(|cells| cells.len() != width) {
        return Err("a row does not hold one cell per column".to_owned());
    }
    let mut fields: Vec<Field> = Vec::with_capacity(width);
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(width);
    for (index, column) in row.columns.iter().enumerate() {
        fields.push(Field::new(&column.name, DataType::Utf8, true));
        let cells: StringArray = row
            .rows
            .iter()
            .map(|cells| cells[index].as_deref())
            .collect();
        columns.push(Arc::new(cells));
    }
    let schema = Arc::new(Schema::new(fields));
    let batch = RecordBatch::try_new(Arc::clone(&schema), columns).map_err(|e| e.to_string())?;
    let mut bytes: Vec<u8> = Vec::new();
    let mut writer = FileWriter::try_new(&mut bytes, &schema).map_err(|e| e.to_string())?;
    writer.write(&batch).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    drop(writer);
    let rows = ResultSet {
        schema,
        batches: vec![batch],
    };
    Ok((bytes, rows))
}
