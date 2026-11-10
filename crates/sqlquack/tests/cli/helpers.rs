use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use sqlquack_evidence::replay::main::replay_on_engine::replay_on_engine;
use sqlquack_evidence::replay::models::{PythonFunction, PythonReplay, ReplaySettings};
use sqlquack_policy::policy::constants::EVIDENCE_FILE;

use walkdir::WalkDir;

use crate::test_types::{
    CommandOutcome, CompareTestCase, ConvertOutcome, CoverageOutcome, DivergenceOutcome,
    ErrorCheckOutcome, FunctionMacroOutcome, FunctionStdinOutcome, ProjectFile, ProjectOutcome,
    ProjectTestCase, ReviewOutcome, ReviewedRepository,
};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `published` under `SQLQUACK_EVIDENCE_ROOT` when the make targets set it, else `in_repository`.
fn evidence_dir(published: &str, in_repository: &str) -> PathBuf {
    std::env::var_os("SQLQUACK_EVIDENCE_ROOT").map_or_else(
        || repository_root().join(in_repository),
        |root| PathBuf::from(root).join(published),
    )
}

/// The repository's translation cases, in the pinned evidence or in the repository.
fn cases_dir() -> PathBuf {
    evidence_dir("convert/snowflake/cases", "cases")
}

/// The recorded translation results, in the pinned evidence or in the repository.
fn results_dir() -> PathBuf {
    evidence_dir("convert/snowflake/results", "evidence/results")
}

/// Writes `text` to `path` under the project folder `project`, making its folders.
fn write_project_file(project: &Path, path: &str, text: &str) {
    let path = project.join(path);
    std::fs::create_dir_all(path.parent().expect("a parent directory"))
        .expect("project folder created");
    std::fs::write(path, text).expect("project file written");
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
        // Read and written rather than copied, so files from the read-only evidence snapshot stay writable.
        std::fs::write(&target, std::fs::read(entry.path()).expect("file read"))
            .expect("file copied");
    }
}

/// Runs a command in a copy of the repository's policy and cases, with no recorded results.
pub(crate) fn run_on_repository_cases(arguments: &[&str], mention: &str) -> CommandOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    copy_evidence(directory.path());
    copy_files(&cases_dir(), &directory.path().join("cases"));
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
        .arg("--config")
        .arg(repository_root().join("sqlquack.toml"))
        .args(flags)
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let converted = String::from_utf8_lossy(&output.stdout);
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    let query = after_setup(&connection, &converted);
    let mut statement = connection
        .prepare(query)
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
        .arg("--config")
        .arg(repository_root().join("sqlquack.toml"))
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let converted = String::from_utf8_lossy(&output.stdout);
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection
        .execute_batch("SET TimeZone = 'UTC'")
        .expect("UTC session");
    let query = after_setup(&connection, &converted);
    connection
        .prepare(query)
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
        .arg("--config")
        .arg(repository_root().join("sqlquack.toml"))
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let converted = String::from_utf8_lossy(&output.stdout);
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    let query = after_setup(&connection, &converted);
    connection
        .query_row(query, [], |row| row.get::<_, String>(0))
        .expect("the converted statement returns a text value")
}

/// Runs the setup statements `convert` prints ahead of its SQL and returns the SQL after them.
fn after_setup<'a>(connection: &duckdb::Connection, converted: &'a str) -> &'a str {
    let (setup, query) = converted
        .trim()
        .rsplit_once(";\n")
        .unwrap_or(("", converted.trim()));
    connection
        .execute_batch(setup)
        .expect("the setup statements run");
    query
}

/// The first value the converted `sql` returns over the source file `sources`, its tables made by `setup` on DuckDB.
pub(crate) fn converted_value_over(sql: &str, tables: (&str, &str)) -> String {
    converted_result_over(sql, tables).expect("the converted statement returns a text value")
}

