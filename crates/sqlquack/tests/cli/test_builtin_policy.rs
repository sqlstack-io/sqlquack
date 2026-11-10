use crate::helpers::{convert_outcome, policy_free_exit_code, run_convert};
use crate::test_types::{ConvertTestCase, PolicyFreeTestCase};

const TICKETS_PROJECT: &str = "tests/cli/fixtures/semi_structured_tickets";
const ORDERS_PROJECT: &str = "tests/cli/fixtures/integer_orders";
const EVENTS_PROJECT: &str = "tests/cli/fixtures/volatile_lookback";

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
            arguments: &["function", "query.sql", "--name", "TOTAL"],
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
fn given_a_table_with_semi_structured_columns_when_converting_then_only_reading_them_relies_on_json_fidelity()
 {
    let test_cases = [
        ConvertTestCase {
            description: "a column beside unread ARRAY and VARIANT columns converts under the built-in policy",
            sql: "SELECT ticket_id, subject FROM tickets WHERE ticket_id > 1",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "done, converted\n",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "COUNT(*) reads no column, so it converts too",
            sql: "SELECT COUNT(*) AS n FROM tickets t",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "done, converted\n",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a star reads the ARRAY column, so it relies on json_fidelity, with its warning",
            sql: "SELECT * FROM tickets",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "(1 known difference allowed: json_fidelity —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a qualified read of the ARRAY in WHERE relies on json_fidelity, with its warning",
            sql: "SELECT t.ticket_id FROM tickets t WHERE ARRAY_SIZE(t.Tags) > 0",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "(1 known difference allowed: json_fidelity —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a positional column may be the VARIANT, so it relies on json_fidelity, with its warning",
            sql: "SELECT $4 AS d FROM tickets",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "(1 known difference allowed: json_fidelity —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a subquery reading the VARIANT relies on json_fidelity, with its warning",
            sql: "SELECT ticket_id FROM tickets WHERE ticket_id IN (SELECT ticket_id FROM tickets \
                WHERE details IS NOT NULL)",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "(1 known difference allowed: json_fidelity —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a cast path read of the VARIANT converts, warning that odd JSON raises",
            sql: "SELECT details:kind::VARCHAR AS kind FROM tickets",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "(1 known difference allowed: json_fidelity —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "PARSE_JSON and an array literal build VARIANTs that raise on odd JSON, so they convert",
            sql: "SELECT PARSE_JSON(subject) AS j, [ticket_id, subject] AS a FROM tickets",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "(1 known difference allowed: json_fidelity —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "OBJECT_CONSTRUCT has unrecorded shapes and hazards, so it still refuses",
            sql: "SELECT OBJECT_CONSTRUCT('id', ticket_id) AS o FROM tickets",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "error[SQ0008]: JSON that DuckDB might read differently raises an error",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "comparing an uncast VARIANT rests on an assumption, so it still refuses",
            sql: "SELECT ticket_id FROM tickets WHERE details:kind = 'bug'",
            flags: &["--builtin-policy", "--project", TICKETS_PROJECT],
            mention: "error[SQ0008]: JSON that DuckDB might read differently raises an error",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "--strict refuses reading the VARIANT",
            sql: "SELECT details:kind::VARCHAR AS kind FROM tickets",
            flags: &["--builtin-policy", "--strict", "--project", TICKETS_PROJECT],
            mention: "help: the defaults allow this outside strict mode",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "the repository policy still guards the VARIANT it reads",
            sql: "SELECT details:kind::VARCHAR AS kind FROM tickets",
            flags: &["--project", TICKETS_PROJECT],
            mention: "(1 known difference allowed: json_fidelity —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "the clock converts under the built-in policy with its caveat",
            sql: "SELECT CURRENT_DATE() AS d",
            flags: &["--builtin-policy"],
            mention: "(2 known differences allowed: current_time, session_timezone_utc —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "comparing a TIMESTAMP_LTZ notes that it reads the session's time zone",
            sql: "SELECT CURRENT_TIMESTAMP() > '2020-01-01 00:00:00' AS late",
            flags: &["--builtin-policy"],
            mention: "(2 known differences allowed: current_time, session_timezone_utc —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "DATEDIFF to the clock reads it in the UTC session under the built-in policy",
            sql: "SELECT DATEDIFF('day', occurred_at, CURRENT_TIMESTAMP()) AS age FROM events",
            flags: &["--builtin-policy", "--project", EVENTS_PROJECT],
            mention: "(2 known differences allowed: current_time, session_timezone_utc —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "DATEDIFF with a keyword unit reads the clock the same way",
            sql: "SELECT DATEDIFF(month, occurred_at, CURRENT_TIMESTAMP()) AS age FROM events",
            flags: &["--builtin-policy", "--project", EVENTS_PROJECT],
            mention: "(2 known differences allowed: current_time, session_timezone_utc —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "TIMESTAMPDIFF with a quoted unit reads the clock the same way",
            sql: "SELECT TIMESTAMPDIFF('day', occurred_at, CURRENT_TIMESTAMP()) AS age FROM events",
            flags: &["--builtin-policy", "--project", EVENTS_PROJECT],
            mention: "(2 known differences allowed: current_time, session_timezone_utc —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a naive end falling back to the clock raises on nanoseconds under the built-in policy",
            sql: "SELECT DATEDIFF('day', occurred_at, COALESCE(occurred_at, CURRENT_TIMESTAMP())) AS age \
                FROM events",
            flags: &["--builtin-policy", "--project", EVENTS_PROJECT],
            mention: "(3 known differences allowed: current_time, session_timezone_utc, timestamp_tz_submicro —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "strict mode refuses DATEDIFF to the clock",
            sql: "SELECT DATEDIFF('day', occurred_at, CURRENT_TIMESTAMP()) AS age FROM events",
            flags: &["--builtin-policy", "--strict", "--project", EVENTS_PROJECT],
            mention: "error[SQ0008]: CURRENT_DATE and CURRENT_TIMESTAMP read DuckDB's clock",
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
    const BUILTIN: &[&str] = &["--builtin-policy", "--project", ORDERS_PROJECT];
    const STRICT: &[&str] = &["--builtin-policy", "--strict", "--project", ORDERS_PROJECT];
    const LOUD_ONLY: &[&str] = &[
        "--builtin-policy",
        "--loud-divergences-only",
        "--project",
        ORDERS_PROJECT,
    ];
    let overflow = "(1 known difference allowed: numeric_range —";
    let out_of_range = "(1 known difference allowed: timestamp_range —";
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
            mention: "error[SQ0008]: HASH gives a different value in DuckDB",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "--strict refuses the arithmetic too, saying the defaults allow it outside strict mode",
            sql: "SELECT order_id + 1 AS next_id FROM orders",
            flags: STRICT,
            mention: "help: the defaults allow this outside strict mode",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "--strict refuses the date arithmetic",
            sql: "SELECT DATEADD(day, 1, ordered_at) AS due FROM orders",
            flags: STRICT,
            mention: "error[SQ0008]: Timestamps outside 1677 to 2262 raise an error in DuckDB",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "--strict refuses the value divergence",
            sql: "SELECT HASH(order_id) AS h FROM orders",
            flags: STRICT,
            mention: "help: allow it in strict mode in a sqlquack.toml",
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
    const BUILTIN: &[&str] = &["--builtin-policy", "--project", ORDERS_PROJECT];
    const STRICT: &[&str] = &["--builtin-policy", "--strict", "--project", ORDERS_PROJECT];
    let test_cases = [
        ConvertTestCase {
            description: "a last-30-days filter on the clock's date converts with the clock's caveat",
            sql: "SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_DATE)",
            flags: BUILTIN,
            mention: "(2 known differences allowed: current_time, session_timezone_utc —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a last-30-days filter on CURRENT_TIMESTAMP() converts, warning that it raises out of range",
            sql: "SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_TIMESTAMP())",
            flags: BUILTIN,
            mention: "(3 known differences allowed: current_time, session_timezone_utc, timestamp_range —",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "--strict refuses the last-30-days filter on CURRENT_TIMESTAMP()",
            sql: "SELECT order_id FROM orders WHERE ordered_at >= DATEADD(day, -30, CURRENT_TIMESTAMP())",
            flags: STRICT,
            mention: "error[SQ0008]: CURRENT_DATE and CURRENT_TIMESTAMP read DuckDB's clock",
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
