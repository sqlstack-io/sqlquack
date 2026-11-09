use serde_json::json;
use sqlquack_syntax::Dialect;

use crate::helpers::{faithful_call, projection_at, same_in_both_modes};
use crate::test_types::{FaithfulCallTestCase, ModeAgreementTestCase, ProjectionTestCase};

#[test]
fn given_lowered_spellings_when_parsing_faithfully_then_calls_stay_as_written() {
    let test_cases = [
        FaithfulCallTestCase {
            description: "LEN stays a call with 1 arguments and lowers to length",
            sql: "SELECT LEN(s)",
            expected_call: (json!("LEN"), Some(1), "length".to_owned()),
        },
        FaithfulCallTestCase {
            description: "IFNULL stays a call with 2 arguments and lowers to coalesce",
            sql: "SELECT IFNULL(a, b)",
            expected_call: (json!("IFNULL"), Some(2), "coalesce".to_owned()),
        },
        FaithfulCallTestCase {
            description: "IFF stays a call with 3 arguments and lowers to if_func",
            sql: "SELECT IFF(c, 1, 2)",
            expected_call: (json!("IFF"), Some(3), "if_func".to_owned()),
        },
        FaithfulCallTestCase {
            description: "TRY_TO_NUMBER stays a call with 1 arguments and lowers to to_number",
            sql: "SELECT TRY_TO_NUMBER(s)",
            expected_call: (json!("TRY_TO_NUMBER"), Some(1), "to_number".to_owned()),
        },
        FaithfulCallTestCase {
            description: "MOD stays a call with 2 arguments and lowers to mod_func",
            sql: "SELECT MOD(a, b)",
            expected_call: (json!("MOD"), Some(2), "mod_func".to_owned()),
        },
        FaithfulCallTestCase {
            description: "SUM stays a call with 1 arguments and lowers to sum",
            sql: "SELECT SUM(x)",
            expected_call: (json!("SUM"), Some(1), "sum".to_owned()),
        },
        FaithfulCallTestCase {
            description: "COUNT stays a call with 1 arguments and lowers to count",
            sql: "SELECT COUNT(*)",
            expected_call: (json!("COUNT"), Some(1), "count".to_owned()),
        },
        FaithfulCallTestCase {
            description: "ROW_NUMBER stays a call with 0 arguments and lowers to row_number",
            sql: "SELECT ROW_NUMBER()",
            expected_call: (json!("ROW_NUMBER"), Some(0), "row_number".to_owned()),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            faithful_call(test_case.sql),
            test_case.expected_call,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_keyword_syntax_and_kept_calls_when_parsing_faithfully_then_both_modes_agree() {
    let test_cases = [
        ModeAgreementTestCase {
            description: "SELECT EXTRACT(year FROM d) parses the same in both modes",
            sql: "SELECT EXTRACT(year FROM d)",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        ModeAgreementTestCase {
            description: "SELECT POSITION('a' IN s) parses the same in both modes",
            sql: "SELECT POSITION('a' IN s)",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        ModeAgreementTestCase {
            description: "SELECT TRIM(BOTH ' ' FROM s) parses the same in both modes",
            sql: "SELECT TRIM(BOTH ' ' FROM s)",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        ModeAgreementTestCase {
            description: "SELECT DATEADD(day, 1, d) parses the same in both modes",
            sql: "SELECT DATEADD(day, 1, d)",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        ModeAgreementTestCase {
            description: "SELECT DATE_PART(hour, t) parses the same in both modes",
            sql: "SELECT DATE_PART(hour, t)",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        ModeAgreementTestCase {
            description: "SELECT CONCAT(a, b) parses the same in both modes",
            sql: "SELECT CONCAT(a, b)",
            dialect: Dialect::Snowflake,
            expected_same: true,
        },
        ModeAgreementTestCase {
            description: "DuckDB calls are not changed",
            sql: "SELECT length(s), coalesce(a, b)",
            dialect: Dialect::DuckDB,
            expected_same: true,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            same_in_both_modes(test_case.sql, test_case.dialect),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_quoted_path_keys_with_dots_when_parsing_then_they_stay_in_brackets() {
    let test_cases = [
        ProjectionTestCase {
            description: "SELECT v:\"a.b\" FROM t keeps the path [\"a.b\"]",
            sql: "SELECT v:\"a.b\" FROM t",
            pointer: "/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("[\"a.b\"]"),
        },
        ProjectionTestCase {
            description: "SELECT v:a.\"x.y\" FROM t keeps the path a[\"x.y\"]",
            sql: "SELECT v:a.\"x.y\" FROM t",
            pointer: "/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("a[\"x.y\"]"),
        },
        ProjectionTestCase {
            description: "SELECT v:a.\"x y\"[2] FROM t keeps the path a[\"x y\"][2]",
            sql: "SELECT v:a.\"x y\"[2] FROM t",
            pointer: "/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("a[\"x y\"][2]"),
        },
        ProjectionTestCase {
            description: "SELECT v:a['b.c'] FROM t keeps the path a[\"b.c\"]",
            sql: "SELECT v:a['b.c'] FROM t",
            pointer: "/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("a[\"b.c\"]"),
        },
        ProjectionTestCase {
            description: "SELECT v:a.b FROM t keeps the path a.b",
            sql: "SELECT v:a.b FROM t",
            pointer: "/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("a.b"),
        },
        ProjectionTestCase {
            description: "SELECT v:\"Q\".r FROM t keeps the path Q.r",
            sql: "SELECT v:\"Q\".r FROM t",
            pointer: "/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("Q.r"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            projection_at(test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_interval_qualifier_with_precisions_when_parsing_faithfully_then_both_are_kept() {
    let qualifier = |this, leading, to, fraction| {
        json!({"type": "qualifier", "this": this, "leading_precision": leading,
            "expression": to, "fractional_precision": fraction})
    };
    let test_cases = [
        ProjectionTestCase {
            description: "YEAR(9) TO MONTH keeps the leading precision on its field",
            sql: "SELECT INTERVAL '1' YEAR(9) TO MONTH",
            pointer: "/interval/unit",
            expected_value: qualifier("Year", json!(9), json!("Month"), json!(null)),
        },
        ProjectionTestCase {
            description: "SECOND(9), (3) leaves (3) as the next select item",
            sql: "SELECT INTERVAL '1' SECOND(9), (3)",
            pointer: "/interval/unit",
            expected_value: qualifier("Second", json!(9), json!(null), json!(null)),
        },
        ProjectionTestCase {
            description: "DAY TO SECOND(3) keeps the fractional precision",
            sql: "SELECT INTERVAL '1' DAY TO SECOND(3)",
            pointer: "/interval/unit",
            expected_value: qualifier("Day", json!(null), json!("Second"), json!(3)),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            projection_at(test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