/// The first value `sql` returns, converted and run after `setup` and its macros, or DuckDB's error.
pub(crate) fn converted_result_over(
    sql: &str,
    (sources, setup): (&str, &str),
) -> Result<String, String> {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    write_project_file(directory.path(), "sources/tables.yml", sources);
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--json", "--config"])
        .arg(repository_root().join("sqlquack.toml"))
        .arg("--project")
        .arg(directory.path())
        .arg(directory.path().join("query.sql"))
        .output()
        .expect("sqlquack runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("a JSON report");
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    connection.execute_batch(setup).expect("tables made");
    for statement in report["macros"].as_array().expect("macros listed") {
        connection
            .execute_batch(statement.as_str().unwrap_or_default())
            .expect("the macro is made");
    }
    connection
        .query_row(report["sql"].as_str().unwrap_or_default(), [], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| error.to_string())
}

/// The exit code and standard error of converting `sql` with the override `definition`.
pub(crate) fn override_failure(sql: &str, definition: &str) -> (Option<i32>, String) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let overrides = directory.path().join("overrides");
    std::fs::create_dir(&overrides).expect("overrides directory");
    std::fs::write(overrides.join("reversed_text.sql"), definition).expect("override written");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--config"])
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
    write_project_file(
        directory.path(),
        "sources/orders.yml",
        "sources:\n  - name: orders\n    columns:\n      - name: order_id\n        type: NUMBER(10,0)\n      - name: sku\n        type: VARCHAR\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--json", "--config"])
        .arg(repository_root().join("sqlquack.toml"))
        .arg("--project")
        .arg(directory.path())
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

/// Copies the repository's evidence policy into `directory`, where the evidence commands read it.
pub(crate) fn copy_evidence(directory: &Path) {
    let target = directory.join(EVIDENCE_FILE);
    std::fs::create_dir_all(target.parent().expect("a parent folder")).expect("evidence folder");
    std::fs::copy(repository_root().join(EVIDENCE_FILE), target).expect("evidence copied");
}

/// The exit code of `arguments` beside a copy of the repository's `policy_file`, over three SQL files.
pub(crate) fn policy_free_exit_code(arguments: &[&str], policy_file: Option<&str>) -> Option<i32> {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), "SELECT 1 AS total").expect("query written");
    std::fs::write(
        directory.path().join("now.sql"),
        "SELECT CURRENT_TIMESTAMP() AS t",
    )
    .expect("query written");
    std::fs::write(directory.path().join("hash.sql"), "SELECT HASH(1) AS h")
        .expect("query written");
    policy_file.iter().for_each(|path| {
        std::fs::copy(
            repository_root().join(path),
            directory.path().join("sqlquack.toml"),
        )
        .expect("policy copied");
    });
    write_project_file(
        directory.path(),
        "functions/sql/total.sql",
        "FUNCTION (\n  arguments (),\n  returns NUMBER(1,0),\n);\n1\n",
    );
    Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(arguments)
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs")
        .status
        .code()
}

const PYTHON_PROJECT: &str = "crates/sqlquack/tests/cli/fixtures/python_functions";
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

/// The numeric_range constructs and replay mode as the evidence policy declares them.
const NUMERIC_RANGE_SECTION: &str = "[divergences.numeric_range]";
const WARN_REPLAY: &str = "replay = \"warn\"";

/// Verifies the recorded SUM overflow case with a marker line and the divergence's replay mode.
pub(crate) fn verify_overflow_case(case_marker: &str, policy_default: &str) -> DivergenceOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = repository_root();
    let policy = std::fs::read_to_string(root.join(EVIDENCE_FILE)).expect("policy read");
    let (before, section) = policy
        .split_once(NUMERIC_RANGE_SECTION)
        .expect("the policy declares numeric_range");
    let section = section.replacen(WARN_REPLAY, &format!("replay = \"{policy_default}\""), 1);
    let policy = format!("{before}{NUMERIC_RANGE_SECTION}{section}");
    copy_evidence(directory.path());
    std::fs::write(directory.path().join(EVIDENCE_FILE), policy).expect("policy written");
    copy_files(
        &cases_dir().join("fidelity"),
        &directory.path().join("cases/fidelity"),
    );
    let cases = directory.path().join("cases/aggregate");
    std::fs::create_dir_all(&cases).expect("cases directory");
    std::fs::write(
        cases.join("overflow.toml"),
        format!("{OVERFLOW_CASE}{case_marker}"),
    )
    .expect("case written");
    copy_files(&results_dir(), &directory.path().join("evidence/results"));
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
    copy_evidence(directory.path());
    copy_files(
        &cases_dir().join("fidelity"),
        &directory.path().join("cases/fidelity"),
    );
    let cases = directory.path().join("cases/conversion");
    std::fs::create_dir_all(&cases).expect("cases directory");
    std::fs::copy(
        cases_dir().join("conversion/to_number.toml"),
        cases.join("to_number.toml"),
    )
    .expect("case copied");
    copy_files(&results_dir(), &directory.path().join("evidence/results"));
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

