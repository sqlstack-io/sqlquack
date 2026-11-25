use sqlquack_blocks::blocks::constants::UPPERCASE_EXPANSIONS;

use crate::helpers::{converted_first_value, parsed_copies};
use crate::test_types::{
    ChainValueTestCase, FullCaseMappingTestCase, OperandCopiesTestCase, PatternCopiesTestCase,
};

/// Text only an UPPER lookup holds: the uppercase mapping of U+0587.
const UPPER_LOOKUP_MARKER: &str = "\u{535}\u{552}";

#[test]
fn given_split_over_a_computed_operand_when_converting_then_duckdb_parses_the_operand_once() {
    let test_cases = [
        PatternCopiesTestCase {
            description: "an UPPER operand",
            sql: "SELECT SPLIT(UPPER(a), ' ') AS c FROM (SELECT 'x y' AS a)",
            pattern: UPPER_LOOKUP_MARKER,
            expected_copies: 1,
        },
        PatternCopiesTestCase {
            description: "an UPPER operand and an empty separator",
            sql: "SELECT SPLIT(UPPER(a), '') AS c FROM (SELECT 'x y' AS a)",
            pattern: UPPER_LOOKUP_MARKER,
            expected_copies: 1,
        },
    ];
    for test_case in test_cases {
        let copies = parsed_copies(test_case.sql, test_case.pattern);

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

#[test]
fn given_upper_when_converting_then_duckdb_parses_one_lookup_and_no_list_position() {
    let test_cases = [
        PatternCopiesTestCase {
            description: "no list_position",
            sql: "SELECT UPPER(a) AS c FROM (SELECT 'x' AS a)",
            pattern: "\"function_name\":\"list_position\"",
            expected_copies: 0,
        },
        PatternCopiesTestCase {
            description: "the mapped texts parsed once",
            sql: "SELECT UPPER(a) AS c FROM (SELECT 'x' AS a)",
            pattern: UPPER_LOOKUP_MARKER,
            expected_copies: 1,
        },
        PatternCopiesTestCase {
            description: "one lookup string_split beside the one that splits the text",
            sql: "SELECT UPPER(a) AS c FROM (SELECT 'x' AS a)",
            pattern: "\"function_name\":\"string_split\"",
            expected_copies: 2,
        },
        PatternCopiesTestCase {
            description: "one lookup per UPPER call",
            sql: "SELECT UPPER(a) AS c, UPPER(a || 'y') AS d FROM (SELECT 'x' AS a)",
            pattern: UPPER_LOOKUP_MARKER,
            expected_copies: 2,
        },
    ];
    for test_case in test_cases {
        let copies = parsed_copies(test_case.sql, test_case.pattern);

        assert_eq!(
            copies, test_case.expected_copies,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_upper_over_every_multi_character_mapping_when_converting_then_duckdb_returns_full_case_mapping()
 {
    let test_cases = [
        FullCaseMappingTestCase {
            description: "each mapped character between plain letters",
            between: "a",
            expected_mappings: 102,
        },
        FullCaseMappingTestCase {
            description: "the mapped characters side by side",
            between: "",
            expected_mappings: 102,
        },
    ];
    for test_case in test_cases {
        let text: String = UPPERCASE_EXPANSIONS
            .iter()
            .map(|(from, _)| format!("{}{from}", test_case.between))
            .collect();
        let sql = format!("SELECT UPPER(a) AS c FROM (SELECT '{text}' AS a)");

        let value = converted_first_value(&sql);

        assert_eq!(
            UPPERCASE_EXPANSIONS.len(),
            test_case.expected_mappings,
            "{}",
            test_case.description
        );
        assert_eq!(value, text.to_uppercase(), "{}", test_case.description);
    }
}

#[test]
fn given_upper_when_converting_then_duckdb_returns_snowflakes_value() {
    let test_cases = [
        ChainValueTestCase {
            description: "text without a multi-character mapping",
            sql: "SELECT UPPER(a) AS c FROM (SELECT 'abc \u{e9}' AS a)",
            expected_value: "ABC \u{c9}",
        },
        ChainValueTestCase {
            description: "a sharp s among other characters",
            sql: "SELECT UPPER(a) AS c FROM (SELECT 'stra\u{df}e' AS a)",
            expected_value: "STRASSE",
        },
        ChainValueTestCase {
            description: "a computed operand",
            sql: "SELECT UPPER(a || '\u{fb03}') AS c FROM (SELECT 'x' AS a)",
            expected_value: "XFFI",
        },
        ChainValueTestCase {
            description: "an empty text",
            sql: "SELECT '[' || UPPER(a) || ']' AS c FROM (SELECT '' AS a)",
            expected_value: "[]",
        },
        ChainValueTestCase {
            description: "a NULL operand",
            sql: "SELECT COALESCE(UPPER(a), 'none') AS c FROM (SELECT CAST(NULL AS VARCHAR) AS a)",
            expected_value: "none",
        },
    ];
    for test_case in test_cases {
        let value = converted_first_value(test_case.sql);

        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
}
