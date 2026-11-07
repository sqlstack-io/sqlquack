use crate::helpers::{convert_outcome, policy_free_exit_code, run_convert};
use crate::test_types::{ConvertTestCase, PolicyFreeTestCase};

const TICKETS_SCHEMA: &str = "tests/cli/fixtures/semi_structured_tickets.json";
const ORDERS_SCHEMA: &str = "tests/cli/fixtures/integer_orders.json";
const EVENTS_SCHEMA: &str = "tests/cli/fixtures/volatile_lookback.json";

#[test]
fn given_no_policy_file_when_converting_or_counting_then_the_built_in_policy_applies() {
    let test_cases = [
        PolicyFreeTestCase {
            description: "convert runs under the built-in policy",
            arguments: &["convert", "query.sql"],
            policy_file: None,
            expected_exit_code: Some(0),
        },
        PolicyFreeTestCase {
            description: "census runs under the built-in policy",
            arguments: &["census", "query.sql"],
            policy_file: None,
            expected_exit_code: Some(0),
        },
        PolicyFreeTestCase {
            description: "index runs under the built-in policy",
            arguments: &["index", "query.sql"],
            policy_file: None,
            expected_exit_code: Some(0),
        },
        PolicyFreeTestCase {
            description: "function runs under the built-in policy",
            arguments: &[
                "function",
                "query.sql",
                "--name",
                "TOTAL",
                "--schema",
                "schema.json",
            ],
            policy_file: None,
            expected_exit_code: Some(0),
        },
        PolicyFreeTestCase {
            description: "the built-in policy refuses a divergence the repository policy allows",
            arguments: &["convert", "json.sql"],
            policy_file: None,
            expected_exit_code: Some(1),
        },
        PolicyFreeTestCase {
            description: "a sqlquack.toml in the directory is still read",
            arguments: &["convert", "json.sql"],
            policy_file: Some("sqlquack.toml"),
            expected_exit_code: Some(0),
        },
        PolicyFreeTestCase {
            description: "--builtin-policy still refuses where a sqlquack.toml allows it",
            arguments: &["convert", "--builtin-policy", "json.sql"],
            policy_file: Some("sqlquack.toml"),
            expected_exit_code: Some(1),
        },
        PolicyFreeTestCase {
            description: "the built-in policy converts the clock, which both engines read when the query runs",
            arguments: &["convert", "now.sql"],
            policy_file: None,
            expected_exit_code: Some(0),
        },
        PolicyFreeTestCase {
            description: "the built-in policy refuses the clock in strict mode",
            arguments: &["convert", "--strict", "now.sql"],
            policy_file: None,
            expected_exit_code: Some(1),
        },
        PolicyFreeTestCase {
            description: "the built-in policy refuses the clock where only loud divergences may convert",
            arguments: &["convert", "--loud-divergences-only", "now.sql"],
            policy_file: None,
            expected_exit_code: Some(1),
        },
        PolicyFreeTestCase {
            description: "the built-in policy refuses the clock in a session outside UTC",
            arguments: &["convert", "--session", "TIMEZONE=Europe/Paris", "now.sql"],
            policy_file: None,
            expected_exit_code: Some(1),
        },
        PolicyFreeTestCase {
            description: "a --policy file that does not exist is a usage error",
            arguments: &["convert", "--policy", "missing.toml", "query.sql"],
            policy_file: None,
            expected_exit_code: Some(2),
        },
        PolicyFreeTestCase {
            description: "an invalid sqlquack.toml is a usage error",
            arguments: &["convert", "query.sql"],
            policy_file: Some("crates/sqlquack/tests/cli/fixtures/invalid_policy.toml"),
            expected_exit_code: Some(2),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            policy_free_exit_code(test_case.arguments, test_case.policy_file),
            test_case.expected_exit_code,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_table_with_semi_structured_columns_when_converting_then_only_reading_them_needs_json_fidelity()
 {
    let test_cases = [
        ConvertTestCase {
            description: "a column beside unread ARRAY and VARIANT columns converts under the built-in policy",
            sql: "SELECT ticket_id, subject FROM tickets WHERE ticket_id > 1",
            flags: &["--builtin-policy", "--schema", TICKETS_SCHEMA],
            mention: "converted under the built-in default",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "COUNT(*) reads no column, so it converts too",
            sql: "SELECT COUNT(*) AS n FROM tickets t",
            flags: &["--builtin-policy", "--schema", TICKETS_SCHEMA],
            mention: "converted under the built-in default",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a star reads the ARRAY column, so it still needs json_fidelity",
            sql: "SELECT * FROM tickets",
            flags: &["--builtin-policy", "--schema", TICKETS_SCHEMA],
            mention: "needs known divergence `json_fidelity`",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a qualified read of the ARRAY in WHERE still needs json_fidelity",
            sql: "SELECT t.ticket_id FROM tickets t WHERE ARRAY_SIZE(t.Tags) > 0",
            flags: &["--builtin-policy", "--schema", TICKETS_SCHEMA],
            mention: "needs known divergence `json_fidelity`",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a positional column may be the VARIANT, so it still needs json_fidelity",
            sql: "SELECT $4 AS d FROM tickets",
            flags: &["--builtin-policy", "--schema", TICKETS_SCHEMA],
            mention: "needs known divergence `json_fidelity`",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a subquery reading the VARIANT still needs json_fidelity",
            sql: "SELECT ticket_id FROM tickets WHERE ticket_id IN (SELECT ticket_id FROM tickets \
                WHERE details IS NOT NULL)",
            flags: &["--builtin-policy", "--schema", TICKETS_SCHEMA],
            mention: "needs known divergence `json_fidelity`",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "the repository policy still guards the VARIANT it reads",
            sql: "SELECT details:kind::VARCHAR AS kind FROM tickets",
            flags: &["--schema", TICKETS_SCHEMA],
            mention: "relies on known divergence json_fidelity at node:cast, node:column",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "the clock converts under the built-in policy with its caveat",
            sql: "SELECT CURRENT_DATE() AS d",
            flags: &["--builtin-policy"],
            mention: "take their value when the query runs",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "comparing a TIMESTAMP_LTZ notes that it reads the session's time zone",
            sql: "SELECT CURRENT_TIMESTAMP() > '2020-01-01 00:00:00' AS late",
            flags: &["--builtin-policy"],
            mention: "compared with text that has no offset",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "DATEDIFF to the clock reads it in the UTC session under the built-in policy",
            sql: "SELECT DATEDIFF('day', occurred_at, CURRENT_TIMESTAMP()) AS age FROM events",
            flags: &["--builtin-policy", "--schema", EVENTS_SCHEMA],
            mention: "relies on assumption session_timezone_utc at fn:CURRENT_TIMESTAMP, fn:DATEDIFF",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "DATEDIFF with a keyword unit reads the clock the same way",
            sql: "SELECT DATEDIFF(month, occurred_at, CURRENT_TIMESTAMP()) AS age FROM events",
            flags: &["--builtin-policy", "--schema", EVENTS_SCHEMA],
            mention: "converted under the built-in default",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "TIMESTAMPDIFF with a quoted unit reads the clock the same way",
            sql: "SELECT TIMESTAMPDIFF('day', occurred_at, CURRENT_TIMESTAMP()) AS age FROM events",
            flags: &["--builtin-policy", "--schema", EVENTS_SCHEMA],
            mention: "converted under the built-in default",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a naive end falling back to the clock raises on nanoseconds under the built-in policy",
            sql: "SELECT DATEDIFF('day', occurred_at, COALESCE(occurred_at, CURRENT_TIMESTAMP())) AS age \
                FROM events",
            flags: &["--builtin-policy", "--schema", EVENTS_SCHEMA],
            mention: "relies on known divergence timestamp_tz_submicro at node:coalesce",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "strict mode refuses DATEDIFF to the clock",
            sql: "SELECT DATEDIFF('day', occurred_at, CURRENT_TIMESTAMP()) AS age FROM events",
            flags: &["--builtin-policy", "--strict", "--schema", EVENTS_SCHEMA],
            mention: "needs known divergence",
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

#[test]
fn given_arithmetic_that_only_raises_at_its_range_when_converting_then_the_built_in_policy_warns() {
    const BUILTIN: &[&str] = &["--builtin-policy", "--schema", ORDERS_SCHEMA];
    const STRICT: &[&str] = &["--builtin-policy", "--strict", "--schema", ORDERS_SCHEMA];
    const LOUD_ONLY: &[&str] = &[
        "--builtin-policy",
        "--loud-divergences-only",
        "--schema",
        ORDERS_SCHEMA,
    ];
    let overflow = "needs more than 38 digits, DuckDB raises an Overflow error";
    let out_of_range = "lands outside 1677-09-21 00:12:43.145224192 to 2262-04-11 23:47:16.854775807 UTC, the converted SQL raises an error";
    let test_cases = [
        ConvertTestCase {
            description: "adding to a NUMBER(38,0) converts, warning that DuckDB raises past 38 digits",
            sql: "SELECT order_id + 1 AS next_id FROM orders",
            flags: BUILTIN,
            mention: overflow,
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "SUM of a NUMBER(38,0) converts with the same warning",
            sql: "SELECT SUM(quantity) AS units FROM orders",
            flags: BUILTIN,
            mention: overflow,
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "DATEADD of a TIMESTAMP_NTZ(9) converts, warning that it raises out of range",
            sql: "SELECT DATEADD(day, 1, ordered_at) AS due FROM orders",
            flags: BUILTIN,
            mention: out_of_range,
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "adding an interval to a TIMESTAMP_NTZ(9) converts with the same warning",
            sql: "SELECT ordered_at + INTERVAL '1 day' AS due FROM orders",
            flags: BUILTIN,
            mention: out_of_range,
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "only-loud mode keeps the range warnings, which never return a different value",
            sql: "SELECT order_id * 2 AS doubled FROM orders",
            flags: LOUD_ONLY,
            mention: overflow,
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a value divergence, which can return a different value, still refuses",
            sql: "SELECT HASH(order_id) AS h FROM orders",
            flags: BUILTIN,
            mention: "needs known divergence `hash`",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "--strict refuses the arithmetic too, naming the constructs the default allows",
            sql: "SELECT order_id + 1 AS next_id FROM orders",
            flags: STRICT,
            mention: "allows `numeric_range` only for node:add, node:sub, node:mul, fn:SUM",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "--strict refuses the date arithmetic",
            sql: "SELECT DATEADD(day, 1, ordered_at) AS due FROM orders",
            flags: STRICT,
            mention: "needs known divergence `timestamp_range`",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "--strict refuses the value divergence",
            sql: "SELECT HASH(order_id) AS h FROM orders",
            flags: STRICT,
            mention: "needs known divergence `hash`",
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

#[test]
fn given_a_last_30_days_filter_when_converting_then_the_built_in_policy_warns() {
    const BUILTIN: &[&str] = &["--builtin-policy", "--schema", ORDERS_SCHEMA];
    const STRICT: &[&str] = &["--builtin-policy", "--strict", "--schema", ORDERS_SCHEMA];
    let test_cases = [
        ConvertTestCase {
            description: "a last-30-days filter on the clock's date converts with the clock's caveat",
            sql: "SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_DATE)",
            flags: BUILTIN,
            mention: "take their value when the query runs",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a last-30-days filter on CURRENT_TIMESTAMP() converts, warning that it raises out of range",
            sql: "SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_TIMESTAMP())",
            flags: BUILTIN,
            mention: "compared at nanosecond precision",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "--strict refuses the last-30-days filter on CURRENT_TIMESTAMP()",
            sql: "SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_TIMESTAMP())",
            flags: STRICT,
            mention: "needs known divergence",
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