/// Runs `sqlquack review` on the policy, recorded results and the cases behind review `files`.
pub(crate) fn review_repository(files: &[&str]) -> ReviewedRepository {
    let directory = review_directory();
    for file in files {
        let case = Path::new("cases").join(file).with_extension("toml");
        let target = directory.path().join(&case);
        std::fs::create_dir_all(target.parent().expect("a parent directory"))
            .expect("directory created");
        std::fs::copy(cases_dir().join(file).with_extension("toml"), target).expect("case copied");
    }
    copy_files(&results_dir(), &directory.path().join("evidence/results"));
    ReviewedRepository {
        exit_code: run_review(directory.path()),
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
    let directory = review_directory();
    copy_files(
        &cases_dir().join("project-functions"),
        &directory.path().join("cases/project-functions"),
    );
    let exit_code = run_review(directory.path());
    let text =
        std::fs::read_to_string(directory.path().join("exported").join(file)).unwrap_or_default();
    ReviewOutcome {
        exit_code,
        missing_lines: expected_lines
            .iter()
            .filter(|expected| !text.lines().any(|line| line.starts_with(*expected)))
            .map(|expected| (*expected).to_owned())
            .collect(),
    }
}

/// A temporary directory holding the policy and the equivalence files every review needs.
fn review_directory() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary directory");
    copy_evidence(directory.path());
    for entry in WalkDir::new(cases_dir())
        .into_iter()
        .flatten()
        .filter(|entry| {
            std::fs::read_to_string(entry.path())
                .is_ok_and(|text| text.lines().any(|line| line.starts_with("equivalence = ")))
        })
    {
        let relative = entry
            .path()
            .strip_prefix(cases_dir())
            .expect("inside the cases");
        let target = directory.path().join("cases").join(relative);
        std::fs::create_dir_all(target.parent().expect("a parent directory"))
            .expect("directory created");
        std::fs::copy(entry.path(), target).expect("file copied");
    }
    directory
}

/// The exit code of `sqlquack review --out exported` run in `directory`.
fn run_review(directory: &Path) -> Option<i32> {
    Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["review", "--out", "exported"])
        .current_dir(directory)
        .output()
        .expect("sqlquack runs")
        .status
        .code()
}

