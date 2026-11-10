use crate::lint::tests::helpers::diagnostics_for_dialect;
use crate::lint::tests::test_types::TrailingFromTestCase;

/// A query whose final FROM names no relation: the terminal-shape rule reads past that FROM.
const SQL: &str = "WITH t AS (SELECT 1) SELECT * FROM";

#[test]
fn given_a_query_ending_in_from_when_linting_then_each_dialect_refuses_or_lints_it() {
    let test_cases = [
        TrailingFromTestCase {
            description: "Snowflake refuses it",
            dialect: "snowflake",
            expected_parse_error: true,
        },
        TrailingFromTestCase {
            description: "DuckDB refuses it",
            dialect: "duckdb",
            expected_parse_error: true,
        },
        TrailingFromTestCase {
            description: "SQLite refuses it",
            dialect: "sqlite",
            expected_parse_error: true,
        },
        TrailingFromTestCase {
            description: "PostgreSQL refuses it",
            dialect: "postgres",
            expected_parse_error: true,
        },
        TrailingFromTestCase {
            description: "Generic refuses it",
            dialect: "generic",
            expected_parse_error: true,
        },
        TrailingFromTestCase {
            description: "BigQuery refuses it",
            dialect: "bigquery",
            expected_parse_error: true,
        },
        TrailingFromTestCase {
            description: "Databricks parses it, so the terminal-shape rule lints it",
            dialect: "databricks",
            expected_parse_error: false,
        },
    ];
    for test_case in test_cases {
        let result = diagnostics_for_dialect(SQL, test_case.dialect, &["SQLCLEAN035"]);
        assert_eq!(
            result
                .as_ref()
                .err()
                .is_some_and(|error| error.contains("Parse error")),
            test_case.expected_parse_error,
            "{}: {result:?}",
            test_case.description
        );
        assert_eq!(
            result.is_ok(),
            !test_case.expected_parse_error,
            "{}: {result:?}",
            test_case.description
        );
    }
}
