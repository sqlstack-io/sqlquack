use serde_json::json;

use crate::verdicts::main::current_verdicts::current_verdicts;
use crate::verdicts::tests::helpers::one_file;
use crate::verdicts::tests::test_types::CurrentTestCase;

#[test]
fn given_a_corpus_file_when_parsed_then_each_dialect_counts_statements_and_refusals() {
    let test_cases = [
        CurrentTestCase {
            description: "a Snowflake-only and a DuckDB-only statement each count in their dialect",
            entries: vec![
                ("snowflake", "SELECT v:a FROM t"),
                ("duckdb", "SELECT FROM WHERE"),
            ],
            expected_verdicts: vec![("duckdb", 1, vec![1]), ("snowflake", 1, vec![])],
        },
        CurrentTestCase {
            description: "an entry for both dialects is parsed as each, refused in only one",
            entries: vec![("both", "SELECT 1 ** 2"), ("both", "SELECT (")],
            expected_verdicts: vec![("duckdb", 2, vec![1]), ("snowflake", 2, vec![0, 1])],
        },
    ];
    for test_case in test_cases {
        let directory = tempfile::tempdir().expect("temporary directory");
        let entries: Vec<serde_json::Value> = test_case
            .entries
            .iter()
            .map(|(dialect, sql)| json!({ "dialect": dialect, "sql": sql }))
            .collect();
        std::fs::write(
            directory.path().join("c.json"),
            json!({ "entries": entries }).to_string(),
        )
        .expect("corpus written");
        std::fs::write(directory.path().join("notes.txt"), "not a corpus").expect("note written");
        let parse = current_verdicts(directory.path()).expect("verdicts");
        assert_eq!(
            (parse.verdicts.corpora, parse.broken),
            (one_file(test_case.expected_verdicts).corpora, vec![]),
            "{}",
            test_case.description
        );
    }
}
