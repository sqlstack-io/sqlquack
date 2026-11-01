use crate::helpers::{
    convert_outcome, converted_column_names, converted_first_value, converted_run_error,
    policy_free_exit_code, run_convert,
};
use crate::test_types::{
    ColumnNameTestCase, ConvertOutcome, ConvertTestCase, ConvertedRunTestCase, PolicyFreeTestCase,
    ValueTestCase,
};

const QUOTED_ALIAS: &str =
    "WITH totals AS (SELECT 1 AS \"orderTotal\") SELECT ordertotal FROM totals";
const PLACED_DATE: &str = "SELECT TO_DATE(placed_at) AS placed_on FROM orders";
const DECLARED_SCHEMA: &str = "tests/cli/fixtures/declared_offsets.json";
const NAIVE_TO_ZONED: &str =
    "SELECT '2024-03-10 22:30:00'::TIMESTAMP_NTZ(6)::TIMESTAMP_TZ AS placed_at";

#[test]
fn given_snowflake_sql_when_converting_then_duckdb_sql_or_named_refusals_are_printed() {
    let test_cases = [
        ConvertTestCase {
            description: "a TIMESTAMP_NTZ(3) compared with a text column refuses: only NTZ(9) is recorded",
            sql: "WITH t AS (SELECT '2024-01-01 00:00:00.123'::TIMESTAMP_NTZ(3) AS placed_at, \
                  '2024-01-01 00:00:00.1239' AS placed_text) SELECT placed_at = placed_text AS same FROM t",
            flags: &[],
            mention: "comparing TIMESTAMP_NTZ(3) with a VARCHAR column is recorded only for TIMESTAMP_NTZ(9)",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a quantifier straight after a lazy quantifier refuses",
            sql: "SELECT REGEXP_COUNT('xaxax', '(xa)+??x') AS matches",
            flags: &[],
            mention: "a quantifier directly after a lazy quantifier",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "LAG IGNORE NULLS with an offset of 0 refuses",
            sql: "WITH t AS (SELECT 1 AS order_id, 2 AS quantity) \
                  SELECT LAG(quantity, 0) IGNORE NULLS OVER (ORDER BY order_id) AS previous FROM t",
            flags: &[],
            mention: "IGNORE NULLS is recorded only with an offset of 1 or more",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a statement that converts prints DuckDB SQL and exits 0",
            sql: "SELECT 1 + 2 AS total",
            flags: &[],
            mention: "sqlquack convert: done, converted under utc-v1",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a subquery in a hierarchical query's projection is refused",
            sql: "WITH bins AS (SELECT 1 AS id, NULL AS parent) SELECT id, (SELECT 100) AS m FROM bins START WITH parent IS NULL CONNECT BY parent = PRIOR id",
            flags: &[],
            mention: "in a hierarchical query's projection or ordering",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a refused statement prints no SQL, names the refusal and exits 1",
            sql: QUOTED_ALIAS,
            flags: &[],
            mention: "refused statement",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "with quoted identifiers ignoring case it converts under utc-qiic-v1",
            sql: QUOTED_ALIAS,
            flags: &["--session", "QUOTED_IDENTIFIERS_IGNORE_CASE=TRUE"],
            mention: "converted under utc-qiic-v1",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a quoted identifier with a non-ASCII letter is refused when case is ignored",
            sql: "SELECT 1 AS \"caf\u{e9}\"",
            flags: &["--session", "QUOTED_IDENTIFIERS_IGNORE_CASE=true"],
            mention: "non-ASCII character",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "session parameters matching no recorded profile are refused by name",
            sql: "SELECT 1 AS total",
            flags: &["--session", "WEEK_START=1"],
            mention: "match no recorded session profile: WEEK_START = 1",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a session-dependent statement converts naming its assumption",
            sql: NAIVE_TO_ZONED,
            flags: &[],
            mention: "relies on assumption session_timezone_utc at node:cast",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "strict mode refuses it",
            sql: NAIVE_TO_ZONED,
            flags: &["--strict"],
            mention: "session_timezone_utc",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "strict mode converts it once --utc-session confirms UTC sessions",
            sql: NAIVE_TO_ZONED,
            flags: &["--strict", "--utc-session"],
            mention: "relies on assumption session_timezone_utc",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a non-UTC TIMEZONE refuses a zone-dependent statement despite --utc-session",
            sql: NAIVE_TO_ZONED,
            flags: &[
                "--strict",
                "--utc-session",
                "--session",
                "TIMEZONE=America/New_York",
            ],
            mention: "TIMEZONE = America/New_York is not UTC",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a declared-UTC TIMESTAMP_TZ column's date converts naming its assumption",
            sql: PLACED_DATE,
            flags: &["--schema", DECLARED_SCHEMA],
            mention: "relies on assumption declared_offset:ORDERS.PLACED_AT at fn:TO_DATE",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a TIMESTAMP_TZ column without a declared offset keeps refusing",
            sql: "SELECT TO_DATE(shipped_at) AS shipped_on FROM orders",
            flags: &["--schema", DECLARED_SCHEMA],
            mention: "casting TIMESTAMP_TZ(9) to DATE has no proven conversion",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "the declared offset carries through a CTE and COALESCE to DATE_TRUNC",
            sql: "WITH latest AS (SELECT COALESCE(placed_at, CONVERT_TIMEZONE('UTC', shipped_at)) AS at FROM orders) SELECT DATE_TRUNC('day', at) AS day FROM latest",
            flags: &["--schema", DECLARED_SCHEMA],
            mention: "relies on assumption declared_offset:ORDERS.PLACED_AT at fn:DATE_TRUNC",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "strict mode refuses relying on a declared offset",
            sql: PLACED_DATE,
            flags: &["--schema", DECLARED_SCHEMA, "--strict"],
            mention: "assumption `declared_offset`, which the policy does not allow in this mode",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "strict mode converts it once --confirm-declared-offsets confirms them",
            sql: PLACED_DATE,
            flags: &[
                "--schema",
                DECLARED_SCHEMA,
                "--strict",
                "--confirm-declared-offsets",
            ],
            mention: "relies on assumption declared_offset:ORDERS.PLACED_AT",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "strict mode refuses a declared offset read through FIRST_VALUE",
            sql: "SELECT FIRST_VALUE(placed_at) OVER (ORDER BY order_id)::DATE AS first_on FROM orders",
            flags: &["--schema", DECLARED_SCHEMA, "--strict"],
            mention: "assumption `declared_offset`, which the policy does not allow in this mode",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "LAST_VALUE keeps the origin of every declared column it reads",
            sql: "SELECT DATEDIFF('day', placed_at, LAST_VALUE(paid_at) OVER (ORDER BY order_id)) AS days FROM orders",
            flags: &["--schema", DECLARED_SCHEMA],
            mention: "relies on assumption declared_offset:ORDERS.PAID_AT",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a declared offset other than UTC is a usage error",
            sql: PLACED_DATE,
            flags: &[
                "--schema",
                "tests/cli/fixtures/declared_offset_unsupported.json",
            ],
            mention: "the declared offset `-05:00` is not supported; only +00:00 (UTC) is",
            expected_outcome: convert_outcome(2, false),
        },
        ConvertTestCase {
            description: "an offset declared for a column other than TIMESTAMP_TZ is a usage error",
            sql: "SELECT placed_on FROM orders",
            flags: &["--schema", "tests/cli/fixtures/declared_offset_naive.json"],
            mention: "an offset may be declared only for a TIMESTAMP_TZ column",
            expected_outcome: convert_outcome(2, false),
        },
        ConvertTestCase {
            description: "a non-UTC TIMEZONE refuses a zone-dependent statement in default mode",
            sql: NAIVE_TO_ZONED,
            flags: &["--session", "TIMEZONE=America/New_York"],
            mention: "session_timezone_utc",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a non-UTC TIMEZONE leaves a zone-independent statement converting",
            sql: "SELECT 1 + 2 AS total",
            flags: &["--session", "TIMEZONE=America/New_York"],
            mention: "converted under utc-v1",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "an explicit UTC TIMEZONE converts as before",
            sql: NAIVE_TO_ZONED,
            flags: &["--session", "TIMEZONE=UTC"],
            mention: "relies on assumption session_timezone_utc",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a misspelt parameter is refused by name, never falling back to utc-v1",
            sql: "SELECT 1 AS \"total\"",
            flags: &["--session", "QUOTED_IDENTIFIER_IGNORE_CASE=true"],
            mention: "unknown session parameters, which no recorded profile covers: \
                      QUOTED_IDENTIFIER_IGNORE_CASE",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "an unknown parameter is refused by name",
            sql: "SELECT 1 AS total",
            flags: &["--session", "ORDER_TOTAL_MODE=fast"],
            mention: "ORDER_TOTAL_MODE",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a parameter that cannot change results is accepted and ignored",
            sql: "SELECT 1 AS total",
            flags: &["--session", "QUERY_TAG=nightly orders"],
            mention: "converted under utc-v1",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "an empty parameter name is a usage error",
            sql: "SELECT 1 AS total",
            flags: &["--session", "=true"],
            mention: "names no parameter",
            expected_outcome: convert_outcome(2, false),
        },
        ConvertTestCase {
            description: "a parameter given twice with different values, in any case, is a usage error",
            sql: "SELECT 1 AS total",
            flags: &[
                "--session",
                "QUOTED_IDENTIFIERS_IGNORE_CASE=true",
                "--session",
                "quoted_identifiers_ignore_case=false",
            ],
            mention: "gives QUOTED_IDENTIFIERS_IGNORE_CASE twice",
            expected_outcome: convert_outcome(2, false),
        },
        ConvertTestCase {
            description: "comment-only input is a usage error",
            sql: "-- no statement here\n",
            flags: &[],
            mention: "the input holds no SQL statement",
            expected_outcome: convert_outcome(2, false),
        },
        ConvertTestCase {
            description: "the built-in default allows no divergence, so it refuses a zone-dependent statement",
            sql: NAIVE_TO_ZONED,
            flags: &["--builtin-policy"],
            mention: "under the built-in default",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "--json reports the outcome on standard output, still exiting 1 on refusal",
            sql: QUOTED_ALIAS,
            flags: &["--json"],
            mention: "refused",
            expected_outcome: ConvertOutcome {
                exit_code: Some(1),
                printed_sql: false,
                json_converted: Some(false),
                stderr_mentions: true,
            },
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

#[test]
fn given_quoted_names_when_converted_and_run_on_duckdb_then_output_names_follow_the_session() {
    let test_cases = [
        ColumnNameTestCase {
            description: "under utc-qiic-v1 a quoted lower-case alias comes back upper case",
            sql: "SELECT 1 AS \"total\", 2 AS \"orderCount\"",
            flags: &["--session", "QUOTED_IDENTIFIERS_IGNORE_CASE=true"],
            expected_names: vec!["TOTAL".to_owned(), "ORDERCOUNT".to_owned()],
        },
        ColumnNameTestCase {
            description: "under utc-v1 a quoted alias keeps its case",
            sql: "SELECT 1 AS \"total\", 2 AS \"orderCount\"",
            flags: &[],
            expected_names: vec!["total".to_owned(), "orderCount".to_owned()],
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_column_names(test_case.sql, test_case.flags),
            test_case.expected_names,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_no_policy_file_when_converting_or_counting_then_the_command_exits_2() {
    let test_cases = [
        PolicyFreeTestCase {
            description: "convert",
            arguments: &["convert", "query.sql"],
            expected_exit_code: Some(2),
        },
        PolicyFreeTestCase {
            description: "census",
            arguments: &["census", "query.sql"],
            expected_exit_code: Some(2),
        },
        PolicyFreeTestCase {
            description: "index",
            arguments: &["index", "query.sql"],
            expected_exit_code: Some(2),
        },
        PolicyFreeTestCase {
            description: "function",
            arguments: &[
                "function",
                "query.sql",
                "--name",
                "TOTAL",
                "--schema",
                "schema.json",
            ],
            expected_exit_code: Some(2),
        },
        PolicyFreeTestCase {
            description: "census with --builtin-policy runs",
            arguments: &["census", "--builtin-policy", "query.sql"],
            expected_exit_code: Some(0),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            policy_free_exit_code(test_case.arguments),
            test_case.expected_exit_code,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_hierarchy_with_a_cycle_when_running_the_conversion_then_duckdb_raises_the_depth_guard() {
    let test_cases = [
        ConvertedRunTestCase {
            description: "a cycle reached from the start rows raises past the depth limit",
            sql: "WITH edges AS (SELECT 1 AS id, 2 AS parent UNION ALL SELECT 2, 1 UNION ALL SELECT 3, NULL) SELECT id, LEVEL AS lvl FROM edges START WITH id = 1 CONNECT BY parent = PRIOR id",
            expected_error: Some("connect_by_depth"),
        },
        ConvertedRunTestCase {
            description: "a walk without a cycle runs to the end",
            sql: "WITH edges AS (SELECT 1 AS id, NULL AS parent UNION ALL SELECT 2, 1 UNION ALL SELECT 3, 2) SELECT id, LEVEL AS lvl FROM edges START WITH parent IS NULL CONNECT BY parent = PRIOR id",
            expected_error: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_run_error(test_case.sql).map(|error| error
                .contains(test_case.expected_error.unwrap_or(""))
                .then_some(())),
            test_case.expected_error.map(|_| Some(())),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_bracket_expression_with_a_lazy_spelling_when_converting_then_the_class_is_kept_literal()
{
    let test_cases = [
        ValueTestCase {
            description: "a class opening with ] keeps +? as literal members",
            sql: "SELECT REGEXP_SUBSTR('x+', '[]x+?]+') AS matched",
            expected_value: "x+",
        },
        ValueTestCase {
            description: "a negated class opening with ] keeps +? as literal members",
            sql: "SELECT REGEXP_SUBSTR('x+', '[^]+?]+') AS matched",
            expected_value: "x",
        },
        ValueTestCase {
            description: "a POSIX class inside a bracket keeps the bracket open",
            sql: "SELECT REGEXP_SUBSTR('a+ b', '[[:space:]+?]+') AS matched",
            expected_value: "+ ",
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
