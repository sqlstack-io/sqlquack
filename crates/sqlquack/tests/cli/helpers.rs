use std::path::{Path, PathBuf};
use std::process::Command;

use sqlquack_evidence::replay::main::replay_on_duckdb::replay_on_duckdb;
use sqlquack_evidence::replay::models::{PythonFunction, PythonReplay, ReplaySettings};

use walkdir::WalkDir;

use crate::test_types::{
    CommandOutcome, ConvertOutcome, CoverageOutcome, DivergenceOutcome, ErrorCheckOutcome,
    FunctionMacroOutcome, ReviewOutcome, ReviewedRepository,
};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn copy_files(from: &Path, to: &Path) {
    for entry in WalkDir::new(from)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
    {
        let target = to.join(entry.path().strip_prefix(from).expect("inside the tree"));
        std::fs::create_dir_all(target.parent().expect("a parent directory"))
            .expect("directory created");
        std::fs::copy(entry.path(), &target).expect("file copied");
    }
}

/// Runs a command in a copy of the repository's policy and cases, with no recorded results.
pub(crate) fn run_on_repository_cases(arguments: &[&str], mention: &str) -> CommandOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = repository_root();
    std::fs::copy(
        root.join("sqlquack.toml"),
        directory.path().join("sqlquack.toml"),
    )
    .expect("policy copied");
    copy_files(&root.join("cases"), &directory.path().join("cases"));
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(arguments)
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    let pending = directory.path().join("evidence/pending");
    CommandOutcome {
        exit_code: output.status.code(),
        wrote_pending_files: std::fs::read_dir(pending)
            .is_ok_and(|mut entries| entries.next().is_some()),
        stderr_mentions: String::from_utf8_lossy(&output.stderr).contains(mention),
    }
}

/// The column names DuckDB gives the statement `sqlquack convert` prints for `sql` with `flags`.
pub(crate) fn converted_column_names(sql: &str, flags: &[&str]) -> Vec<String> {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .arg("convert")
        .arg("--policy")
        .arg(repository_root().join("sqlquack.toml"))
        .args(flags)
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let converted = String::from_utf8_lossy(&output.stdout);
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    let mut statement = connection
        .prepare(converted.trim())
        .expect("the converted statement prepares");
    let _ = statement.query([]).expect("the converted statement runs");
    statement.column_names()
}

/// Converts `sql`, runs it on DuckDB in UTC, and returns DuckDB's error if running fails.
pub(crate) fn converted_run_error(sql: &str) -> Option<String> {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .arg("convert")
        .arg("--policy")
        .arg(repository_root().join("sqlquack.toml"))
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let converted = String::from_utf8_lossy(&output.stdout);
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch("SET TimeZone = 'UTC'")
        .expect("UTC session");
    connection
        .prepare(converted.trim())
        .and_then(|mut statement| {
            statement
                .query_map([], |_| Ok(()))
                .and_then(|rows| rows.collect::<Result<Vec<()>, _>>())
        })
        .err()
        .map(|error| error.to_string())
}

/// The first column of the first row the converted `sql` returns on DuckDB, as text.
pub(crate) fn converted_first_value(sql: &str) -> String {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .arg("convert")
        .arg("--policy")
        .arg(repository_root().join("sqlquack.toml"))
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let converted = String::from_utf8_lossy(&output.stdout);
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .query_row(converted.trim(), [], |row| row.get::<_, String>(0))
        .expect("the converted statement returns a text value")
}

