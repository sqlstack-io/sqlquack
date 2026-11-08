use crate::format::tests::helpers::{native_bigquery_refusal, recorded_bigquery_rejections};
use crate::format::tests::test_types;

#[test]
fn given_recorded_bigquery_verdicts_when_parsing_natively_then_the_parser_also_refuses() {
    let test_cases = [
        test_types::BigQueryReceiptTestCase {
            description: "SELECT * FROM x.*",
            expected_rejection: ("Syntax error: Unexpected \"*\" at [1:17]", "1:17"),
        },
        test_types::BigQueryReceiptTestCase {
            description: "SELECT * FROM x.y*",
            expected_rejection: (
                "Syntax error: Expected end of input but got \"*\" at [1:18]",
                "1:18",
            ),
        },
        test_types::BigQueryReceiptTestCase {
            description: "SELECT 1 AS hash",
            expected_rejection: ("Syntax error: Unexpected keyword HASH at [1:13]", "1:13"),
        },
        test_types::BigQueryReceiptTestCase {
            description: "SELECT 1 AS at",
            expected_rejection: ("Syntax error: Unexpected keyword AT at [1:13]", "1:13"),
        },
        test_types::BigQueryReceiptTestCase {
            description: "WITH cte(c) AS (SELECT * FROM t) SELECT * FROM cte",
            expected_rejection: (
                "Syntax error: Expected \")\" but got identifier \"c\" at [1:10]",
                "1:10",
            ),
        },
        test_types::BigQueryReceiptTestCase {
            description: "SELECT * FROM t AS t(c1, c2)",
            expected_rejection: (
                "Syntax error: Expected end of input but got \"(\" at [1:21]",
                "1:21",
            ),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            (
                recorded_bigquery_rejections(test_case.description),
                native_bigquery_refusal(test_case.description)
            ),
            (
                vec![
                    Some((
                        test_case.expected_rejection.0.to_string(),
                        test_case.expected_rejection.1.to_string()
                    ));
                    2
                ],
                true
            ),
            "{}",
            test_case.description
        );
    }
}
