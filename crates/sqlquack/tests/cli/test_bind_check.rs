use crate::helpers::convert_streams;
use crate::test_types::BindCheckTestCase;

#[test]
fn given_the_bind_check_off_when_converting_then_the_output_is_the_same() {
    let test_cases = [
        BindCheckTestCase {
            description: "a converted query",
            sql: "WITH orders AS (SELECT 1 AS a UNION ALL SELECT 2) SELECT a + 1 AS b FROM orders",
            expected_converted: true,
        },
        BindCheckTestCase {
            description: "a query refused for an undeclared relation",
            sql: "SELECT a FROM customers",
            expected_converted: false,
        },
        BindCheckTestCase {
            description: "a query refused for a function SQLQuack cannot convert",
            sql: "SELECT DATE_PART('epoch_second', 'x') AS a",
            expected_converted: false,
        },
    ];
    for test_case in test_cases {
        let checked = convert_streams(test_case.sql, &["--no-config", "--compact"]);

        let unchecked = convert_streams(
            test_case.sql,
            &["--no-config", "--compact", "--no-bind-check"],
        );

        assert_eq!(unchecked, checked, "{}", test_case.description);
        assert_eq!(
            !checked.0.trim().is_empty(),
            test_case.expected_converted,
            "{}: {checked:?}",
            test_case.description
        );
    }
}
