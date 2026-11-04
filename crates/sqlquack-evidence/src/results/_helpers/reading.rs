//! Reading metadata and Arrow IPC files.

use std::io::Cursor;
use std::path::Path;

use arrow::ipc::reader::FileReader;
use arrow::record_batch::RecordBatch;
use sha2::{Digest, Sha256};
use sqlquack_compare::comparison::models::{RecordedColumn, RecordedResult};
use sqlquack_compare::values::models::ResultSet;

use crate::constants::EVIDENCE_FORMAT;
use crate::errors::EvidenceError;
use crate::results::_helpers::type_names::snowflake_type;
use crate::results::models::ResultMetadata;

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn read_metadata(path: &Path) -> Result<ResultMetadata, EvidenceError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| EvidenceError::InvalidResult(format!("{}: {error}", path.display())))?;
    let metadata: ResultMetadata = serde_json::from_str(&text)
        .map_err(|error| EvidenceError::InvalidResult(format!("{}: {error}", path.display())))?;
    if metadata.format != EVIDENCE_FORMAT {
        return Err(EvidenceError::InvalidResult(format!(
            "{}: format {} is not {EVIDENCE_FORMAT}",
            path.display(),
            metadata.format
        )));
    }
    Ok(metadata)
}

pub(crate) fn read_arrow(hash: &str, bytes: Vec<u8>) -> Result<ResultSet, EvidenceError> {
    let reader = FileReader::try_new(Cursor::new(bytes), None).map_err(|error| {
        EvidenceError::InvalidResult(format!("{hash}: not an Arrow IPC file: {error}"))
    })?;
    let schema = reader.schema();
    let batches = reader
        .collect::<Result<Vec<RecordBatch>, _>>()
        .map_err(|error| EvidenceError::InvalidResult(format!("{hash}: {error}")))?;
    Ok(ResultSet { schema, batches })
}

pub(crate) fn recorded_result(
    label: &str,
    columns: &[WarehouseColumn],
    row_count: Option<u64>,
    rows: ResultSet,
) -> Result<RecordedResult, EvidenceError> {
    if rows.schema.fields().len() != columns.len() {
        return Err(EvidenceError::InvalidResult(format!(
            "{label}: the Arrow file has {} columns, the metadata describes {}",
            rows.schema.fields().len(),
            columns.len()
        )));
    }
    let rows_read: usize = rows.batches.iter().map(RecordBatch::num_rows).sum();
    if row_count != Some(rows_read as u64) {
        return Err(EvidenceError::InvalidResult(format!(
            "{label}: the Arrow file has {rows_read} rows, the metadata says {row_count:?}"
        )));
    }
    Ok(RecordedResult {
        columns: columns
            .iter()
            .map(|column| RecordedColumn {
                name: column.name.clone(),
                snowflake_type: snowflake_type(column),
            })
            .collect(),
        rows,
    })
}
