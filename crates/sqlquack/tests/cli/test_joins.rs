use crate::helpers::{convert_outcome, converted_first_value, run_convert};
use crate::test_types::{ConvertTestCase, ValueTestCase};

#[test]
fn given_join_without_on_when_converted_and_run_on_duckdb_then_it_is_a_cross_join() {
    let test_cases = [
        ValueTestCase {
            description: "JOIN without ON pairs every left row with every right row, as recorded",
            sql: "SELECT TO_VARCHAR(COUNT(*)) AS n FROM (SELECT 1 AS x UNION ALL SELECT 2) l \
                  JOIN (SELECT 1 AS y UNION ALL SELECT 2 UNION ALL SELECT 3) r",
            expected_value: "6",
        },
        ValueTestCase {
            description: "INNER JOIN without ON, its match left to WHERE, keeps only matching pairs",
            sql: "SELECT TO_VARCHAR(COUNT(*)) AS n FROM (SELECT 1 AS x UNION ALL SELECT 2) l \
                  INNER JOIN (SELECT 2 AS y UNION ALL SELECT 2 UNION ALL SELECT 3) r WHERE l.x = r.y",
            expected_value: "2",
        },
        ValueTestCase {
            description: "JOIN without ON against an empty side returns no rows",
            sql: "SELECT TO_VARCHAR(COUNT(*)) AS n FROM (SELECT 1 AS x) l \
                  JOIN (SELECT 1 AS y WHERE FALSE) r",
            expected_value: "0",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_first_value(test_case.sql),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_outer_join_without_on_when_converting_then_it_refuses() {
    let test_cases = [
        ConvertTestCase {
            description: "LEFT JOIN without ON, unrecorded against an empty side, refuses",
            sql: "SELECT l.x AS x FROM (SELECT 1 AS x) l LEFT JOIN (SELECT 1 AS y) r",
            flags: &[],
            mention: "no rule converts a Left join of this shape",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "FULL JOIN without ON, unrecorded against an empty side, refuses",
            sql: "SELECT l.x AS x FROM (SELECT 1 AS x) l FULL JOIN (SELECT 1 AS y) r",
            flags: &[],
            mention: "no rule converts a Full join of this shape",
            expected_outcome: convert_outcome(1, false),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_convert(test_case.sql, test_case.flags, test_case.mention),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
