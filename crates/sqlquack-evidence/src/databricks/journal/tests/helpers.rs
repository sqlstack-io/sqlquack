use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sqlquack_compare::values::main::canonical_rows::canonical_rows;
use sqlquack_compare::values::models::CanonicalValue;

use crate::databricks::journal::main::import_databricks_journal::import_databricks_journal;
use crate::databricks::journal::models::ImportSummary;
use crate::databricks::journal::tests::test_types::Outcome;
use crate::results::main::load_result::load_result;

/// The pending case's source SQL; its SHA-256 names it in journals.
const SOURCE_SQL: &str = "SELECT 1 AS r";
/// Its case hash.
const CASE_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000001";

/// A journal line answering the pending case, with `fields` added.
fn journal_line(fields: &serde_json::Value) -> serde_json::Value {
    let sql_sha256: String = Sha256::digest(SOURCE_SQL.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut line = serde_json::json!({
        "key": "cases/translation/functions/f.toml#t001",
        "construct": "db.functions.f",
        "kind": "translation",
        "case_hash": "recording-tool-hash",
        "sql_sha256": sql_sha256,
        "statement_id_sha256": "ab",
        "recorded_at": "2026-11-16T17:00:09+00:00",
    });
    line.as_object_mut().expect("the line is an object").extend(
        fields
            .as_object()
            .expect("the fields are an object")
            .clone(),
    );
    line
}

/// Imports `fields`' journal line for the one pending Databricks case, into `root/results`.
fn import(root: &Path, fields: &serde_json::Value) -> ImportSummary {
    let pending = root.join("pending");
    std::fs::create_dir_all(&pending).expect("pending directory");
    let case = serde_json::json!({
        "format": 1,
        "case_hash": CASE_HASH,
        "case_reference": "translation/functions/f.toml#t001",
        "source_dialect": "databricks",
        "session_parameters": {},
        "pinned_parameters": {"ANSI_MODE": true, "TIMEZONE": "UTC"},
        "source_sql": SOURCE_SQL,
        "duckdb_version": "v2.0.0",
    });
    std::fs::write(pending.join(format!("{CASE_HASH}.json")), case.to_string())
        .expect("pending case written");
    let journal: PathBuf = root.join("out.journal.jsonl");
    std::fs::write(&journal, journal_line(fields).to_string()).expect("journal written");
    import_databricks_journal(&[journal], (&pending, &root.join("results")), "2026.45")
        .expect("the journal imports")
}

/// The one cell of a `type_text` column holding `cell`, as read back; `None` when left pending.
pub(crate) fn decoded_cell(type_text: &str, cell: Option<&str>) -> Option<CanonicalValue> {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fields = serde_json::json!({
        "state": "SUCCEEDED",
        "columns": [{"name": "r", "type_name": "X", "type_text": type_text}],
        "rows": [[cell]],
        "row_count": 1,
        "truncated": false,
    });
    import(directory.path(), &fields);
    let stored = load_result(&directory.path().join("results"), CASE_HASH).ok()??;
    let rows = canonical_rows(&stored.rows?.rows).ok()?;
    rows.first()?.first().cloned()
}

/// What importing `fields`' line twice does; a pending reason holding `fragment` becomes it.
pub(crate) fn outcome_of(fields: &serde_json::Value, fragment: Option<&str>) -> Outcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let first = import(directory.path(), fields);
    let second = import(directory.path(), fields);
    let stored = load_result(&directory.path().join("results"), CASE_HASH).expect("readable");
    Outcome {
        sqlstate: stored.and_then(|stored| stored.metadata.error?.sqlstate),
        pending_reason: first.not_recordable.first().map(|(_, reason)| {
            fragment
                .filter(|fragment| reason.contains(fragment))
                .map_or_else(|| reason.clone(), str::to_owned)
        }),
        already_recorded: second.already_recorded,
    }
}