/// Converts `DOUBLED(PRICE NUMBER(10,2))`, its project file returning `returns`, with `sqlquack function`, then runs its macro.
pub(crate) fn run_function_macro(
    (returns, body): (&str, &str),
    call: &str,
    flags: &[&str],
) -> FunctionMacroOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    write_project_file(
        directory.path(),
        "sources/fees.yml",
        "sources:\n  - name: __sqlquack_relation__fees\n    columns:\n      - name: amount\n        type: NUMBER(10,2)\n",
    );
    write_project_file(
        directory.path(),
        "functions/sql/doubled.sql",
        &format!(
            "FUNCTION (\n  arguments (price NUMBER(10,2)),\n  returns {returns},\n);\n{body}\n"
        ),
    );
    std::fs::write(directory.path().join("doubled.sql"), body).expect("body written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["function", "doubled.sql", "--name", "DOUBLED"])
        .arg("--config")
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
    for statement in converted["macros"].as_array().into_iter().flatten() {
        connection
            .execute_batch(statement.as_str().unwrap_or_default())
            .expect("a helper macro is created");
    }
    connection
        .execute_batch(&format!(
            "CREATE TABLE \"__SQLQUACK_RELATION__FEES\" AS \
             SELECT CAST(amount AS DECIMAL(10,2)) AS \"AMOUNT\" FROM (VALUES (1.00), (2.00)) AS t(amount); \
             CREATE MACRO \"DOUBLED\"({}) AS {macro_body}",
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
    copy_evidence(directory.path());
    copy_files(
        &cases_dir().join("fidelity"),
        &directory.path().join("cases/fidelity"),
    );
    let cases = directory.path().join("cases/semi-structured");
    std::fs::create_dir_all(&cases).expect("cases directory");
    std::fs::copy(
        cases_dir().join("semi-structured/arrays_batch2.toml"),
        cases.join("arrays_batch2.toml"),
    )
    .expect("case copied");
    copy_files(&results_dir(), &directory.path().join("evidence/results"));
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

[[case]]
id = "zero_divisor"
description = "A division by zero, which Snowflake raises as an error."
columns = [{ name = "quantity", type = "NUMBER(38,0)" }]
rows = [["1"]]
query = "SELECT quantity / 0 AS ratio FROM input"
order = "ordered"
tags = ["error"]
"#;

/// Each pending file's case reference, `compile_error` and `expected_error`.
pub(crate) fn pending_flags() -> Vec<(String, Option<bool>, Option<bool>)> {
    let directory = tempfile::tempdir().expect("temporary directory");
    copy_evidence(directory.path());
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
    let mut flags: Vec<(String, Option<bool>, Option<bool>)> =
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
                    pending["expected_error"].as_bool(),
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
    let builtin = flags.contains(&"--no-config") || flags.contains(&"--config");
    let policy_flags: Vec<&std::ffi::OsStr> =
        [std::ffi::OsStr::new("--config"), policy.as_os_str()]
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
        printed_sql: stdout
            .split(";\n")
            .find(|statement| !statement.starts_with("CREATE "))
            .is_some_and(|statement| {
                statement.starts_with("SELECT") || statement.starts_with("WITH")
            }),
        json_converted: json["converted"].as_bool(),
        stderr_mentions: String::from_utf8_lossy(&output.stderr).contains(mention),
    }
}

/// The `convert --json` report of `sql` over the Python functions fixture, with `flags`.
pub(crate) fn python_report(sql: &str, flags: &[&str]) -> serde_json::Value {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--json", "--config"])
        .arg(repository_root().join("sqlquack.toml"))
        .arg("--project")
        .arg(repository_root().join(PYTHON_PROJECT))
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
    let macros: Vec<String> = report["macros"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|statement| statement.as_str().map(str::to_owned))
        .collect();
    let rows = replay_on_engine(
        (&macros, report["sql"].as_str().unwrap_or_default()),
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

/// Exit code, standard output and standard error of `sqlquack arguments` in `directory` on `input`.
pub(crate) fn run_with_input(
    directory: &std::path::Path,
    arguments: &[&str],
    input: &str,
) -> (Option<i32>, String, String) {
    run_with_environment(directory, arguments, input, &[])
}

/// Runs `sqlquack` in an empty folder, with a copy of the repository's `policy_file` as its sqlquack.toml.
pub(crate) fn run_beside_policy(
    arguments: &[&str],
    (policy_file, input): (Option<&str>, &str),
) -> (Option<i32>, String, String) {
    let directory = tempfile::tempdir().expect("temporary directory");
    policy_file.iter().for_each(|path| {
        std::fs::copy(
            repository_root().join(path),
            directory.path().join("sqlquack.toml"),
        )
        .expect("policy copied");
    });
    run_with_input(directory.path(), arguments, input)
}

/// Runs `sqlquack` like `run_with_input`, with the colour variables cleared and then `environment` set.
pub(crate) fn run_with_environment(
    directory: &std::path::Path,
    arguments: &[&str],
    input: &str,
    environment: &[(&str, &str)],
) -> (Option<i32>, String, String) {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(arguments)
        .current_dir(directory)
        .env_remove("NO_COLOR")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .env_remove("FORCE_COLOR")
        .envs(environment.iter().copied())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("sqlquack runs");
    child
        .stdin
        .take()
        .expect("standard input")
        .write_all(input.as_bytes())
        .expect("input written");
    let output = child.wait_with_output().expect("sqlquack finishes");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// How `sqlquack function` ends on `body` from standard input, the project declaring `DOUBLED`.
pub(crate) fn function_from_stdin(name: &str, body: &str) -> FunctionStdinOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    write_project_file(
        directory.path(),
        "functions/sql/doubled.sql",
        "FUNCTION (\n  arguments (price NUMBER(10,2)),\n  returns BOOLEAN,\n);\nprice > 1\n",
    );
    let (exit_code, stdout, stderr) = run_with_input(
        directory.path(),
        &["function", "-", "--name", name, "--no-config"],
        body,
    );
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();
    FunctionStdinOutcome {
        exit_code,
        converted: report["converted"].as_bool(),
        first_code: report["refusals"][0]["code"].as_str().map(str::to_owned),
        stderr_lines: stderr
            .lines()
            .filter(|line| {
                line.starts_with("error[") || line.contains(" --> ") || line.contains("failed:")
            })
            .map(str::to_owned)
            .collect(),
    }
}

/// A committed recording of `I` NUMBER(38,0) and `X` TEXT: rows (0, 'fig') and (1, 'pear').
const FRUIT_RECORDING: &str = "1707ba43b41cc7c6da39bc8bc7c4825e471d14c66d8a30e3d1495b25be170e8f";

/// The outcome `sqlquack compare` prints for one case over the fruit recording and its inputs.
pub(crate) fn compare_outcome(test_case: &CompareTestCase) -> String {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database = directory.path().join("inputs.duckdb");
    duckdb::Connection::open(&database)
        .expect("DuckDB opens")
        .execute_batch(
            "CREATE TABLE fruit (i INTEGER, x VARCHAR); INSERT INTO fruit VALUES (0, 'fig'), (1, 'pear');",
        )
        .expect("inputs written");
    let recording = results_dir().join(FRUIT_RECORDING);
    let mut sidecar: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(recording.with_extension("json")).expect("sidecar read"),
    )
    .expect("sidecar parses");
    sidecar["arrow_sha256"] = test_case
        .arrow_sha256
        .map_or(sidecar["arrow_sha256"].clone(), serde_json::Value::from);
    let recorded = directory.path().join("recorded.json");
    std::fs::write(&recorded, sidecar.to_string()).expect("sidecar written");
    std::fs::copy(
        recording.with_extension("arrow"),
        recorded.with_extension("arrow"),
    )
    .expect("Arrow file copied");
    let sql = directory.path().join("converted.sql");
    std::fs::write(&sql, test_case.sql).expect("SQL written");
    let entry = serde_json::json!({
        "id": "fruit",
        "recorded": recorded,
        "database": database,
        "sql": sql,
        "ordered": test_case.ordered,
    });
    let manifest = directory.path().join("manifest.json");
    std::fs::write(&manifest, serde_json::json!([entry]).to_string()).expect("manifest written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .current_dir(repository_root())
        .args(["compare", "--manifest"])
        .arg(&manifest)
        .output()
        .expect("sqlquack runs");
    let outcomes: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("outcomes are JSON");
    format!("{} {}", outcomes[0]["id"], outcomes[0]["outcome"])
}

/// Each statement `verify --translation-statements` writes for the batch-2 aggregates and fidelity.
pub(crate) fn translation_statements() -> Vec<serde_json::Value> {
    let directory = tempfile::tempdir().expect("temporary directory");
    copy_evidence(directory.path());
    copy_files(
        &cases_dir().join("fidelity"),
        &directory.path().join("cases/fidelity"),
    );
    let cases = directory.path().join("cases/aggregate");
    std::fs::create_dir_all(&cases).expect("cases directory");
    std::fs::copy(
        cases_dir().join("aggregate/aggregates_batch2.toml"),
        cases.join("aggregates_batch2.toml"),
    )
    .expect("case copied");
    copy_files(&results_dir(), &directory.path().join("evidence/results"));
    let status = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["verify", "--translation-statements", "statements.json"])
        .current_dir(directory.path())
        .status()
        .expect("sqlquack runs");
    assert!(status.success(), "verify fails: {status}");
    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(directory.path().join("statements.json")).expect("written"),
    )
    .expect("statements parse");
    assert_eq!(written["format"], "sqlquack-translation-statements/1");
    written["statements"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// The first refusal's help converting `sql` under the repository policy, its path as `<policy>`.
pub(crate) fn first_refusal_help(sql: &str, flags: &[&str]) -> String {
    let policy = repository_root().join("sqlquack.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--json", "--config"])
        .arg(&policy)
        .args(flags)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(sql.as_bytes())?;
            child.wait_with_output()
        })
        .expect("sqlquack runs");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("a JSON report");
    report["refusals"][0]["help"]
        .as_str()
        .unwrap_or_default()
        .replace(&policy.display().to_string(), "<policy>")
}

/// The exit code and standard error of converting a statement in a project with the source file `text`.
pub(crate) fn source_failure(text: &str) -> (Option<i32>, String) {
    let directory = tempfile::tempdir().expect("temporary directory");
    write_project_file(directory.path(), "sources/orders.yml", text);
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--no-config", "--project"])
        .arg(directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .expect("stdin")
                .write_all(b"SELECT 1 AS one")?;
            child.wait_with_output()
        })
        .expect("sqlquack runs");
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// `names` as owned strings, for comparing with converted column names.
pub(crate) fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// `sqlquack convert --json` run inside a project folder built from `test_case`.
pub(crate) fn project_convert(test_case: &ProjectTestCase) -> ProjectOutcome {
    let directory = project_folder(test_case.files, test_case.folder, test_case.sql);
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["convert", "--json"])
        .args(test_case.flags)
        .arg("query.sql")
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    ProjectOutcome {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// `sqlquack run query.sql` with `flags` in a folder holding the project `files` and `sql`.
pub(crate) fn project_run(files: &[ProjectFile], sql: &str, flags: &[&str]) -> ProjectOutcome {
    let directory = project_folder(files, "", sql);
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["run", "query.sql"])
        .args(flags)
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    ProjectOutcome {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// `sqlquack run` over a Parquet fixture `rows` writes with DuckDB, typed by `columns`.
pub(crate) fn parquet_run(
    files: &[ProjectFile],
    (columns, rows): (&str, &str),
    sql: &str,
) -> ProjectOutcome {
    let directory = project_folder(files, "", sql);
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    let path = directory.path().join("orders.parquet");
    connection
        .execute_batch(&format!(
            "CREATE TABLE t ({columns}); INSERT INTO t VALUES {rows}; COPY t TO '{}' (FORMAT parquet)",
            path.display()
        ))
        .expect("Parquet written");
    let output = Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .args(["run", "query.sql", "--fixture", "orders=./orders.parquet"])
        .current_dir(directory.path())
        .output()
        .expect("sqlquack runs");
    ProjectOutcome {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// A temporary folder holding the project `files` under `folder`, and `sql` as `query.sql`.
fn project_folder(files: &[ProjectFile], folder: &str, sql: &str) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temporary directory");
    let project = directory.path().join(folder);
    std::fs::create_dir_all(&project).expect("project folder created");
    for file in files {
        let path = project.join(file.path);
        std::fs::create_dir_all(path.parent().expect("a parent directory"))
            .expect("directory created");
        let text: String = file
            .repository
            .iter()
            .map(|copied| {
                std::fs::read_to_string(repository_root().join(copied)).expect("file read") + "\n"
            })
            .chain(std::iter::once(file.text.to_owned()))
            .collect();
        std::fs::write(path, text).expect("file written");
    }
    std::fs::write(directory.path().join("query.sql"), sql).expect("query written");
    directory
}
