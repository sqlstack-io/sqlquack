use crate::refusals::models::RefusalCategory;
use crate::refusals::tests::helpers::{
    not_expressible_reasons, rejection_codes, rejection_names, unmatched_refusals, unparsed_refusal,
};
use crate::refusals::tests::test_types::{
    ConstructRefusalTestCase, RejectionTestCase, UnparsedTestCase,
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
            description: "a star without ILIKE, or ILIKE without REPLACE or RENAME, is left to conversion",
            sql: "SELECT * REPLACE (price * 2 AS price) FROM orders; \
                  SELECT * ILIKE '%_id' FROM orders",
            expected_rejections: vec![],
            expected_codes: vec![],
        },
        RejectionTestCase {
            description: "a date-time type with a precision before a string is rejected; a cast or a plain TIME or TIMESTAMP literal is not",
            sql: "SELECT TIMESTAMP_NTZ(3) '2026-01-02 03:04:05' AS v; \
                  SELECT timestamp_tz(3) '2026-01-02 03:04:05' AS v; \
                  SELECT '2026-01-02 03:04:05'::TIMESTAMP_NTZ(3) AS v; \
                  SELECT CAST('2026-01-02' AS TIMESTAMP_TZ(3)) AS v; \
                  SELECT CAST('2026-01-02' AS DATETIME(3)) AS v; \
                  SELECT CAST('2026-01-02' AS TIMESTAMPNTZ(3)) AS v; \
                  SELECT CAST('2026-01-02' AS TIMESTAMP_LTZ(3)) AS v; \
                  SELECT TRY_CAST('2026-01-02' AS TIMESTAMPLTZ(3)) AS v; \
                  SELECT DATETIME(3) '2026-01-02 03:04:05' AS v; \
                  SELECT TIMESTAMP(3) '2026-01-02 03:04:05' AS v; \
                  SELECT time (3) '03:04:05' AS v; \
                  SELECT CAST('2026-01-02' AS TIMESTAMP(3)) AS v; \
                  SELECT '03:04:05'::TIME(3) AS v; SELECT TIME '03:04:05' AS v; \
                  SELECT TIMESTAMP '2026-01-02 03:04:05' AS v; SELECT v1 'm' AS v",
            expected_rejections: vec!["unsupported_type_literal"; 5],
            expected_codes: vec!["SQ0211"; 5],
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
