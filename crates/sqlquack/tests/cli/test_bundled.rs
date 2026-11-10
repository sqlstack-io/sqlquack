use crate::helpers::{convert_outcome, run_convert};
use crate::test_types::{ConvertTestCase, ValueTestCase};

// The distributions workflow also runs this test and the full convert test in test_convert.rs with
// DuckDB compiled in (`--features bundled`), as the wheels build it.
#[test]
fn given_zone_free_snowflake_sql_when_converting_then_the_checking_engine_binds_it() {
    let test_cases = [
        ConvertTestCase {
            description: "a constant converts once the checking engine binds it",
            sql: "SELECT 1 AS total",
            flags: &[],
            mention: "sqlquack convert: done, converted\n",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a VARIANT path converts with DuckDB's json functions",
            sql: "SELECT PARSE_JSON('{\"total\": 3}'):total AS total",
            flags: &[],
            mention: "sqlquack convert: done, converted (1 known difference allowed: json_fidelity",
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

// DuckDB built from its sources reports a dev version unless the build names the release, and the
// Python replay (`PINNED_DUCKDB_VERSION` in sqlquack_runtime.py) and extension downloads then refuse
// it. The distributions workflow runs this with DuckDB compiled in as well.
#[test]
fn given_the_linked_duckdb_when_asked_its_version_then_it_names_the_pinned_release() {
    let test_cases = [
        ValueTestCase {
            description: "version() names the release, not a dev build",
            sql: "SELECT version()",
            expected_value: "v2.0.0",
        },
        ValueTestCase {
            description: "pragma_version's library version names the same release",
            sql: "SELECT library_version FROM pragma_version()",
            expected_value: "v2.0.0",
        },
    ];
    let connection = duckdb::Connection::open_in_memory().expect("in-memory DuckDB opens");
    for test_case in test_cases {
        let version: String = connection
            .query_row(test_case.sql, [], |row| row.get(0))
            .expect(test_case.description);
        assert_eq!(
            version, test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
