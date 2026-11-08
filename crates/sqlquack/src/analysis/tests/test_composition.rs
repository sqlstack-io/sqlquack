use std::time::Duration;

use crate::analysis::tests::helpers::{timed_conversion, unpacking_query};
use crate::analysis::tests::test_types::{CompositionTestCase, LargeLiteralTestCase};

/// The least base time growth is measured from, so tiny timings do not swing the ratio.
const TIMING_FLOOR: Duration = Duration::from_millis(50);

#[test]
fn given_composed_variant_text_when_converting_then_sql_and_time_grow_linearly() {
    let test_cases = [
        CompositionTestCase {
            description: "one column, text written through nested PARSE_JSON and TO_VARCHAR",
            columns: 1,
            depths: [1, 4, 8],
            expected_max_time_ratio: 16,
        },
        CompositionTestCase {
            description: "sixteen columns of flattened elements, as staging models unpack them",
            columns: 16,
            depths: [1, 2, 3],
            expected_max_time_ratio: 6,
        },
    ];
    for test_case in test_cases {
        let [(low, low_time), (middle, _), (high, high_time)] = test_case.depths.map(|depth| {
            let (sql, elapsed) = timed_conversion(&unpacking_query(test_case.columns, depth));
            let sql = sql.unwrap_or_else(|| panic!("{}: converts", test_case.description));
            (sql.len(), elapsed)
        });
        let [first, second, third] = test_case.depths;
        assert_eq!(
            (high - middle) * (second - first),
            (middle - low) * (third - second),
            "{}: each level must add the same SQL: {low}, {middle}, {high}",
            test_case.description
        );
        assert!(
            high_time <= low_time.max(TIMING_FLOOR) * test_case.expected_max_time_ratio,
            "{}: depth {third} took {high_time:?} against {low_time:?} at depth {first}",
            test_case.description
        );
    }
}

#[test]
fn given_large_literals_when_converting_then_time_stays_bounded() {
    let rows = |count: usize| vec!["(1)"; count].join(", ");
    let test_cases = [
        LargeLiteralTestCase {
            description: "an array literal of 250 elements converts",
            sql: format!("SELECT [{}] AS v", vec!["1"; 250].join(", ")),
            expected_converted: true,
            expected_budget_seconds: 15,
        },
        LargeLiteralTestCase {
            description: "VALUES of 1,000 rows converts",
            sql: format!("SELECT x FROM (VALUES {}) AS t(x)", rows(1000)),
            expected_converted: true,
            expected_budget_seconds: 15,
        },
        LargeLiteralTestCase {
            description: "4,000 unaliased DATE literals after a line comment, half one value, convert",
            sql: format!(
                "-- dates\nSELECT {}, {}",
                (1000..3000)
                    .map(|year| format!("DATE '{year}-01-01'"))
                    .collect::<Vec<String>>()
                    .join(", "),
                vec!["DATE '2024-01-01'"; 2000].join(", ")
            ),
            expected_converted: true,
            expected_budget_seconds: 15,
        },
        LargeLiteralTestCase {
            description: "VALUES of 100,000 rows refuses, not overflowing the stack",
            sql: format!("SELECT x FROM (VALUES {}) AS t(x)", rows(100_000)),
            expected_converted: false,
            expected_budget_seconds: 15,
        },
    ];
    for test_case in test_cases {
        let (converted, elapsed) = timed_conversion(&test_case.sql);
        assert_eq!(
            converted.is_some(),
            test_case.expected_converted,
            "{}",
            test_case.description
        );
        assert!(
            elapsed < Duration::from_secs(test_case.expected_budget_seconds),
            "{}: took {elapsed:?}",
            test_case.description
        );
    }
}
