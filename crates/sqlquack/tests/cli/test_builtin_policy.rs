use crate::helpers::{convert_outcome, policy_free_exit_code, run_convert};
use crate::test_types::{ConvertTestCase, PolicyFreeTestCase};

const TICKETS_SCHEMA: &str = "tests/cli/fixtures/semi_structured_tickets.json";
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
