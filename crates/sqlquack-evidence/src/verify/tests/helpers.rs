use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use arrow::array::{ArrayRef, Decimal128Array, RecordBatch};
use arrow::datatypes::DataType;
use arrow::ipc::writer::FileWriter;
use sqlquack_compare::values::models::ResultSet;
use sqlquack_policy::policy::main::load_policy::load_policy;

use crate::cases::main::load_cases::load_cases;
use crate::pending::main::write_pending::write_pending;
use crate::pending::models::PendingCase;
use crate::replay::main::replay_on_duckdb::replay_on_duckdb;
use crate::replay::models::ReplaySettings;
use crate::results::_helpers::reading::sha256_hex;
use crate::results::models::{RecordedOutcome, ResultMetadata, WarehouseColumn};
use crate::verify::main::verify_cases::verify_cases;
use crate::verify::models::Verdict;
use crate::verify::tests::test_types::{Tampering, UnchangedSqlForTests};

/// Marks every stand-in result: produced by DuckDB in a test, never warehouse evidence.
const STAND_IN_RECORDER: &str = "test fixture: DuckDB stand-in, not evidence";

const POLICY: &str = r#"
schema_version = 1

[comparator]
column_names = "case_insensitive"
null_order = "nulls_last_on_asc_first_on_desc"
timestamp_tz = "instant"

[comparator.float_ulps]
float32 = 0
float64 = 0

[[comparator.type_equality]]
snowflake = "NUMBER(p,s)"
duckdb = "DECIMAL(p,s)"
reason = "same values"

[[comparator.type_equality]]
snowflake = "NUMBER(10,0)"
duckdb = "INTEGER"
reason = "test fixture: DuckDB stand-ins describe INTEGER as NUMBER(10,0)"

[[comparator.type_equality]]
snowflake = "VARCHAR(n)"
duckdb = "VARCHAR"
reason = "length does not change values"

[session_profiles.utc-v1]
description = "UTC"
duckdb_time_zone = "UTC"

[session_profiles.utc-v1.snowflake]
TIMEZONE = "UTC"

[edge_cases.numeric]
description = "numbers"
required = ["null"]
"#;

const CASE_FILE: &str = r#"
category = "numeric"
construct = "node:add"
session_profile = "utc-v1"

[[case]]
id = "decimal_addition"
description = "Decimal addition with NULL."
columns = [{ name = "a", type = "DECIMAL(10,2)" }, { name = "b", type = "DECIMAL(10,2)" }]
rows = [["1.25", "2.50"], ["NULL", "1.00"], ["-3.00", "0.75"]]
expression = "a + b"
tags = ["null"]

[[case]]
id = "sorted_bag"
description = "Values compared as a bag."
columns = [{ name = "price", type = "DECIMAL(10,2)" }]
rows = [["3.00"], ["1.00"]]
query = "SELECT price FROM input"
order = "bag"
tags = []
"#;

fn warehouse_column(name: &str, data_type: &DataType) -> WarehouseColumn {
    let (type_name, precision, scale, internal_size) = match data_type {
        DataType::Decimal128(precision, scale) => (
            "FIXED",
            Some(i64::from(*precision)),
            Some(i64::from(*scale)),
            None,
        ),
        DataType::Int32 => ("FIXED", Some(10), Some(0), None),
        _ => ("TEXT", None, None, Some(16_777_216)),
    };
    WarehouseColumn {
        name: name.to_ascii_uppercase(),
        type_code: 0,
        type_name: type_name.to_owned(),
        precision,
        scale,
        internal_size,
        nullable: Some(true),
    }
}

fn changed_value(rows: &ResultSet) -> Vec<RecordBatch> {
    let batch = &rows.batches[0];
    let last = batch.num_columns() - 1;
    let changed: ArrayRef = match batch.column(last).data_type() {
        DataType::Decimal128(precision, scale) => Arc::new(
            Decimal128Array::from(vec![Some(1); batch.num_rows()])
                .with_precision_and_scale(*precision, *scale)
                .expect("valid decimal"),
        ),
        _ => batch.column(last).clone(),
    };
    let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
    columns[last] = changed;
    vec![RecordBatch::try_new(batch.schema(), columns).expect("same schema")]
}

fn write_arrow(path: &Path, rows: &ResultSet, batches: &[RecordBatch]) -> Vec<u8> {
    let file = File::create(path).expect("arrow file created");
    let mut writer = FileWriter::try_new(file, &rows.schema).expect("arrow writer");
    for batch in batches {
        writer.write(batch).expect("batch written");
    }
    writer.finish().expect("arrow file finished");
    std::fs::read(path).expect("arrow file read back")
}

