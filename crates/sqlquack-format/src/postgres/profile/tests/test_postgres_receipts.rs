use crate::postgres::profile::tests::helpers::{
    native_postgres_rejection, recorded_postgres_rejections,
};
use crate::postgres::profile::tests::test_types;

#[test]
fn given_recorded_postgres_verdicts_when_parsing_natively_then_the_parser_agrees() {
    let test_cases = [
        test_types::PostgresReceiptTestCase {
            description: "SELECT * FROM foo WHERE id = ?",
            expected_rejection: Some(("syntax error at end of input", 31)),
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT * FROM foo WHERE id = %s",
            expected_rejection: Some(("syntax error at or near \"%\"", 30)),
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT * FROM foo WHERE id = %(id_param)s",
            expected_rejection: Some(("syntax error at or near \"%\"", 30)),
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT a, LOGICAL_OR(b) FROM table GROUP BY a",
            expected_rejection: Some(("syntax error at or near \"table\"", 30)),
        },
        test_types::PostgresReceiptTestCase {
            description: "select count() OVER(partition by a order by a range offset preceding exclude current row)",
            expected_rejection: Some(("syntax error at or near \"offset\"", 53)),
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT js, js IS JSON ARRAY WITH UNIQUE KEYS AS \"array w. UK?\", js IS JSON ARRAY WITHOUT UNIQUE KEYS AS \"array w/o UK?\", js IS JSON ARRAY UNIQUE KEYS AS \"array w UK 2?\" FROM t",
            expected_rejection: Some(("syntax error at or near \"KEYS\"", 146)),
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT js IS JSON ARRAY UNIQUE KEYS FROM t",
            expected_rejection: Some(("syntax error at or near \"KEYS\"", 32)),
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT date_col - INTERVAL '30' DAY FROM t",
            expected_rejection: None,
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT date_col - INTERVAL '1' HOUR AS one_hour_later",
            expected_rejection: None,
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT date_add(current_date, interval '7' day)",
            expected_rejection: None,
        },
        test_types::PostgresReceiptTestCase {
            description: "SELECT MLEAST(VARIADIC ARRAY[]::numeric[])",
            expected_rejection: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            (
                recorded_postgres_rejections(test_case.description),
                native_postgres_rejection(test_case.description)
            ),
            (
                vec![
                    test_case
                        .expected_rejection
                        .map(|(message, position)| (message.to_string(), position));
                    2
                ],
                test_case
                    .expected_rejection
                    .map(|(message, position)| (message.to_string(), position))
            ),
            "{}",
            test_case.description
        );
    }
}
