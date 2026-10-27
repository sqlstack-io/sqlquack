use serde_json::json;

use crate::parser::tests::helpers::parse_error;
use crate::parser::tests::helpers::present_at;
use crate::parser::tests::helpers::value_at;
use crate::parser::tests::test_types::ParseErrorTestCase;
use crate::parser::tests::test_types::ParsePresenceTestCase;
use crate::parser::tests::test_types::ParseValueTestCase;
use crate::tokens::models::DialectType;

#[test]
fn given_duckdb_keyword_relation_aliases_when_parsing_then_clause_boundaries_hold() {
    let test_cases = [
        ParseValueTestCase {
            description: "a select without an alias orders NULLS FIRST first",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/order_by/expressions/0/nulls_first",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "a select without an alias orders NULLS LAST second",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/order_by/expressions/1/nulls_first",
            expected_value: Ok(json!(false)),
        },
        ParseValueTestCase {
            description: "a select without an alias has two orderings",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/order_by/expressions/2",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a select without an alias keeps the table alias",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/from/expressions/0/table/alias",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a left join after a table without an alias is one join",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items LEFT JOIN selected_items ON TRUE",
            pointer: "/0/select/joins/1",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a left join after a table without an alias keeps its kind",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items LEFT JOIN selected_items ON TRUE",
            pointer: "/0/select/joins/0/kind",
            expected_value: Ok(json!("Left")),
        },
        ParseValueTestCase {
            description: "a left join after a table without an alias keeps the table alias",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items LEFT JOIN selected_items ON TRUE",
            pointer: "/0/select/from/expressions/0/table/alias",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a positional join after a table without an alias is one join",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items POSITIONAL JOIN selected_items",
            pointer: "/0/select/joins/1",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a positional join after a table without an alias keeps its kind",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items POSITIONAL JOIN selected_items",
            pointer: "/0/select/joins/0/kind",
            expected_value: Ok(json!("Positional")),
        },
        ParseValueTestCase {
            description: "a positional join after a table without an alias keeps the table alias",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items POSITIONAL JOIN selected_items",
            pointer: "/0/select/from/expressions/0/table/alias",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a select aliased top orders NULLS FIRST first",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/order_by/expressions/0/nulls_first",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "a select aliased top orders NULLS LAST second",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/order_by/expressions/1/nulls_first",
            expected_value: Ok(json!(false)),
        },
        ParseValueTestCase {
            description: "a select aliased top has two orderings",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/order_by/expressions/2",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a select aliased top keeps the table alias",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_value: Ok(json!("top")),
        },
        ParseValueTestCase {
            description: "a left join after a table aliased top is one join",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items top LEFT JOIN selected_items ON TRUE",
            pointer: "/0/select/joins/1",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a left join after a table aliased top keeps its kind",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items top LEFT JOIN selected_items ON TRUE",
            pointer: "/0/select/joins/0/kind",
            expected_value: Ok(json!("Left")),
        },
        ParseValueTestCase {
            description: "a left join after a table aliased top keeps the table alias",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items top LEFT JOIN selected_items ON TRUE",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_value: Ok(json!("top")),
        },
        ParseValueTestCase {
            description: "a positional join after a table aliased top is one join",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items top POSITIONAL JOIN selected_items",
            pointer: "/0/select/joins/1",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "a positional join after a table aliased top keeps its kind",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items top POSITIONAL JOIN selected_items",
            pointer: "/0/select/joins/0/kind",
            expected_value: Ok(json!("Positional")),
        },
        ParseValueTestCase {
            description: "a positional join after a table aliased top keeps the table alias",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items top POSITIONAL JOIN selected_items",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_value: Ok(json!("top")),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            value_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_reserved_words_as_relation_aliases_when_parsing_duckdb_then_parsing_fails() {
    let test_cases = [
        ParseErrorTestCase {
            description: "end cannot alias a relation",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items end",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "where cannot alias a relation",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items where",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "qualify cannot alias a relation",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items qualify",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "select cannot alias a relation",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items select",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "join cannot alias a relation",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items join",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "limit cannot alias a relation",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT * FROM ranked_items limit",
            expected_error: "Parse error",
        },
    ];

    for test_case in test_cases {
        let error = parse_error(test_case.dialect, test_case.sql);
        assert!(
            error.contains(test_case.expected_error),
            "{}: {error}",
            test_case.description
        );
    }
}

#[test]
fn given_duckdb_keyword_relation_aliases_when_parsing_then_every_clause_is_kept() {
    let test_cases = [
        ParsePresenceTestCase {
            description: "a select without an alias keeps its where",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/where_clause",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select without an alias keeps its group by",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/group_by",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select without an alias keeps its having",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/having",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select without an alias keeps its windows",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/windows",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select without an alias keeps its qualify",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/qualify",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select without an alias keeps its limit",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/limit",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select without an alias keeps its offset",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/offset",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select aliased top keeps its where",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/where_clause",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select aliased top keeps its group by",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/group_by",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select aliased top keeps its having",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/having",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select aliased top keeps its windows",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/windows",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select aliased top keeps its qualify",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/qualify",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select aliased top keeps its limit",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/limit",
            expected_present: Ok(true),
        },
        ParsePresenceTestCase {
            description: "a select aliased top keeps its offset",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT item_id, ROW_NUMBER() OVER w AS rn FROM ranked_items top WHERE item_id > 0 GROUP BY item_id HAVING COUNT(*) > 0 WINDOW w AS (ORDER BY item_id) QUALIFY rn = 1 ORDER BY item_id NULLS FIRST, rn NULLS LAST LIMIT 5 OFFSET 1",
            pointer: "/0/select/offset",
            expected_present: Ok(true),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            present_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_present,
            "{}",
            test_case.description
        );
    }
}
