use crate::check::models::{LocalFailure, LocalVerdict};
use crate::depth::_helpers::analysed::{duckdb_analysis, postgres_analysis, sqlite_analysis};
use crate::depth::tests::helpers::{deep, failed, syntax_error};
use crate::depth::tests::test_types::AnalysisTestCase;

/// The analysis probe reads a parser's resource guard as a guard failure, as the coverage verdict does.
#[test]
fn given_an_analysis_probe_when_a_guard_refuses_then_the_verdict_is_a_guard_failure() {
    let test_cases = [
        AnalysisTestCase {
            description: "DuckDB: nesting past the depth limit is a guard failure",
            analysis: duckdb_analysis,
            sql: deep,
            expected_verdict: failed(LocalFailure::Guard),
        },
        AnalysisTestCase {
            description: "DuckDB: a syntax error is a rejection",
            analysis: duckdb_analysis,
            sql: syntax_error,
            expected_verdict: LocalVerdict::Rejected,
        },
        AnalysisTestCase {
            description: "PostgreSQL: nesting past the depth limit is a guard failure",
            analysis: postgres_analysis,
            sql: deep,
            expected_verdict: failed(LocalFailure::Guard),
        },
        AnalysisTestCase {
            description: "PostgreSQL: a syntax error is a rejection",
            analysis: postgres_analysis,
            sql: syntax_error,
            expected_verdict: LocalVerdict::Rejected,
        },
        AnalysisTestCase {
            description: "SQLite: nesting past the depth limit is a guard failure",
            analysis: sqlite_analysis,
            sql: deep,
            expected_verdict: failed(LocalFailure::Guard),
        },
        AnalysisTestCase {
            description: "SQLite: a syntax error is a rejection",
            analysis: sqlite_analysis,
            sql: syntax_error,
            expected_verdict: LocalVerdict::Rejected,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            (test_case.analysis)(&(test_case.sql)()),
            test_case.expected_verdict,
            "{}",
            test_case.description
        );
    }
}
