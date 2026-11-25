use crate::helpers::{converted_first_value, parsed_copies};
use crate::test_types::{ChainValueTestCase, OperandCopiesTestCase};

#[test]
fn given_split_over_a_computed_operand_when_converting_then_duckdb_parses_the_operand_once() {
    let test_cases = [
        OperandCopiesTestCase {
            description: "an UPPER operand",
            sql: "SELECT SPLIT(UPPER(a || 'operand_marker'), ' ') AS c FROM (SELECT 'x y' AS a)",
            expected_copies: 1,
        },
        OperandCopiesTestCase {
            description: "an UPPER operand and an empty separator",
            sql: "SELECT SPLIT(UPPER(a || 'operand_marker'), '') AS c FROM (SELECT 'x y' AS a)",
            expected_copies: 1,
        },
    ];
    for test_case in test_cases {
        let copies = parsed_copies(test_case.sql, "operand_marker");

        assert_eq!(
            copies, test_case.expected_copies,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_split_over_a_computed_operand_when_converting_then_duckdb_returns_snowflakes_value() {
    let test_cases = [
        ChainValueTestCase {
            description: "the parts of the upper-cased text",
            sql: "SELECT TO_VARCHAR(SPLIT(UPPER(a), ' ')) AS c FROM (SELECT 'x straße' AS a)",
            expected_value: "[\"X\",\"STRASSE\"]",
        },
        ChainValueTestCase {
            description: "an empty separator keeps the text whole",
            sql: "SELECT TO_VARCHAR(SPLIT(UPPER(a), '')) AS c FROM (SELECT 'x y' AS a)",
            expected_value: "[\"X Y\"]",
        },
        ChainValueTestCase {
            description: "a NULL operand",
            sql: "SELECT COALESCE(TO_VARCHAR(SPLIT(UPPER(a), ' ')), 'none') AS c \
                  FROM (SELECT CAST(NULL AS VARCHAR) AS a)",
            expected_value: "none",
        },
    ];
    for test_case in test_cases {
        let value = converted_first_value(test_case.sql);

        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
}

#[test]
fn given_lower_when_converting_then_duckdb_parses_no_lambda() {
    let test_cases = [
        OperandCopiesTestCase {
            description: "LOWER over a column",
            sql: "SELECT LOWER(a) AS c FROM (SELECT 'X' AS a)",
            expected_copies: 0,
        },
        OperandCopiesTestCase {
            description: "LOWER over a computed operand",
            sql: "SELECT LOWER(a || 'Y') AS c FROM (SELECT 'X' AS a)",
            expected_copies: 0,
        },
    ];
    for test_case in test_cases {
        let copies = parsed_copies(test_case.sql, "\"LAMBDA\"");

        assert_eq!(
            copies, test_case.expected_copies,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_lower_when_converting_then_duckdb_returns_snowflakes_value() {
    let test_cases = [
        ChainValueTestCase {
            description: "a dotted capital I becomes i and a combining dot",
            sql: "SELECT LOWER(a) AS c FROM (SELECT '\u{130}' AS a)",
            expected_value: "i\u{307}",
        },
        ChainValueTestCase {
            description: "the mapping among other characters",
            sql: "SELECT LOWER(a) AS c FROM (SELECT 'A\u{130}B\u{130}\u{c9}' AS a)",
            expected_value: "ai\u{307}bi\u{307}\u{e9}",
        },
        ChainValueTestCase {
            description: "text without the mapped character",
            sql: "SELECT LOWER(a) AS c FROM (SELECT 'Stra\u{df}e' AS a)",
            expected_value: "stra\u{df}e",
        },
        ChainValueTestCase {
            description: "a NULL operand",
            sql: "SELECT COALESCE(LOWER(a), 'none') AS c FROM (SELECT CAST(NULL AS VARCHAR) AS a)",
            expected_value: "none",
        },
    ];
    for test_case in test_cases {
        let value = converted_first_value(test_case.sql);

        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
}
