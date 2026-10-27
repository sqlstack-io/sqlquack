use sqlquack_syntax::Dialect;

use crate::helpers::{
    strict_outcome, tolerant_finishes_within, tolerant_joined, tolerant_ranges, tolerant_tokens,
};
use crate::test_types::{
    LosslessTestCase, StrictTestCase, TerminationTestCase, TolerantRangeTestCase,
    TolerantTokenTestCase,
};

#[test]
fn given_sql_when_parsing_strictly_then_any_error_refuses_the_input() {
    let test_cases = [
        StrictTestCase {
            description: "valid statements all parse",
            sql: "SELECT 1; SELECT 2",
            expected_outcome: Ok(2),
        },
        StrictTestCase {
            description: "one invalid statement is a parse failure",
            sql: "SELECT 1; SELECT FROM WHERE",
            expected_outcome: Err(true),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            strict_outcome(test_case.sql),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_parsing_tolerantly_then_every_statement_range_is_kept() {
    let test_cases = [
        TolerantRangeTestCase {
            description: "valid and invalid statements keep their ranges",
            sql: "SELECT 1;\nSELECT FROM WHERE;\n-- note\nSELECT ';' AS s; ",
            dialect: Dialect::Snowflake,
            expected_ranges: vec![
                ("SELECT 1", true),
                ("SELECT FROM WHERE", false),
                ("SELECT ';' AS s", true),
            ],
        },
        TolerantRangeTestCase {
            description: "untokenizable input is one failed statement",
            sql: "  SELECT 'unterminated  ",
            dialect: Dialect::DuckDB,
            expected_ranges: vec![("SELECT 'unterminated", false)],
        },
        TolerantRangeTestCase {
            description: "only top-level semicolons split statements",
            sql: "SELECT f(1; 2); SELECT 3",
            dialect: Dialect::Snowflake,
            expected_ranges: vec![("SELECT f(1; 2)", false), ("SELECT 3", true)],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            tolerant_ranges(test_case.sql, test_case.dialect),
            test_case
                .expected_ranges
                .into_iter()
                .map(|(range, parsed)| (range.to_owned(), parsed))
                .collect::<Vec<_>>(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_parsing_tolerantly_then_tokens_cover_the_input_or_the_failure_is_kept() {
    let test_cases = [
        TolerantTokenTestCase {
            description: "the tokens join back into the input",
            sql: "SELECT 1;\nSELECT FROM WHERE;\n-- note\nSELECT ';' AS s; ",
            dialect: Dialect::Snowflake,
            expected_tokens: (
                "SELECT 1;\nSELECT FROM WHERE;\n-- note\nSELECT ';' AS s; ".to_owned(),
                23,
                false,
            ),
        },
        TolerantTokenTestCase {
            description: "untokenizable input has no tokens and keeps the failure",
            sql: "  SELECT 'unterminated  ",
            dialect: Dialect::DuckDB,
            expected_tokens: ("".to_owned(), 0, true),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            tolerant_tokens(test_case.sql, test_case.dialect),
            test_case.expected_tokens,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_truncated_option_value_when_parsing_tolerantly_then_parsing_finishes() {
    let test_cases = [TerminationTestCase {
        description: "a stage file format cut inside its options ends",
        sql: "ALTER TABLE l SET STAGE_FILE_FORMAT(T=(=",
        seconds: 30,
        expected_finished: true,
    }];

    for test_case in test_cases {
        assert_eq!(
            tolerant_finishes_within(test_case.sql, test_case.seconds),
            test_case.expected_finished,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_unterminated_option_list_when_parsing_tolerantly_then_the_tokens_join_back() {
    let test_cases = [
        LosslessTestCase {
            description: "an identity option list cut after START WITH keeps every token in Snowflake",
            sql: "CREATE TABLE c (pk BIGINT GENERATED ALWAYS AS IDENTITY (START WITH 10",
            dialect: Dialect::Snowflake,
            expected_joined: "CREATE TABLE c (pk BIGINT GENERATED ALWAYS AS IDENTITY (START WITH 10",
        },
        LosslessTestCase {
            description: "an identity option list cut after START WITH keeps every token in DuckDB",
            sql: "CREATE TABLE c (pk BIGINT GENERATED ALWAYS AS IDENTITY (START WITH 10",
            dialect: Dialect::DuckDB,
            expected_joined: "CREATE TABLE c (pk BIGINT GENERATED ALWAYS AS IDENTITY (START WITH 10",
        },
        LosslessTestCase {
            description: "a system-versioning option list cut after HISTORY_TABLE keeps every token in Snowflake",
            sql: "CREATE TABLE t (a INT) WITH (SYSTEM_VERSIONING = ON (HISTORY_TABLE = dbo.h",
            dialect: Dialect::Snowflake,
            expected_joined: "CREATE TABLE t (a INT) WITH (SYSTEM_VERSIONING = ON (HISTORY_TABLE = dbo.h",
        },
        LosslessTestCase {
            description: "a system-versioning option list cut after HISTORY_TABLE keeps every token in DuckDB",
            sql: "CREATE TABLE t (a INT) WITH (SYSTEM_VERSIONING = ON (HISTORY_TABLE = dbo.h",
            dialect: Dialect::DuckDB,
            expected_joined: "CREATE TABLE t (a INT) WITH (SYSTEM_VERSIONING = ON (HISTORY_TABLE = dbo.h",
        },
        LosslessTestCase {
            description: "a data-deletion option list cut after FILTER_COLUMN keeps every token in Snowflake",
            sql: "ALTER TABLE t SET (DATA_DELETION = ON (FILTER_COLUMN = a",
            dialect: Dialect::Snowflake,
            expected_joined: "ALTER TABLE t SET (DATA_DELETION = ON (FILTER_COLUMN = a",
        },
        LosslessTestCase {
            description: "a data-deletion option list cut after FILTER_COLUMN keeps every token in DuckDB",
            sql: "ALTER TABLE t SET (DATA_DELETION = ON (FILTER_COLUMN = a",
            dialect: Dialect::DuckDB,
            expected_joined: "ALTER TABLE t SET (DATA_DELETION = ON (FILTER_COLUMN = a",
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            tolerant_joined(test_case.sql, test_case.dialect),
            test_case.expected_joined,
            "{}",
            test_case.description
        );
    }
}