/// The first value the converted `sql` returns over `schema`, its tables made by `setup` on DuckDB.
pub(crate) fn converted_value_over(sql: &str, (schema, setup): (&str, &str)) -> String {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    std::fs::write(directory.path().join("schema.json"), schema).expect("schema written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .arg("convert")
        .arg("--policy")
        .arg(repository_root().join("sqlquack.toml"))
        .arg("--schema")
        .arg(directory.path().join("schema.json"))
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let converted = String::from_utf8_lossy(&output.stdout);
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection.execute_batch(setup).expect("tables made");
    connection
        .query_row(converted.trim(), [], |row| row.get::<_, String>(0))
        .expect("the converted statement returns a text value")
}

/// The exit code and standard error of converting `sql` with the override `definition`.
pub(crate) fn override_failure(sql: &str, definition: &str) -> (Option<i32>, String) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let overrides = directory.path().join("overrides");
    std::fs::create_dir(&overrides).expect("overrides directory");
    std::fs::write(overrides.join("reversed_text.sql"), definition).expect("override written");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--policy"])
        .arg(repository_root().join("sqlquack.toml"))
        .arg("--overrides")
        .arg(&overrides)
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// `sql` converted with the override `definition`: the report, and its first value over three orders.
pub(crate) fn override_outcome(
    sql: &str,
    definition: &str,
    flags: &[&str],
) -> (serde_json::Value, Option<String>) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let overrides = directory.path().join("overrides");
    std::fs::create_dir(&overrides).expect("overrides directory");
    std::fs::write(overrides.join("reversed_text.sql"), definition).expect("override written");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    std::fs::write(
        directory.path().join("schema.json"),
        r#"{"relations": {"ORDERS": [["ORDER_ID", "NUMBER(10,0)"], ["SKU", "VARCHAR"]]}}"#,
    )
    .expect("schema written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--json", "--policy"])
        .arg(repository_root().join("sqlquack.toml"))
        .arg("--schema")
        .arg(directory.path().join("schema.json"))
        .arg("--overrides")
        .arg(&overrides)
        .args(flags)
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("a JSON report");
    let value = (report["converted"] == serde_json::Value::Bool(true))
        .then(|| converted_value_with_macros(&report));
    (report, value)
}

/// The first value `report`'s SQL returns over three orders, once its macros run on DuckDB.
fn converted_value_with_macros(report: &serde_json::Value) -> String {
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch(
            "CREATE TABLE \"ORDERS\" (\"ORDER_ID\" DECIMAL(10,0), \"SKU\" VARCHAR); \
             INSERT INTO \"ORDERS\" VALUES (1, 'ab-1'), (2, 'cd-2'), (3, 'ef-3')",
        )
        .expect("orders made");
    for statement in report["macros"].as_array().expect("macros listed") {
        connection
            .execute_batch(statement.as_str().unwrap_or_default())
            .expect("the override's macro is made");
    }
    connection
        .query_row(report["sql"].as_str().unwrap_or_default(), [], |row| {
            row.get::<_, String>(0)
        })
        .expect("the converted statement returns a text value")
}

/// The exit code of `arguments` run where no policy file is, over a one-statement file.
pub(crate) fn policy_free_exit_code(arguments: &[&str]) -> Option<i32> {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), "SELECT 1 AS total").expect("query written");
    std::fs::write(
        directory.path().join("schema.json"),
        "{\"functions\": {\"TOTAL\": {\"language\": \"sql\", \"arguments\": [], \"returns\": \"NUMBER(1,0)\"}}}",
    )
    .expect("schema written");
    Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(arguments)
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs")
        .status
        .code()
}

const PYTHON_SCHEMA: &str = "crates/sqlquack/tests/cli/fixtures/python_functions.json";
const PYTHON_HANDLER: &str = "handle";

const OVERFLOW_CASE: &str = r#"category = "aggregate"
construct = "node:sum"
session_profile = "utc-v1"

[[case]]
id = "sum_overflow"
description = "SUM of integers beyond 38 digits."
columns = [{ name = "x", type = "NUMBER(38,0)" }]
rows = [
  ["99999999999999999999999999999999999999"],
  ["1"],
]
query = "SELECT SUM(x) AS s FROM input"
order = "ordered"
tags = ["duplicates", "null", "empty-input"]
"#;

/// The numeric_range constructs and default mode as sqlquack.toml declares them.
const NUMERIC_RANGE_SECTION: &str = "[divergences.numeric_range]";
const WARN_DEFAULT: &str = "default = \"warn\"";

/// Verifies the recorded SUM overflow case with a marker line and the divergence's default mode.
pub(crate) fn verify_overflow_case(case_marker: &str, policy_default: &str) -> DivergenceOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = repository_root();
    let policy = std::fs::read_to_string(root.join("sqlquack.toml")).expect("policy read");
    let (before, section) = policy
        .split_once(NUMERIC_RANGE_SECTION)
        .expect("the policy declares numeric_range");
    let section = section.replacen(WARN_DEFAULT, &format!("default = \"{policy_default}\""), 1);
    let policy = format!("{before}{NUMERIC_RANGE_SECTION}{section}");
    std::fs::write(directory.path().join("sqlquack.toml"), policy).expect("policy written");
    copy_files(
        &root.join("cases/fidelity"),
        &directory.path().join("cases/fidelity"),
    );
    let cases = directory.path().join("cases/aggregate");
    std::fs::create_dir_all(&cases).expect("cases directory");
    std::fs::write(
        cases.join("overflow.toml"),
        format!("{OVERFLOW_CASE}{case_marker}"),
    )
    .expect("case written");
    copy_files(
        &root.join("evidence/results"),
        &directory.path().join("evidence/results"),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["verify", "--json"])
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    DivergenceOutcome {
        exit_code: output.status.code(),
        verdict: report["cases"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|case| case["reference"] == "aggregate/overflow.toml#sum_overflow")
            .and_then(|case| case["verdict"].as_str())
            .unwrap_or_default()
            .to_owned(),
    }
}

