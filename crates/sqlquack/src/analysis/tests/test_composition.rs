use std::time::Duration;

use crate::analysis::tests::helpers::{timed_conversion, unpacking_query};
use crate::analysis::tests::test_types::CompositionTestCase;

#[test]
fn given_composed_variant_text_when_converting_then_sql_and_time_grow_linearly() {
    let test_cases = [
        CompositionTestCase {
            description: "one column, text written through nested PARSE_JSON and TO_VARCHAR",
            columns: 1,
            depths: [1, 4, 8],
            expected_budget_seconds: 2,
        },
        CompositionTestCase {
            description: "sixteen columns of flattened elements, as staging models unpack them",
            columns: 16,
            depths: [1, 2, 3],
            expected_budget_seconds: 8,
        },
    ];
    for test_case in test_cases {
        let [low, middle, high] = test_case.depths.map(|depth| {
            let (sql, elapsed) = timed_conversion(&unpacking_query(test_case.columns, depth));
            let sql = sql.unwrap_or_else(|| panic!("{}: converts", test_case.description));
            assert!(
                elapsed < Duration::from_secs(test_case.expected_budget_seconds),
                "{}: depth {depth} took {elapsed:?}",
                test_case.description
            );
            sql.len()
        });
        let [first, second, third] = test_case.depths;
        assert_eq!(
            (high - middle) * (second - first),
            (middle - low) * (third - second),
            "{}: each level must add the same SQL: {low}, {middle}, {high}",
            test_case.description
        );
    }
}
