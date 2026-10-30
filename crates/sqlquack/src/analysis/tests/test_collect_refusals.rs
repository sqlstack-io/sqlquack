use crate::analysis::tests::helpers::{collected_summary, converted_sql};
use crate::analysis::tests::test_types::{
    CollectedRefusalsTestCase, CollectedSummary, CollectionIsolationTestCase,
};

#[test]
fn given_independent_blockers_when_collecting_refusals_then_every_one_is_listed() {
    let test_cases = [
        CollectedRefusalsTestCase {
            description: "two refused functions in different select items",
            sql: "SELECT SOUNDEX(note) AS label, JAROWINKLER_SIMILARITY(note, 'gift') AS score \
                  FROM customers",
            expected_summary: CollectedSummary {
                constructs: vec![
                    "fn:JAROWINKLER_SIMILARITY".to_owned(),
                    "fn:SOUNDEX".to_owned(),
                ],
                possibly_incomplete: false,
            },
        },
        CollectedRefusalsTestCase {
            description: "an object star excluding a quoted name, as conversion refuses it",
            sql: "SELECT OBJECT_CONSTRUCT(c.* EXCLUDE (\"note\")) AS v FROM customers AS c",
            expected_summary: CollectedSummary {
                constructs: vec!["node:star".to_owned()],
                possibly_incomplete: false,
            },
        },
        CollectedRefusalsTestCase {
            description: "a refused function in the select list and another in WHERE",
            sql: "SELECT SOUNDEX(note) AS label FROM customers \
                  WHERE JAROWINKLER_SIMILARITY(note, 'gift') > 0.5",
            expected_summary: CollectedSummary {
                constructs: vec![
                    "fn:JAROWINKLER_SIMILARITY".to_owned(),
                    "fn:SOUNDEX".to_owned(),
                ],
                possibly_incomplete: false,
            },
        },
        CollectedRefusalsTestCase {
            description: "a refused function inside a typed parent: the cast keeps its target type",
            sql: "SELECT CAST(SOUNDEX(note) AS VARCHAR) || 'x' AS label, \
                  JAROWINKLER_SIMILARITY(note, 'gift') AS score FROM customers",
            expected_summary: CollectedSummary {
                constructs: vec![
                    "fn:JAROWINKLER_SIMILARITY".to_owned(),
                    "fn:SOUNDEX".to_owned(),
                ],
                possibly_incomplete: false,
            },
        },
        CollectedRefusalsTestCase {
            description: "an unknown relation, and a refused function over its columns",
            sql: "SELECT SOUNDEX(r.reason) AS reason, c.note FROM customers AS c \
                  JOIN returns AS r ON r.customer_id = c.customer_id",
            expected_summary: CollectedSummary {
                constructs: vec!["fn:SOUNDEX".to_owned(), "statement".to_owned()],
                possibly_incomplete: false,
            },
        },
        CollectedRefusalsTestCase {
            description: "a parent refused over an untyped placeholder may be a knock-on",
            sql: "SELECT MAX(r.amount) AS most FROM returns AS r",
            expected_summary: CollectedSummary {
                constructs: vec!["statement".to_owned()],
                possibly_incomplete: true,
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            collected_summary(test_case.sql),
            test_case.expected_summary,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_converting_statements_when_collecting_refusals_elsewhere_then_their_sql_is_unchanged() {
    let test_cases = [CollectionIsolationTestCase {
        description: "collecting a refused statement leaves later conversions byte-identical",
        converting: &[
            "SELECT customer_id, UPPER(note) AS note FROM customers WHERE customer_id > 10",
            "SELECT o.customer_id, SUM(o.amount) AS total FROM orders AS o \
             JOIN customers AS c ON c.customer_id = o.customer_id GROUP BY o.customer_id",
            "SELECT customer_id, CASE WHEN amount > 10 THEN 'big' ELSE 'small' END AS size \
             FROM orders ORDER BY customer_id",
        ],
        refused: "SELECT SOUNDEX(note) AS label, JAROWINKLER_SIMILARITY(note, 'gift') AS score \
                  FROM customers",
        expected_outcome: (3, 2, true),
    }];
    for test_case in test_cases {
        let before = converted_sql(test_case.converting);
        let collected = collected_summary(test_case.refused);
        let after = converted_sql(test_case.converting);
        assert_eq!(
            (
                before.iter().flatten().count(),
                collected.constructs.len(),
                before == after
            ),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
