use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use arrow::array::{Array, ArrayRef, Decimal128Array, RecordBatch, TimestampNanosecondArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::FileWriter;
use sqlquack_compare::values::models::ResultSet;
use sqlquack_policy::policy::main::load_policy::load_policy;
use sqlquack_policy::policy::models::Policy;

use crate::cases::main::load_cases::load_cases;
use crate::cases::models::{CaseRole, LoadedCase};
use crate::pending::main::write_pending::write_pending;
use crate::pending::models::PendingCase;
use crate::replay::main::replay_on_duckdb::replay_on_duckdb;
use crate::replay::models::ReplaySettings;
use crate::results::_helpers::reading::sha256_hex;
use crate::results::models::{
    RecordedFailure, RecordedOutcome, ResultMetadata, SessionDefaults, WarehouseColumn,
};
use crate::verify::_helpers::compile_errors::undeclared_compile_error;
use crate::verify::_helpers::session::parameter_text;
use crate::verify::main::verify_cases::verify_cases;
use crate::verify::models::VerifyInputs;
use crate::verify::tests::test_types::{RoundTrip, Tampering, UnchangedSqlForTests};
use crate::verify::types::FailureModes;
use sqlquack_compare::comparison::models::RowOrder;

/// Marks every stand-in result: produced by DuckDB in a test, never warehouse evidence.
const STAND_IN_RECORDER: &str = "test fixture: DuckDB stand-in, not evidence";
const STAND_IN_PARAMETERS_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// The case whose stand-in result the tampering functions alter.
pub(crate) const TAMPERED_CASE: &str = "numeric/addition.toml#decimal_addition";

/// The typing case.
pub(crate) const TAMPERED_TYPING: &str = "numeric/addition.toml#typed_sum";

/// The canonical spelling of the equivalence case.
pub(crate) const TAMPERED_CANONICAL: &str = "numeric/remainder.toml#remainder~canonical";

const EQUIVALENCE_FILE: &str = r#"
category = "numeric"
construct = "node:mod_func"
session_profile = "utc-v1"
equivalence = "mod_function_to_operator"

[[case]]
id = "remainder"
description = "MOD and % agree."
columns = [{ name = "a", type = "DECIMAL(10,2)" }, { name = "b", type = "DECIMAL(10,2)" }]
rows = [["7.50", "2.00"], ["NULL", "1.00"]]
expression = "MOD(a, b)"
canonical = "a % b"
tags = ["null"]
"#;

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

[session_profiles.utc-v1.pinned]
DATE_OUTPUT_FORMAT = "YYYY-MM-DD"

[divergences.timestamp_range]
constructs = ["node:cast"]
default = "warn"
strict = "refuse"
reason = "DuckDB nanosecond timestamps span a shorter range."
failure = "loud"
native_errors = ["Conversion Error: Could not convert string '*' to INT64*"]

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

[[case]]
id = "typed_sum"
description = "The type of a decimal sum, recorded without values."
columns = [{ name = "a", type = "DECIMAL(10,2)" }]
rows = [["1.25"], ["NULL"]]
expression = "a + a"
tags = ["null"]
evidence = "typing"
values = false
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
        sub_microsecond_checked: false,
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

/// Drops the values: removes the Arrow file and records a types outcome instead.
pub(crate) fn values_dropped(results: &Path, hash: &str) {
    std::fs::remove_file(results.join(format!("{hash}.arrow"))).expect("arrow file removed");
    let metadata_path = results.join(format!("{hash}.json"));
    let mut metadata: ResultMetadata =
        serde_json::from_str(&std::fs::read_to_string(&metadata_path).expect("metadata read"))
            .expect("metadata parses");
    metadata.outcome = RecordedOutcome::Types;
    metadata.arrow_sha256 = None;
    std::fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&metadata).expect("metadata serialises"),
    )
    .expect("metadata written");
}