const TEXT_TOO_LARGE: &str = "conversion/to_number.toml#text_too_large";

fn verify_json(directory: &Path, arguments: &[&str]) -> (Option<i32>, serde_json::Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["verify", "--json"])
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("sqlquack runs");
    (
        output.status.code(),
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null),
    )
}

fn case_report<'a>(report: &'a serde_json::Value, reference: &str) -> &'a serde_json::Value {
    report["cases"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|case| case["reference"] == reference)
        .unwrap_or(&serde_json::Value::Null)
}

fn rewrite_error(path: &Path, (errno, sqlstate): (i64, &str)) {
    let mut metadata: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("result read"))
            .expect("result parses");
    metadata["error"]["errno"] = serde_json::Value::from(errno);
    metadata["error"]["sqlstate"] = serde_json::Value::from(sqlstate);
    std::fs::write(path, metadata.to_string()).expect("result written");
}

/// Writes a re-recording of the stored result at `path` whose error carries `message`.
fn write_audited(directory: &Path, (path, hash): (&Path, &str), message: &str) {
    let audited = directory.join("evidence/audit/results");
    std::fs::create_dir_all(&audited).expect("audit results directory");
    let mut metadata: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("result read"))
            .expect("result parses");
    metadata["error"]["message"] = serde_json::Value::from(message);
    std::fs::write(audited.join(format!("{hash}.json")), metadata.to_string())
        .expect("audit result written");
}

/// Verifies the recorded text-to-NUMBER overflow, its Snowflake error replaced when given.
pub(crate) fn verify_error_check(
    (recorded_error, audited_message): (Option<(i64, &str)>, Option<&str>),
    arguments: &[&str],
) -> ErrorCheckOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = repository_root();
    std::fs::copy(
        root.join("sqlquack.toml"),
        directory.path().join("sqlquack.toml"),
    )
    .expect("policy copied");
    copy_files(
        &root.join("cases/fidelity"),
        &directory.path().join("cases/fidelity"),
    );
    let cases = directory.path().join("cases/conversion");
    std::fs::create_dir_all(&cases).expect("cases directory");
    std::fs::copy(
        root.join("cases/conversion/to_number.toml"),
        cases.join("to_number.toml"),
    )
    .expect("case copied");
    copy_files(
        &root.join("evidence/results"),
        &directory.path().join("evidence/results"),
    );
    let (_, report) = verify_json(directory.path(), &[]);
    let hash = case_report(&report, TEXT_TOO_LARGE)["hash"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let path = directory
        .path()
        .join(format!("evidence/results/{hash}.json"));
    recorded_error
        .iter()
        .for_each(|error| rewrite_error(&path, *error));
    audited_message
        .iter()
        .for_each(|message| write_audited(directory.path(), (&path, &hash), message));
    let (exit_code, report) = verify_json(directory.path(), arguments);
    let case = case_report(&report, TEXT_TOO_LARGE);
    ErrorCheckOutcome {
        exit_code,
        verdict: case["verdict"].as_str().unwrap_or_default().to_owned(),
        error_check: case["error_check"]["outcome"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        snowflake_message: case["snowflake_message"].as_str().map(str::to_owned),
    }
}

/// Runs `sqlquack review` once on a copy of the policy, cases and recorded results.
pub(crate) fn review_repository() -> ReviewedRepository {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = repository_root();
    std::fs::copy(
        root.join("sqlquack.toml"),
        directory.path().join("sqlquack.toml"),
    )
    .expect("policy copied");
    copy_files(&root.join("cases"), &directory.path().join("cases"));
    copy_files(
        &root.join("evidence/results"),
        &directory.path().join("evidence/results"),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["review", "--out", "exported"])
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    ReviewedRepository {
        exit_code: output.status.code(),
        directory,
    }
}

/// The review's exit code and the expected lines missing from one exported file.
pub(crate) fn review_export(
    review: &ReviewedRepository,
    file: &str,
    expected_lines: &[&str],
) -> ReviewOutcome {
    let text = std::fs::read_to_string(review.directory.path().join("exported").join(file))
        .unwrap_or_default();
    ReviewOutcome {
        exit_code: review.exit_code,
        missing_lines: expected_lines
            .iter()
            .filter(|expected| !text.lines().any(|line| line == **expected))
            .map(|expected| (*expected).to_owned())
            .collect(),
    }
}

/// Reviews the project function cases, with the equivalence files every review needs.
pub(crate) fn review_project_functions(file: &str, expected_lines: &[&str]) -> ReviewOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = repository_root();
    std::fs::copy(
        root.join("sqlquack.toml"),
        directory.path().join("sqlquack.toml"),
    )
    .expect("policy copied");
    copy_files(
        &root.join("cases/project-functions"),
        &directory.path().join("cases/project-functions"),
    );
    for entry in WalkDir::new(root.join("cases"))
        .into_iter()
        .flatten()
        .filter(|entry| {
            std::fs::read_to_string(entry.path())
                .is_ok_and(|text| text.lines().any(|line| line.starts_with("equivalence = ")))
        })
    {
        let relative = entry
            .path()
            .strip_prefix(&root)
            .expect("inside the repository");
        let target = directory.path().join(relative);
        std::fs::create_dir_all(target.parent().expect("a parent directory"))
            .expect("directory created");
        std::fs::copy(entry.path(), target).expect("file copied");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["review", "--out", "exported"])
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    let text =
        std::fs::read_to_string(directory.path().join("exported").join(file)).unwrap_or_default();
    ReviewOutcome {
        exit_code: output.status.code(),
        missing_lines: expected_lines
            .iter()
            .filter(|expected| !text.lines().any(|line| line.starts_with(*expected)))
            .map(|expected| (*expected).to_owned())
            .collect(),
    }
}

