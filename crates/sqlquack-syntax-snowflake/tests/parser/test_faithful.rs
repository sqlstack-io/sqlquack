use serde_json::json;
use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{faithful_call, kind, projection_at, statements};
use crate::test_types::{FaithfulCallTestCase, ProjectionKindTestCase, ProjectionTestCase};

#[test]
fn given_lowered_spellings_when_parsing_faithfully_then_calls_stay_as_written() {
    let test_cases = [
        FaithfulCallTestCase {
            description: "LEN stays a call with 1 arguments",
            sql: "SELECT LEN(s)",
            expected_call: (json!("LEN"), Some(1)),
        },
        FaithfulCallTestCase {
            description: "IFNULL stays a call with 2 arguments",
            sql: "SELECT IFNULL(a, b)",
            expected_call: (json!("IFNULL"), Some(2)),
        },
        FaithfulCallTestCase {
            description: "IFF stays a call with 3 arguments",
            sql: "SELECT IFF(c, 1, 2)",
            expected_call: (json!("IFF"), Some(3)),
        },
        FaithfulCallTestCase {
            description: "TRY_TO_NUMBER stays a call with 1 arguments",
            sql: "SELECT TRY_TO_NUMBER(s)",
            expected_call: (json!("TRY_TO_NUMBER"), Some(1)),
        },
        FaithfulCallTestCase {
            description: "MOD stays a call with 2 arguments",
            sql: "SELECT MOD(a, b)",
            expected_call: (json!("MOD"), Some(2)),
        },
        FaithfulCallTestCase {
            description: "SUM stays a call with 1 arguments",
            sql: "SELECT SUM(x)",
            expected_call: (json!("SUM"), Some(1)),
        },
        FaithfulCallTestCase {
            description: "COUNT stays a call with 1 arguments",
            sql: "SELECT COUNT(*)",
            expected_call: (json!("COUNT"), Some(1)),
        },
        FaithfulCallTestCase {
            description: "ROW_NUMBER stays a call with 0 arguments",
            sql: "SELECT ROW_NUMBER()",
            expected_call: (json!("ROW_NUMBER"), Some(0)),
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
fn given_keyword_syntax_and_kept_calls_when_parsing_faithfully_then_they_keep_their_typed_node() {
    let test_cases = [
        ProjectionKindTestCase {
            description: "SELECT EXTRACT(year FROM d): keyword syntax stays a typed node",
            sql: "SELECT EXTRACT(year FROM d)",
            dialect: Dialect::Snowflake,
            expected_kind: "extract",
        },
        ProjectionKindTestCase {
            description: "SELECT POSITION('a' IN s): keyword syntax stays a typed node",
            sql: "SELECT POSITION('a' IN s)",
            dialect: Dialect::Snowflake,
            expected_kind: "str_position",
        },
        ProjectionKindTestCase {
            description: "SELECT TRIM(BOTH ' ' FROM s): keyword syntax stays a typed node",
            sql: "SELECT TRIM(BOTH ' ' FROM s)",
            dialect: Dialect::Snowflake,
            expected_kind: "trim",
        },
        ProjectionKindTestCase {
            description:
                "SELECT DATEADD(day, 1, d): a call the typed parse keeps under its own name",
            sql: "SELECT DATEADD(day, 1, d)",
            dialect: Dialect::Snowflake,
            expected_kind: "function",
        },
        ProjectionKindTestCase {
            description:
                "SELECT DATE_PART(hour, t): a call the typed parse keeps under its own name",
            sql: "SELECT DATE_PART(hour, t)",
            dialect: Dialect::Snowflake,
            expected_kind: "function",
        },
        ProjectionKindTestCase {
            description: "SELECT CONCAT(a, b): a call the typed parse keeps under its own name",
            sql: "SELECT CONCAT(a, b)",
            dialect: Dialect::Snowflake,
            expected_kind: "function",
        },
        ProjectionKindTestCase {
            description: "SELECT length(s): DuckDB keeps its typed nodes",
            sql: "SELECT length(s)",
            dialect: Dialect::DuckDB,
            expected_kind: "length",
        },
        ProjectionKindTestCase {
            description: "SELECT coalesce(a, b): DuckDB keeps its typed nodes",
            sql: "SELECT coalesce(a, b)",
            dialect: Dialect::DuckDB,
            expected_kind: "coalesce",
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind(
                &statements(test_case.sql, test_case.dialect, CallMode::Faithful)[0]["select"]
                    ["expressions"][0]
            ),
            test_case.expected_kind,
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

#[test]
fn given_snowflake_derived_tables_join_aliases_and_semantic_views_when_parsing_then_the_tree_names_what_they_are(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "a DELETE USING subquery is a derived table with its alias",
            sql: "DELETE FROM v1 USING ( SELECT 1 ) AS s ( c ) , t2 WHERE TRUE ;",
            pointer: "/0/delete/using_items/0/subquery/alias/name",
            expected_value: json!("s"),
        },
        ProjectionTestCase {
            description: "a DELETE USING subquery keeps its column aliases",
            sql: "DELETE FROM v1 USING ( SELECT 1 ) AS s ( c ) , t2 WHERE TRUE ;",
            pointer: "/0/delete/using_items/0/subquery/column_aliases/0/name",
            expected_value: json!("c"),
        },
        ProjectionTestCase {
            description: "a table beside a DELETE USING subquery stays a table",
            sql: "DELETE FROM v1 USING ( SELECT 1 ) AS s ( c ) , t2 WHERE TRUE ;",
            pointer: "/0/delete/using_items/1/table/name/name",
            expected_value: json!("t2"),
        },
        ProjectionTestCase {
            description: "a DELETE USING only tables keeps its table list",
            sql: "DELETE FROM v1 USING t2 , t3 AS x ;",
            pointer: "/0/delete/using/1/alias/name",
            expected_value: json!("x"),
        },
        ProjectionTestCase {
            description: "FIRST on a join with no condition is the joined table's alias",
            sql: "SELECT 1 FROM t1 JOIN t2 FIRST ( v1 ) ;",
            pointer: "/0/select/joins/0/this/table/alias/name",
            expected_value: json!("FIRST"),
        },
        ProjectionTestCase {
            description: "the parenthesised name after a FIRST alias is a column alias",
            sql: "SELECT 1 FROM t1 JOIN t2 FIRST ( v1 ) ;",
            pointer: "/0/select/joins/0/this/table/column_aliases/0/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "FIRST after ON stays the read-order clause",
            sql: "SELECT 1 FROM t1 INNER DIRECTED JOIN t2 ON TRUE FIRST ( v1 ) ;",
            pointer: "/0/select/joins/0/first_last",
            expected_value: json!("FIRST"),
        },
        ProjectionTestCase {
            description: "a semantic view's qualified name is a table reference",
            sql: "SELECT * FROM SEMANTIC_VIEW ( d1 . v1 METRICS 1 AS v1 DIMENSIONS 1 v1 ) ;",
            pointer: "/0/select/from/expressions/0/semantic_view/this/table/schema/name",
            expected_value: json!("d1"),
        },
        ProjectionTestCase {
            description: "an explicit AS in a semantic view clause is recorded",
            sql: "SELECT * FROM SEMANTIC_VIEW ( d1 . v1 METRICS 1 AS v1 DIMENSIONS 1 v1 ) ;",
            pointer: "/0/select/from/expressions/0/semantic_view/metrics/tuple/expressions/0/alias/alias_explicit_as",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a bare alias in a semantic view clause has no AS",
            sql: "SELECT * FROM SEMANTIC_VIEW ( d1 . v1 METRICS 1 AS v1 DIMENSIONS 1 v1 ) ;",
            pointer: "/0/select/from/expressions/0/semantic_view/dimensions/tuple/expressions/0/alias/alias_explicit_as",
            expected_value: json!(false),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            statements(test_case.sql, Dialect::Snowflake, CallMode::Faithful)
                .pointer(test_case.pointer)
                .cloned()
                .unwrap_or_default(),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