/// Records a warehouse error in place of the result.
pub(crate) fn errored(results: &Path, hash: &str) {
    let metadata_path = results.join(format!("{hash}.json"));
    let mut metadata: ResultMetadata =
        serde_json::from_str(&std::fs::read_to_string(&metadata_path).expect("metadata read"))
            .expect("metadata parses");
    metadata.outcome = RecordedOutcome::Error;
    metadata.columns = Vec::new();
    metadata.row_count = None;
    metadata.error = Some(RecordedFailure {
        errno: Some(100_071),
        sqlstate: Some("22000".to_owned()),
        message: None,
    });
    std::fs::write(
        &metadata_path,
        serde_json::to_string_pretty(&metadata).expect("metadata serialises"),
    )
    .expect("metadata written");
}

/// Records a pinned parameter with another effective value than the profile pins.
pub(crate) fn pinned_value_changed(results: &Path, hash: &str) {
    edit_metadata(results, hash, |metadata| {
        metadata
            .effective_parameters
            .get_or_insert_with(BTreeMap::new)
            .insert(
                "DATE_OUTPUT_FORMAT".to_owned(),
                Some("DD/MM/YYYY".to_owned()),
            );
    });
}

/// Drops the effective parameters, as in a result recorded before pinning.
pub(crate) fn recorded_before_pinning(results: &Path, hash: &str) {
    edit_metadata(results, hash, |metadata| {
        metadata.effective_parameters = None;
        metadata.parameters_sha256 = None;
    });
}

/// Account defaults with `DATE_OUTPUT_FORMAT` set to `value`.
pub(crate) fn defaults_with_date_format(value: &str) -> SessionDefaults {
    SessionDefaults {
        format: 1,
        recorder: STAND_IN_RECORDER.to_owned(),
        recorded_at: "1970-01-01T00:00:00Z".to_owned(),
        parameters: BTreeMap::from([("DATE_OUTPUT_FORMAT".to_owned(), value.to_owned())]),
        parameters_sha256: STAND_IN_PARAMETERS_SHA256.to_owned(),
    }
}

fn edit_metadata(results: &Path, hash: &str, change: impl Fn(&mut ResultMetadata)) {
    let metadata_path = results.join(format!("{hash}.json"));
    let mut metadata: ResultMetadata =
        serde_json::from_str(&std::fs::read_to_string(&metadata_path).expect("metadata read"))
            .expect("metadata parses");
    change(&mut metadata);
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
    let rows = replay_on_duckdb((&[], &pending.source_sql), None, &settings)
        .expect("DuckDB runs the case");
    let keeps_values = pending.values != Some(false);
    let arrow_path = results.join(format!("{}.arrow", pending.case_hash));
    let arrow_sha256 = keeps_values.then(|| sha256_hex(&write_arrow(&arrow_path, &rows)));
    let metadata = ResultMetadata {
        format: 1,
        case_hash: pending.case_hash.clone(),
        outcome: [RecordedOutcome::Types, RecordedOutcome::Rows][usize::from(keeps_values)],
        recorder: STAND_IN_RECORDER.to_owned(),
        connector: STAND_IN_RECORDER.to_owned(),
        query_id_sha256: None,
        warehouse_version: STAND_IN_RECORDER.to_owned(),
        duckdb_version: pending.duckdb_version.clone(),
        recorded_at: "1970-01-01T00:00:00Z".to_owned(),
        session_parameters: pending.session_parameters.clone(),
        effective_parameters: Some(
            pending
                .session_parameters
                .iter()
                .chain(&pending.pinned_parameters)
                .map(|(key, value)| (key.clone(), Some(parameter_text(value))))
                .collect(),
        ),
        parameters_sha256: Some(STAND_IN_PARAMETERS_SHA256.to_owned()),
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
        arrow_sha256,
        error: None,
    };
    std::fs::write(
        results.join(format!("{}.json", pending.case_hash)),
        serde_json::to_string_pretty(&metadata).expect("metadata serialises"),
    )
    .expect("metadata written");
}