/// Converts `DOUBLED(PRICE NUMBER(10,2))` with `sqlquack function`, then runs its macro.
pub(crate) fn run_function_macro(
    (returns, body): (&str, &str),
    call: &str,
    flags: &[&str],
) -> FunctionMacroOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let schema = format!(
        "{{\"functions\": {{\"DOUBLED\": {{\"language\": \"sql\", \
         \"arguments\": [[\"PRICE\", \"NUMBER(10,2)\"]], \"returns\": {returns}}}}}}}"
    );
    std::fs::write(directory.path().join("schema.json"), schema).expect("schema written");
    std::fs::write(directory.path().join("doubled.sql"), body).expect("body written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args([
            "function",
            "doubled.sql",
            "--name",
            "DOUBLED",
            "--schema",
            "schema.json",
        ])
        .arg("--policy")
        .arg(repository_root().join("sqlquack.toml"))
        .args(flags)
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    let converted: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    let values = converted["body"]
        .as_str()
        .map(|macro_body| macro_values(&converted, macro_body, call))
        .unwrap_or_default();
    FunctionMacroOutcome {
        exit_code: output.status.code(),
        values,
    }
}

fn macro_values(converted: &serde_json::Value, macro_body: &str, call: &str) -> Vec<String> {
    let parameters: Vec<&str> = converted["parameters"]
        .as_array()
        .map(|parameters| {
            parameters
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect()
        })
        .unwrap_or_default();
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch(&format!(
            "CREATE MACRO \"DOUBLED\"({}) AS {macro_body}",
            parameters.join(", ")
        ))
        .expect("the macro is created");
    let mut statement = connection.prepare(call).expect("the call prepares");
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("the call runs")
        .map(|value| value.expect("a value"))
        .collect()
}

const COVERAGE_RULE: &str = "rule.variant_computed_get";

const COVERAGE_HAZARDS: &str = r#"version = 4

[families.text]
sql_type = "VARCHAR"
benign = "'a'"
other = "'b'"

[[families.text.hazard]]
id = "recorded_key"
description = "A key the recorded case holds."
cells = ["'a'"]

[[families.text.hazard]]
id = "unrecorded_key"
description = "A key no recorded case holds."
cells = ["'zz'"]
"#;

