use crate::reparse::tests::helpers::{printer_failure, printer_path, printer_report};
use crate::reparse::tests::test_types::PrinterTestCase;

#[test]
fn given_intended_duckdb_tree_when_printing_then_refuses_any_tree_change() {
    let test_cases = [
        PrinterTestCase {
            description: "a grouped query with explicit NULL ordering prints faithfully",
            duckdb_sql: "SELECT customer_id, COUNT(*) AS orders FROM orders GROUP BY customer_id \
                         ORDER BY orders DESC NULLS LAST",
            expected_failure: None,
            expected_difference_path: None,
        },
        PrinterTestCase {
            description: "dayofweek prints as ISODOW, which numbers days differently",
            duckdb_sql: "SELECT dayofweek(shipped_on) FROM orders",
            expected_failure: Some("changed"),
            expected_difference_path: Some(".select.expressions[0].day_of_week".to_owned()),
        },
        PrinterTestCase {
            description: "the regex match operator prints as a function DuckDB lacks",
            duckdb_sql: "SELECT sku ~ '^A' FROM products",
            expected_failure: Some("changed"),
            expected_difference_path: Some(".select.expressions[0].function".to_owned()),
        },
    ];

    for test_case in test_cases {
        let report = printer_report(test_case.duckdb_sql);
        assert_eq!(
            printer_failure(&report),
            test_case.expected_failure,
            "{}",
            test_case.description
        );
        assert_eq!(
            printer_path(&report),
            test_case.expected_difference_path,
            "{}",
            test_case.description
        );
    }
}
