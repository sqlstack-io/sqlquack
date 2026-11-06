use crate::refusals::constants::SNOWFLAKE_FUNCTION_ARITIES;
use crate::refusals::models::RefusalCategory;
use crate::refusals::tests::helpers::{
    catalogue_function_arities, not_expressible_reasons, rejection_codes, rejection_names,
    unmatched_refusals, unparsed_refusal,
};
use crate::refusals::tests::test_types::{
    ArityListTestCase, ConstructRefusalTestCase, RejectionTestCase, UnparsedTestCase,
};

#[test]
fn given_catalogue_when_refusing_constructs_then_every_occurrence_gets_its_own_refusal() {
    let test_cases = [
        ConstructRefusalTestCase {
            description: "an empty catalogue refuses everything as a missing rule",
            sql: "SELECT c.customer_id, COUNT(o.order_id) FROM customers c \
                  JOIN orders o ON o.customer_id = c.customer_id GROUP BY c.customer_id",
            inexpressible: &[],
            expected_unmatched_refusals: 0,
            expected_not_expressible_reasons: Vec::new(),
        },
        ConstructRefusalTestCase {
            description: "a declared construct is refused with the declared reason",
            sql: "SELECT RANDOM() FROM products",
            inexpressible: &["fn:RANDOM"],
            expected_unmatched_refusals: 0,
            expected_not_expressible_reasons: vec![
                "fn:RANDOM is declared inexpressible for this test".to_owned(),
            ],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            unmatched_refusals(test_case.sql, test_case.inexpressible),
            test_case.expected_unmatched_refusals,
            "{}",
            test_case.description
        );
        assert_eq!(
            not_expressible_reasons(test_case.sql, test_case.inexpressible),
            test_case.expected_not_expressible_reasons,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unparseable_sql_when_refusing_then_reports_parse_fidelity_with_location() {
    let test_cases = [UnparsedTestCase {
        description: "a SELECT with no projection",
        sql: "SELECT FROM WHERE",
        expected_category: RefusalCategory::ParseFidelity,
        expected_located: true,
    }];

    for test_case in test_cases {
        let refusal = unparsed_refusal(test_case.sql);
        assert_eq!(
            refusal.category, test_case.expected_category,
            "{}",
            test_case.description
        );
        assert_eq!(
            refusal.location.is_some(),
            test_case.expected_located,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_snowflake_parses_then_rejects_when_refusing_then_each_rejection_is_named() {
    let test_cases = [
        RejectionTestCase {
            description: "INTERSECT ALL and MINUS ALL are rejected, UNION ALL and plain INTERSECT are not",
            sql: "SELECT 1 INTERSECT ALL SELECT 2; SELECT 1 MINUS ALL SELECT 2; \
                  SELECT 1 UNION ALL SELECT 2; SELECT 1 INTERSECT SELECT 2",
            expected_rejections: vec!["set_operation_all", "set_operation_all"],
            expected_codes: vec!["SQ0201", "SQ0201"],
        },
        RejectionTestCase {
            description: "FOR UPDATE WAIT is rejected, a plain FOR UPDATE is left to conversion",
            sql: "SELECT order_id FROM orders FOR UPDATE WAIT 7; \
                  SELECT order_id FROM orders FOR UPDATE",
            expected_rejections: vec!["lock_wait"],
            expected_codes: vec!["SQ0202"],
        },
        RejectionTestCase {
            description: "QUALIFY needs a window in the SELECT list or QUALIFY, not one in a subquery",
            sql: "SELECT order_id FROM orders QUALIFY order_id > 3; \
                  SELECT order_id FROM orders QUALIFY ROW_NUMBER() OVER (ORDER BY order_id) = 1; \
                  SELECT order_id, RANK() OVER (ORDER BY price) AS r FROM orders QUALIFY r = 1; \
                  SELECT order_id FROM orders \
                  QUALIFY (SELECT MAX(price) OVER () FROM orders LIMIT 1) > 1",
            expected_rejections: vec!["qualify_without_window", "qualify_without_window"],
            expected_codes: vec!["SQ0203", "SQ0203"],
        },
        RejectionTestCase {
            description: "REPLACE of a column ILIKE removes is rejected; one it keeps, in any case, is not",
            sql: "SELECT * ILIKE '%_id' REPLACE (price * 2 AS price) FROM orders; \
                  SELECT * ILIKE '%_id' REPLACE (order_id + 1 AS \"Order_Id\") FROM orders",
            expected_rejections: vec!["star_filter_drops_named_column"],
            expected_codes: vec!["SQ0204"],
        },
        RejectionTestCase {
            description: "RENAME is checked too, and `_` matches exactly one character",
            sql: "SELECT * ILIKE 'price_' RENAME (price AS amount) FROM orders; \
                  SELECT * ILIKE '_rder%' RENAME (order_id AS id) FROM orders; \
                  SELECT orders.* ILIKE 'o%' REPLACE (customer_id AS customer_id) FROM orders",
            expected_rejections: vec![
                "star_filter_drops_named_column",
                "star_filter_drops_named_column",
            ],
            expected_codes: vec!["SQ0204", "SQ0204"],
        },
        RejectionTestCase {
            description: "two FROM items under one alias are rejected, a quoted upper-case one too",
            sql: "SELECT order_id FROM orders o, orders o; \
                  SELECT 1 FROM orders o JOIN (SELECT 1 AS a) \"O\" ON TRUE",
            expected_rejections: vec!["duplicate_alias", "duplicate_alias"],
            expected_codes: vec!["SQ0206", "SQ0206"],
        },
        RejectionTestCase {
            description: "distinct aliases, a quoted lower-case one, unaliased items and nested scopes are not",
            sql: "SELECT 1 FROM orders o, orders p; \
                  SELECT 1 FROM orders o JOIN orders \"o\" ON TRUE; \
                  SELECT 1 FROM orders JOIN orders ON TRUE; \
                  SELECT 1 FROM orders o WHERE EXISTS (SELECT 1 FROM orders o)",
            expected_rejections: vec![],
            expected_codes: vec![],
        },
        RejectionTestCase {
            description: "a call with more arguments than any signature takes is rejected",
            sql: "SELECT LENGTH(order_id, 2) AS x FROM orders; \
                  SELECT hex_encode(status, 1, 2) FROM orders",
            expected_rejections: vec!["too_many_arguments", "too_many_arguments"],
            expected_codes: vec!["SQ0207", "SQ0207"],
        },
        RejectionTestCase {
            description: "calls within their limits, with named arguments, unbounded or unknown are not",
            sql: "SELECT LENGTH(order_id), SUBSTR(status, 1, 2), COALESCE(a, b, c, d, e) FROM orders; \
                  SELECT AI_EXTRACT(file => f, responseFormat => r, scores => TRUE) FROM orders; \
                  SELECT NOSUCHFUNCTION(1, 2, 3, 4) FROM orders",
            expected_rejections: vec![],
            expected_codes: vec![],
        },
        RejectionTestCase {
            description: "a column a lone PIVOT aggregates or pivots on is rejected when read after it",
            sql: "SELECT order_id, total FROM orders PIVOT (SUM(total) FOR status IN ('a', 'b')) AS p; \
                  SELECT order_id FROM orders PIVOT (SUM(total) FOR status IN ('a')) AS p WHERE p.status = 'a'",
            expected_rejections: vec!["pivot_consumed_column", "pivot_consumed_column"],
            expected_codes: vec!["SQ0208", "SQ0208"],
        },
        RejectionTestCase {
            description: "pivoted values, aliases, joins and nested queries are not",
            sql: "SELECT order_id, \"'a'\" FROM orders PIVOT (SUM(total) FOR status IN ('a')) AS p; \
                  SELECT order_id AS total FROM orders PIVOT (SUM(total) FOR status IN ('a')) AS p ORDER BY total; \
                  SELECT total FROM orders PIVOT (SUM(total) FOR status IN ('a')) AS p JOIN totals ON TRUE; \
                  SELECT (SELECT total FROM totals) FROM orders PIVOT (SUM(total) FOR status IN ('a')) AS p",
            expected_rejections: vec![],
            expected_codes: vec![],
        },
        RejectionTestCase {
            description: "a lateral table function joined with an ON condition is rejected",
            sql: "SELECT 1 FROM events e INNER JOIN LATERAL FLATTEN(input => e.payload) f ON TRUE; \
                  SELECT 1 FROM events e LEFT JOIN LATERAL FLATTEN(input => e.payload) f ON f.index = 0",
            expected_rejections: vec![
                "lateral_function_join_condition",
                "lateral_function_join_condition",
            ],
            expected_codes: vec!["SQ0209", "SQ0209"],
        },
        RejectionTestCase {
            description: "a lateral table function joined without ON, or a lateral subquery with ON, is not",
            sql: "SELECT 1 FROM events e JOIN LATERAL FLATTEN(input => e.payload) f; \
                  SELECT 1 FROM events e, LATERAL FLATTEN(input => e.payload) f; \
                  SELECT 1 FROM events e JOIN LATERAL (SELECT 1 AS a) s ON TRUE",
            expected_rejections: vec![],
            expected_codes: vec![],
        },
        RejectionTestCase {
            description: "a star without ILIKE, or ILIKE without REPLACE or RENAME, is left to conversion",
            sql: "SELECT * REPLACE (price * 2 AS price) FROM orders; \
                  SELECT * ILIKE '%_id' FROM orders",
            expected_rejections: vec![],
            expected_codes: vec![],
        },
        RejectionTestCase {
            description: "a rejection nested in a CTE is found",
            sql: "WITH both_ids AS (SELECT 1 AS id EXCEPT ALL SELECT 2) SELECT id FROM both_ids",
            expected_rejections: vec!["set_operation_all"],

            expected_codes: vec!["SQ0201"],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            rejection_names(test_case.sql),
            test_case.expected_rejections,
            "{}",
            test_case.description
        );
        assert_eq!(
            rejection_codes(test_case.sql),
            test_case.expected_codes,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_the_syntax_catalogue_when_listing_function_arities_then_the_list_matches_its_grammars() {
    let test_cases = [ArityListTestCase {
        description: "snowflake_function_arities.txt holds each function every call grammar bounds",
        expected_arities: SNOWFLAKE_FUNCTION_ARITIES,
    }];
    for test_case in test_cases {
        assert_eq!(
            catalogue_function_arities(),
            test_case.expected_arities,
            "{}",
            test_case.description
        );
    }
}
