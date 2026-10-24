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
use crate::verify::tests::test_types::{RoundTrip, Tampering, UnchangedSqlForTests};

/// Marks every stand-in result: produced by DuckDB in a test, never warehouse evidence.
const STAND_IN_RECORDER: &str = "test fixture: DuckDB stand-in, not evidence";

/// The case whose stand-in result the tampering functions alter.
const TAMPERED_CASE: &str = "numeric/addition.toml#decimal_addition";

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
columns = [{ name = "amount", type = "DECIMAL(10,2)" }]
rows = [["3.00"], ["1.00"]]
query = "SELECT amount FROM input"
order = "bag"
tags = []
"#;

/// How DuckDB result types of the fixture cases are described as warehouse columns.
fn fixture_column_types() -> [(DataType, (&'static str, i64, i64)); 2] {
    [
        (DataType::Decimal128(10, 2), ("FIXED", 10, 2)),
        (DataType::Decimal128(11, 2), ("FIXED", 11, 2)),
    ]
}

fn warehouse_column(name: &str, data_type: &DataType) -> WarehouseColumn {
    let (_, (type_name, precision, scale)) = fixture_column_types()
        .into_iter()
        .find(|(fixture_type, _)| fixture_type == data_type)
        .expect("every fixture result type is described");
    WarehouseColumn {
        name: name.to_ascii_uppercase(),
        type_code: 0,
        type_name: type_name.to_owned(),
        precision: Some(precision),
        scale: Some(scale),
        internal_size: None,
        nullable: Some(true),
    }
}

fn read_arrow(path: &Path) -> ResultSet {
    let reader =
        arrow::ipc::reader::FileReader::try_new(File::open(path).expect("arrow file opened"), None)
            .expect("arrow file reads");
    let schema = reader.schema();
    let batches = reader.collect::<Result<Vec<_>, _>>().expect("batches read");
    ResultSet { schema, batches }
}

fn write_arrow(path: &Path, rows: &ResultSet) -> Vec<u8> {
    let file = File::create(path).expect("arrow file created");
    let mut writer = FileWriter::try_new(file, &rows.schema).expect("arrow writer");
    for batch in &rows.batches {
        writer.write(batch).expect("batch written");
    }
    writer.finish().expect("arrow file finished");
    std::fs::read(path).expect("arrow file read back")
}

/// Sets every value of the last column to 0.01 and rewrites the Arrow file; returns its bytes.
fn rewrite_last_column(results: &Path, hash: &str) -> Vec<u8> {
    let path = results.join(format!("{hash}.arrow"));
    let rows = read_arrow(&path);
    let batch = &rows.batches[0];
    let last = batch.num_columns() - 1;
    let changed: ArrayRef = Arc::new(
        Decimal128Array::from(vec![Some(1); batch.num_rows()])
            .with_data_type(batch.column(last).data_type().clone()),
    );
    let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
    columns[last] = changed;
    let changed_rows = ResultSet {
        schema: rows.schema.clone(),
        batches: vec![RecordBatch::try_new(batch.schema(), columns).expect("same schema")],
    };
    write_arrow(&path, &changed_rows)
}

/// Leaves the result as recorded.
pub(crate) fn untouched(_: &Path, _: &str) {}

/// Changes a value and updates the metadata's Arrow hash to match.
pub(crate) fn value_changed(results: &Path, hash: &str) {
    let bytes = rewrite_last_column(results, hash);
    let metadata_path = results.join(format!("{hash}.json"));
    let mut metadata: ResultMetadata =
        serde_json::from_str(&std::fs::read_to_string(&metadata_path).expect("metadata read"))
            .expect("metadata parses");
    metadata.arrow_sha256 = Some(sha256_hex(&bytes));
    std::fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&metadata).expect("metadata serialises"),
    )
    .expect("metadata written");
}

/// Changes a value without touching the metadata.
pub(crate) fn arrow_edited(results: &Path, hash: &str) {
    rewrite_last_column(results, hash);
}

fn record_stand_in(pending: &PendingCase, results: &Path) {
    let settings = ReplaySettings {
        time_zone: "UTC".to_owned(),
        null_order: "nulls_last_on_asc_first_on_desc".to_owned(),
    };
    let rows = replay_on_duckdb(&pending.source_sql, &settings).expect("DuckDB runs the case");
    let bytes = write_arrow(&results.join(format!("{}.arrow", pending.case_hash)), &rows);
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
        arrow_sha256: Some(sha256_hex(&bytes)),
        error: None,
    };
    std::fs::write(
        results.join(format!("{}.json", pending.case_hash)),
        serde_json::to_string_pretty(&metadata).expect("metadata serialises"),
    )
    .expect("metadata written");
}

/// Writes pending files, records DuckDB stand-ins for them, tampers with one, and verifies.
pub(crate) fn round_trip(tampering: Tampering) -> RoundTrip {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let cases_dir = root.join("cases").join("numeric");
    std::fs::create_dir_all(&cases_dir).expect("cases directory");
    std::fs::write(cases_dir.join("addition.toml"), CASE_FILE).expect("case file");
    std::fs::write(root.join("sqlquack.toml"), POLICY).expect("policy written");
    let results = root.join("results");
    let pending_dir = root.join("pending");
    std::fs::create_dir_all(&results).expect("results directory");
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
    for case in pending.values() {
        record_stand_in(case, &results);
    }
    tampering(&results, &pending[TAMPERED_CASE].case_hash);
    let after = write_pending(&cases, &results, &pending_dir, "v2.0.0").expect("pending rewritten");
    let report =
        verify_cases(&cases, &results, &policy, &UnchangedSqlForTests).expect("verify runs");
    RoundTrip {
        pending_before: before.written,
        pending_after: after.written,
        verdicts: report
            .cases
            .iter()
            .map(|case| {
                serde_json::to_value(&case.verdict).expect("verdict serialises")["verdict"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect(),
    }
}