/// Writes pending files, records DuckDB stand-ins for them, tampers with one, and verifies.
pub(crate) fn round_trip(tampering: Tampering, tampered: &str) -> RoundTrip {
    round_trip_with_defaults((tampering, tampered), None)
}

/// As `round_trip`, with the account's session defaults recorded as `defaults`.
pub(crate) fn round_trip_with_defaults(
    (tampering, tampered): (Tampering, &str),
    defaults: Option<SessionDefaults>,
) -> RoundTrip {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    let cases_dir = root.join("cases").join("numeric");
    std::fs::create_dir_all(&cases_dir).expect("cases directory");
    std::fs::write(cases_dir.join("addition.toml"), CASE_FILE).expect("case file");
    std::fs::write(cases_dir.join("remainder.toml"), EQUIVALENCE_FILE).expect("case file");
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
    tampering(&results, &pending[tampered].case_hash);
    let after = write_pending(&cases, &results, &pending_dir, "v2.0.0").expect("pending rewritten");
    let report = verify_cases(VerifyInputs {
        cases: &cases,
        results: &results,
        policy: &policy,
        converter: &UnchangedSqlForTests,
        claims: &[],
        failure_modes: &FailureModes::new(),
        audit_results: None,
        session_defaults: defaults.as_ref(),
    })
    .expect("verify runs");
    RoundTrip {
        pending_before: before.written,
        pending_after: after.written,
        unproven_sessions: report.unproven_sessions,
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

/// The test policy.
pub(crate) fn test_policy() -> Policy {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("sqlquack.toml");
    std::fs::write(&path, POLICY).expect("policy written");
    load_policy(&path).expect("the test policy is valid")
}

/// The repository's own policy.
pub(crate) fn repository_policy() -> Policy {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sqlquack.toml");
    load_policy(&path).expect("the repository policy is valid")
}

/// One nullable column `T` of `data_type` holding `array`.
pub(crate) fn result_set(data_type: DataType, array: ArrayRef) -> ResultSet {
    let schema = Arc::new(Schema::new(vec![Field::new("T", data_type, true)]));
    let batch = RecordBatch::try_new(Arc::clone(&schema), vec![array]).expect("batch");
    ResultSet {
        schema,
        batches: vec![batch],
    }
}

/// The first nanosecond timestamp of `rows`.
pub(crate) fn floored_nanos(rows: &ResultSet) -> i64 {
    rows.batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<TimestampNanosecondArray>()
        .expect("nanosecond timestamps")
        .value(0)
}

/// Whether a recorded `sqlstate` error fails a case declaring it or not, with setup or not.
pub(crate) fn undeclared_compile_error_for(
    sqlstate: &str,
    declared: bool,
    with_setup: bool,
) -> bool {
    let case = LoadedCase {
        reference: "orders.toml#order_total".to_owned(),
        description: String::new(),
        category: String::new(),
        construct: String::new(),
        session_profile: String::new(),
        session_parameters: BTreeMap::new(),
        pinned_parameters: BTreeMap::new(),
        source_sql: String::new(),
        order: RowOrder::Ordered,
        hash: String::new(),
        role: CaseRole::Conversion,
        expected_divergence: None,
        values: true,
        compile_error: declared,
        functions: Vec::new(),
        overrides: Vec::new(),
        setup_sql: [String::from(
            "CREATE FUNCTION order_total() RETURNS NUMBER AS $$ 1 $$",
        )]
        .into_iter()
        .take(usize::from(with_setup))
        .collect(),
        inputs: Vec::new(),
        row_sources: Vec::new(),
    };
    let recorded = RecordedFailure {
        errno: Some(1003),
        sqlstate: Some(sqlstate.to_owned()),
        message: None,
    };
    undeclared_compile_error(&case, Some(&recorded)).is_some()
}