/// Verifies the recorded GET cases against a GET domain with `key_families`; reports its gaps.
pub(crate) fn verify_coverage(key_families: &str, arguments: &[&str]) -> CoverageOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = repository_root();
    std::fs::copy(
        root.join("sqlquack.toml"),
        directory.path().join("sqlquack.toml"),
    )
    .expect("policy copied");
    copy_files(
        &root.join("cases/fidelity"),
        &directory.path().join("cases/fidelity"),
    );
    let cases = directory.path().join("cases/semi-structured");
    std::fs::create_dir_all(&cases).expect("cases directory");
    std::fs::copy(
        root.join("cases/semi-structured/arrays_batch2.toml"),
        cases.join("arrays_batch2.toml"),
    )
    .expect("case copied");
    copy_files(
        &root.join("evidence/results"),
        &directory.path().join("evidence/results"),
    );
    let domains = directory.path().join("domains");
    std::fs::create_dir_all(&domains).expect("domains directory");
    std::fs::write(
        domains.join("get.toml"),
        format!(
            "version = 1\n\n[[rule]]\nid = \"{COVERAGE_RULE}\"\n\n[[rule.form]]\nconstruct = \
             \"fn:GET\"\narguments = [{{ families = [\"variant\"] }}, {{ families = \
             {key_families}, kinds = [\"value\"] }}]\n"
        ),
    )
    .expect("domain written");
    std::fs::write(directory.path().join("hazards.toml"), COVERAGE_HAZARDS)
        .expect("hazards written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args([
            "verify",
            "--domains",
            "domains",
            "--hazards",
            "hazards.toml",
            "--coverage-json",
            "coverage.json",
        ])
        .args(arguments)
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(directory.path().join("coverage.json")).unwrap_or_default(),
    )
    .unwrap_or(serde_json::Value::Null);
    let rule = report["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|rule| rule["rule"] == COVERAGE_RULE)
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let strings = |key: &str| -> Vec<String> {
        rule[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|gap| gap.as_str().map(str::to_owned))
            .collect()
    };
    CoverageOutcome {
        exit_code: output.status.code(),
        shape_gaps: strings("shape_gaps"),
        hazard_gaps: strings("hazard_gaps"),
    }
}

const COMPILE_ERROR_CASES: &str = r#"category = "relational"
construct = "node:select"
session_profile = "utc-v1"

[[case]]
id = "reserved_alias"
description = "An alias Snowflake reserves, which it refuses to compile."
columns = [{ name = "quantity", type = "NUMBER(38,0)" }]
rows = [["1"]]
query = "SELECT quantity AS order FROM input"
order = "ordered"
tags = ["null", "duplicates", "empty-input"]
compile_error = true

[[case]]
id = "plain_alias"
description = "An ordinary alias."
columns = [{ name = "quantity", type = "NUMBER(38,0)" }]
rows = [["1"]]
query = "SELECT quantity AS order_quantity FROM input"
order = "ordered"
tags = ["null", "duplicates", "empty-input"]
"#;

