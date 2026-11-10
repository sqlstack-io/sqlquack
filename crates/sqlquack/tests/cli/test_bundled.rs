use crate::helpers::{convert_outcome, run_convert};
use crate::test_types::ConvertTestCase;

// The distributions workflow also runs this test and the full convert test in test_convert.rs with
// DuckDB compiled in (`--features bundled`), as the wheels build it.
#[test]
fn given_zone_free_snowflake_sql_when_converting_then_the_checking_engine_binds_it() {
    let test_cases = [
        ConvertTestCase {
            description: "a constant converts once the checking engine binds it",
            sql: "SELECT 1 AS total",
            flags: &[],
            mention: "sqlquack convert: done, converted under utc-v1",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a VARIANT path converts with DuckDB's json functions",
            sql: "SELECT PARSE_JSON('{\"total\": 3}'):total AS total",
            flags: &[],
            mention: "sqlquack convert: done, converted under utc-v1",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a statement refused for its own reason still names it",
            sql: "WITH totals AS (SELECT 1 AS \"orderTotal\") SELECT ordertotal FROM totals",
            flags: &[],
            mention: " --> <stdin>:1:",
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