fn record_stand_in(pending: &PendingCase, results: &Path, tampering: Tampering) {
    let settings = ReplaySettings {
        time_zone: "UTC".to_owned(),
        null_order: "nulls_last_on_asc_first_on_desc".to_owned(),
    };
    let rows = replay_on_duckdb(&pending.source_sql, &settings).expect("DuckDB runs the case");
    let arrow_path = results.join(format!("{}.arrow", pending.case_hash));
    let recorded_bytes = write_arrow(&arrow_path, &rows, &rows.batches);
    let tampered = [Tampering::ValueChanged, Tampering::ArrowEdited].contains(&tampering);
    let final_bytes = if tampered {
        write_arrow(&arrow_path, &rows, &changed_value(&rows))
    } else {
        recorded_bytes.clone()
    };
    let hashed_bytes = if tampering == Tampering::ArrowEdited {
        recorded_bytes
    } else {
        final_bytes
    };
    let metadata = ResultMetadata {
        format: 1,
        case_hash: pending.case_hash.clone(),
        outcome: RecordedOutcome::Rows,
        recorder: STAND_IN_RECORDER.to_owned(),
        connector: STAND_IN_RECORDER.to_owned(),
        query_id: None,
        warehouse_version: STAND_IN_RECORDER.to_owned(),
        duckdb_version: pending.duckdb_version.clone(),
        recorded_at: "1970-01-01T00:00:00Z".to_owned(),
        session_parameters: pending.session_parameters.clone(),
        columns: rows
            .schema
            .fields()
            .iter()
            .map(|field| warehouse_column(field.name(), field.data_type()))
            .collect(),
        row_count: Some(
            rows.batches
                .iter()
                .map(RecordBatch::num_rows)
                .sum::<usize>() as u64,
        ),
        arrow_sha256: Some(sha256_hex(&hashed_bytes)),
        error: None,
    };
    std::fs::write(
        results.join(format!("{}.json", pending.case_hash)),
        serde_json::to_string_pretty(&metadata).expect("metadata serialises"),
    )
    .expect("metadata written");
}

pub(crate) struct RoundTrip {
    pub(crate) pending_before: usize,
    pub(crate) pending_after: usize,
    pub(crate) verdicts: Vec<&'static str>,
}

fn verdict_name(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Pending => "pending",
        Verdict::Refused { .. } => "refused",
        Verdict::Passed => "passed",
        Verdict::Failed { .. } => "failed",
    }
}

pub(crate) fn round_trip(tampering: Tampering) -> RoundTrip {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let cases_dir = root.join("cases").join("numeric");
    std::fs::create_dir_all(&cases_dir).expect("cases directory");
    std::fs::write(cases_dir.join("addition.toml"), CASE_FILE).expect("case file");
    let results = root.join("results");
    let pending_dir = root.join("pending");
    std::fs::create_dir_all(&results).expect("results directory");
    std::fs::write(root.join("sqlquack.toml"), POLICY).expect("policy written");
    let policy = load_policy(&root.join("sqlquack.toml")).expect("the test policy is valid");
    let cases = load_cases(&root.join("cases"), &policy).expect("cases load");
    let before = write_pending(&cases, &results, &pending_dir, "v2.0.0").expect("pending written");
    let mut pending: BTreeMap<String, PendingCase> = BTreeMap::new();
    for entry in std::fs::read_dir(&pending_dir)
        .expect("pending listed")
        .flatten()
    {
        let text = std::fs::read_to_string(entry.path()).expect("pending read");
        let case: PendingCase = serde_json::from_str(&text).expect("pending parses");
        pending.insert(case.case_reference.clone(), case);
    }
    for (reference, case) in &pending {
        let applied = if reference.ends_with("#decimal_addition") {
            tampering
        } else {
            Tampering::None
        };
        record_stand_in(case, &results, applied);
    }
    let after = write_pending(&cases, &results, &pending_dir, "v2.0.0").expect("pending rewritten");
    let report =
        verify_cases(&cases, &results, &policy, &UnchangedSqlForTests).expect("verify runs");
    RoundTrip {
        pending_before: before.written,
        pending_after: after.written,
        verdicts: report
            .cases
            .iter()
            .map(|case| verdict_name(&case.verdict))
            .collect(),
    }
}