/// Each pending file's case reference and `compile_error`, for one case declaring it.
pub(crate) fn pending_compile_errors() -> Vec<(String, Option<bool>)> {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::copy(
        repository_root().join("sqlquack.toml"),
        directory.path().join("sqlquack.toml"),
    )
    .expect("policy copied");
    let cases = directory.path().join("cases/relational");
    std::fs::create_dir_all(&cases).expect("cases directory");
    std::fs::write(cases.join("aliases.toml"), COMPILE_ERROR_CASES).expect("cases written");
    let status = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .arg("pending")
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs")
        .status;
    assert!(status.success(), "pending succeeds");
    let mut flags: Vec<(String, Option<bool>)> =
        std::fs::read_dir(directory.path().join("evidence/pending"))
            .expect("pending files")
            .flatten()
            .map(|entry| {
                let text = std::fs::read_to_string(entry.path()).expect("pending file read");
                let pending: serde_json::Value =
                    serde_json::from_str(&text).expect("pending file parses");
                (
                    pending["case_reference"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                    pending["compile_error"].as_bool(),
                )
            })
            .collect();
    flags.sort();
    flags
}

/// Runs `sqlquack convert` on `sql` from standard input with `flags` and the repository policy.
pub(crate) fn run_convert(sql: &str, flags: &[&str], mention: &str) -> ConvertOutcome {
    use std::io::Write;
    let policy = repository_root().join("sqlquack.toml");
    let builtin = flags.contains(&"--builtin-policy");
    let policy_flags: Vec<&std::ffi::OsStr> =
        [std::ffi::OsStr::new("--policy"), policy.as_os_str()]
            .into_iter()
            .filter(|_| !builtin)
            .collect();
    let mut child = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .arg("convert")
        .args(policy_flags)
        .args(flags)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("sqlquack runs");
    child
        .stdin
        .take()
        .expect("standard input")
        .write_all(sql.as_bytes())
        .expect("SQL written");
    let output = child.wait_with_output().expect("sqlquack finishes");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();
    ConvertOutcome {
        exit_code: output.status.code(),
        printed_sql: stdout.starts_with("SELECT") || stdout.starts_with("WITH"),
        json_converted: json["converted"].as_bool(),
        stderr_mentions: String::from_utf8_lossy(&output.stderr).contains(mention),
    }
}

/// The `convert --json` report of `sql` over the Python functions fixture, with `flags`.
pub(crate) fn python_report(sql: &str, flags: &[&str]) -> serde_json::Value {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--json", "--policy"])
        .arg(repository_root().join("sqlquack.toml"))
        .arg("--schema")
        .arg(repository_root().join(PYTHON_SCHEMA))
        .args(flags)
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    serde_json::from_slice(&output.stdout).expect("a JSON report")
}

/// The rows of `report`'s SQL run on DuckDB, or its error, its Python functions registered by the runtime.
pub(crate) fn python_outcome(
    report: &serde_json::Value,
    handlers: &[(&str, &str)],
) -> Result<Vec<Vec<String>>, String> {
    let functions: Vec<PythonFunction> = report["python_functions"]
        .as_array()
        .expect("the functions listed")
        .iter()
        .map(|listed| {
            let name = listed["name"].as_str().unwrap_or_default();
            let source = handlers
                .iter()
                .find(|(handler, _)| *handler == name)
                .map(|(_, source)| (*source).to_owned())
                .expect("a handler for each listed function");
            PythonFunction {
                name: name.to_owned(),
                abi: listed["abi"].as_str().unwrap_or_default().to_owned(),
                arguments: serde_json::from_value(listed["arguments"].clone()).unwrap_or_default(),
                returns: listed["returns"].as_str().unwrap_or_default().to_owned(),
                volatility: listed["volatility"].as_str().unwrap_or_default().to_owned(),
                null_input: listed["null_input"].as_str().unwrap_or_default().to_owned(),
                handler: PYTHON_HANDLER.to_owned(),
                source,
            }
        })
        .collect();
    let replay = PythonReplay {
        interpreter: repository_root().join(".venv/bin/python"),
        functions,
    };
    let settings = ReplaySettings {
        time_zone: "UTC".to_owned(),
        null_order: "nulls_last".to_owned(),
    };
    let rows = replay_on_duckdb(
        (&[], report["sql"].as_str().unwrap_or_default()),
        Some(&replay),
        &settings,
    )?;
    Ok(rows
        .batches
        .iter()
        .flat_map(|batch| {
            (0..batch.num_rows()).map(move |row| {
                (0..batch.num_columns())
                    .map(|column| {
                        arrow::util::display::array_value_to_string(batch.column(column), row)
                            .expect("a printable value")
                    })
                    .collect::<Vec<String>>()
            })
        })
        .collect())
}

/// The rows of `report`'s SQL run on DuckDB, its Python functions registered by the runtime.
pub(crate) fn python_rows(
    report: &serde_json::Value,
    handlers: &[(&str, &str)],
) -> Vec<Vec<String>> {
    python_outcome(report, handlers).expect("the converted statement runs")
}

/// The outcome of a conversion exiting `exit_code`, printing SQL or not, its mention found.
pub(crate) fn convert_outcome(exit_code: i32, printed_sql: bool) -> ConvertOutcome {
    ConvertOutcome {
        exit_code: Some(exit_code),
        printed_sql,
        json_converted: None,
        stderr_mentions: true,
    }
}

/// The standard output and standard error of `sqlquack convert` on `sql` from standard input.
pub(crate) fn convert_streams(sql: &str, flags: &[&str]) -> (String, String) {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .arg("convert")
        .args(flags)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("sqlquack runs");
    child
        .stdin
        .take()
        .expect("standard input")
        .write_all(sql.as_bytes())
        .expect("SQL written");
    let output = child.wait_with_output().expect("sqlquack finishes");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}
