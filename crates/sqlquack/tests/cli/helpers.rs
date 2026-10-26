use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::WalkDir;

use crate::test_types::{
    CommandOutcome, DivergenceOutcome, ErrorCheckOutcome, FunctionMacroOutcome, ReviewOutcome,
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

/// Runs `sqlquack review` on a copy of the evidence; reports expected lines missing from one file.
pub(crate) fn review_repository(file: &str, expected_lines: &[&str]) -> ReviewOutcome {
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
    let text =
        std::fs::read_to_string(directory.path().join("exported").join(file)).unwrap_or_default();
    ReviewOutcome {
        exit_code: output.status.code(),
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
