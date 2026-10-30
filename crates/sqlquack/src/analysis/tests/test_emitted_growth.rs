use crate::analysis::tests::helpers::converted_sql;
use crate::analysis::tests::test_types::EmittedGrowthTestCase;

/// How SPLIT_PART binds its text once; DuckDB's split_part macro names its text twice.
const BOUND_TEXT: &str = "lambda part_text:";

#[test]
fn given_split_part_over_long_text_when_converting_then_the_text_is_bound_once() {
    let test_cases = [
        EmittedGrowthTestCase {
            description: "SPLIT_PART three deep over long converted text binds it at each level",
            sql: "SELECT SPLIT_PART(SPLIT_PART(SPLIT_PART(UPPER(note), ' ', 1), '-', 2), \
                  '/', 0) AS v FROM customers",
            expected_bindings: 3,
        },
        EmittedGrowthTestCase {
            description: "SPLIT_PART of a column, short enough to name twice, is not bound",
            sql: "SELECT SPLIT_PART(note, ' ', 1) AS v FROM customers",
            expected_bindings: 0,
        },
    ];
    for test_case in test_cases {
        let sql = converted_sql(&[test_case.sql])[0]
            .clone()
            .unwrap_or_default();
        assert_eq!(
            (sql.is_empty(), sql.matches(BOUND_TEXT).count()),
            (false, test_case.expected_bindings),
            "{}",
            test_case.description
        );
    }
}
