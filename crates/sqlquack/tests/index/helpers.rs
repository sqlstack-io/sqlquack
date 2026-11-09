use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::test_types::{CaseIndexOutcome, IndexOutcome};

const SOURCES: &str = "sources:\n  - name: customers\n    columns:\n      - name: customer_id\n        \
                       type: NUMBER(38,0)\n      - name: active_flag\n        type: VARCHAR\n      \
                       - name: credit\n        type: VARCHAR\n      - name: note\n        type: VARCHAR\n";
const CORPUS: &[(&str, &str)] = &[
    (
        "active.sql",
        "SELECT customer_id, TRY_CAST(active_flag AS BOOLEAN) AS active FROM customers",
    ),
    (
        "credit.sql",
        "SELECT customer_id, CAST(credit AS NUMBER(10, 2)) AS credit FROM customers",
    ),
    (
        "plain.sql",
        "SELECT customer_id FROM customers WHERE customer_id > 10",
    ),
    ("notes.sql", "SELECT HASH(note) AS hashed FROM customers"),
];

fn sqlquack(directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sqlquack"))
        .current_dir(directory)
        .args(arguments)
        .output()
        .expect("sqlquack runs")
}

fn names(index: &serde_json::Value, section: &str, id: &str, field: &str) -> Vec<String> {
    index[section]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| entry["id"] == id)
        .flat_map(|entry| entry[field].as_array().cloned().unwrap_or_default())
        .filter_map(|name| name.as_str().map(str::to_owned))
        .collect()
}

pub(crate) fn index_shop_corpus() -> IndexOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::create_dir(directory.path().join("corpus")).expect("corpus directory");
    for (name, sql) in CORPUS {
        std::fs::write(directory.path().join("corpus").join(name), sql).expect("fixture written");
    }
    std::fs::create_dir(directory.path().join("sources")).expect("sources directory");
    std::fs::write(directory.path().join("sources/customers.yml"), SOURCES)
        .expect("sources written");
    let output = sqlquack(
        directory.path(),
        &["index", "--builtin-policy", "--json", "corpus"],
    );
    let index: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    IndexOutcome {
        exit_code: output.status.code(),
        text_to_boolean_files: names(&index, "building_blocks", "duckdb.text_to_boolean", "files"),
        text_to_number_files: names(&index, "building_blocks", "duckdb.text_to_number", "files"),
        hash_refusals: names(&index, "refusals", "fn:HASH", "files"),
        converted: index["totals"]["converted"].as_u64().unwrap_or_default(),
        refused: index["totals"]["refused"].as_u64().unwrap_or_default(),
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Indexes a copy of the repository policy and its conversion cases twice.
pub(crate) fn index_repository_cases() -> CaseIndexOutcome {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = workspace_root();
    std::fs::copy(
        root.join("sqlquack.toml"),
        directory.path().join("sqlquack.toml"),
    )
    .expect("policy copied");
    let cases = directory.path().join("cases/conversion");
    std::fs::create_dir_all(&cases).expect("cases directory");
    for entry in std::fs::read_dir(root.join("cases/conversion")).expect("conversion cases") {
        let path = entry.expect("a directory entry").path();
        std::fs::copy(&path, cases.join(path.file_name().expect("a file name")))
            .expect("case copied");
    }
    let first = sqlquack(directory.path(), &["index"]);
    let second = sqlquack(directory.path(), &["index"]);
    let text = String::from_utf8_lossy(&first.stdout);
    let block = text
        .split("\n\n")
        .find(|section| section.starts_with("building block duckdb.text_to_boolean:"))
        .unwrap_or_default();
    CaseIndexOutcome {
        identical_runs: first.status.success() && first.stdout == second.stdout,
        boolean_text_cases_attributed: [
            "  case conversion/to_boolean.toml#text_to_boolean",
            "  case conversion/to_boolean.toml#try_cast_boolean",
        ]
        .iter()
        .all(|expected| block.lines().any(|line| line == *expected)),
        number_to_boolean_not_attributed: !block.is_empty()
            && !block.contains("conversion/to_boolean.toml#number_to_boolean"),
    }
}
