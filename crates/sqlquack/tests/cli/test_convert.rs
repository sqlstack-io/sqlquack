use crate::helpers::{
    convert_outcome, converted_column_names, converted_first_value, converted_run_error,
    converted_value_over, override_failure, override_outcome, policy_free_exit_code,
    python_outcome, python_report, python_rows, run_convert,
};
use crate::test_types::{
    ColumnNameTestCase, ConvertOutcome, ConvertTestCase, ConvertedRunTestCase,
    CountedCallsTestCase, CrossingTestCase, DedicatedOverrideTestCase, OverrideFailureTestCase,
    OverrideTestCase, PolicyFreeTestCase, PythonRefusalTestCase, PythonRunTestCase,
    RunErrorTestCase, ValueTestCase,
};
use sha2::{Digest, Sha256};

const OTHER_BODY_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const COUNTING_HANDLER: &str = "import itertools\nTICKETS = itertools.count(1)\ndef handle(*arguments):\n    return next(TICKETS)\n";
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
            description: "a positional ORDER BY over an unordered aggregated array refuses",
            sql: "WITH totals AS (SELECT customer_id, ARRAY_AGG(sku) AS skus FROM \
                  (SELECT 1 AS customer_id, 'a' AS sku) GROUP BY customer_id), \
                  top_customer AS (SELECT customer_id, skus FROM totals ORDER BY 2 DESC LIMIT 1) \
                  SELECT customer_id FROM top_customer",
            flags: &[],
            mention: "a VARIANT key, which DuckDB orders and groups differently",
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

#[test]
fn given_a_time_with_nanoseconds_when_formatting_then_digits_beyond_the_format_are_truncated() {
    let test_cases = [
        ValueTestCase {
            description: "FF6 truncates the seventh digit instead of rounding into the next day",
            sql: "SELECT TO_VARCHAR(t, 'HH24:MI:SS.FF6') AS f FROM clocks",
            expected_value: "23:59:59.999999",
        },
        ValueTestCase {
            description: "FF3 truncates the digits after milliseconds",
            sql: "SELECT TO_VARCHAR(t, 'HH24:MI:SS.FF3') AS f FROM clocks",
            expected_value: "23:59:59.999",
        },
    ];
    for test_case in test_cases {
        let value = converted_value_over(
            test_case.sql,
            (
                "{\"CLOCKS\": [[\"T\", \"TIME(9)\"]]}",
                "CREATE TABLE \"CLOCKS\" (\"T\" TIME_NS); \
                 INSERT INTO \"CLOCKS\" VALUES ('23:59:59.9999999')",
            ),
        );
        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
}

#[test]
fn given_iso_date_text_under_a_numeric_format_when_running_then_it_raises_instead_of_null() {
    let test_cases = [RunErrorTestCase {
        description: "an ISO T separator is unrecorded, so TRY_TO_DATE raises rather than guessing NULL",
        sql: "SELECT TRY_TO_DATE('2025-10-05T10:00', 'YYYY-MM-DD') AS placed_on",
        expected_mention: "date_text_spelling",
    }];
    for test_case in test_cases {
        assert!(
            converted_run_error(test_case.sql)
                .is_some_and(|error| error.contains(test_case.expected_mention)),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_project_python_functions_when_converting_and_running_then_the_runtime_registers_them() {
    let test_cases = [
        PythonRunTestCase {
            description: "a VARCHAR and a VARIANT Python function, listed and run through the runtime",
            sql: "SELECT sku_family('AB-1') AS family, TO_JSON(sku_parts('ab')) AS parts, \
              sku_parts('ab'):name::VARCHAR AS name",
            handlers: &[
                (
                    "SKU_FAMILY",
                    "def handle(sku):\n    return sku.split('-')[0].lower()\n",
                ),
                (
                    "SKU_PARTS",
                    "def handle(sku):\n    return {'name': sku.upper(), 'codes': [ord(c) for c in sku]}\n",
                ),
            ],
            expected_refused_without_runtime: Some(false),
            expected_functions: serde_json::json!([
                {"name": "SKU_FAMILY", "abi": "scalar-v1", "arguments": ["VARCHAR"], "returns": "VARCHAR",
                 "volatility": "volatile", "null_input": "called"},
                {"name": "SKU_PARTS", "abi": "variant-json-v1", "arguments": ["VARCHAR"], "returns": "VARCHAR",
                 "volatility": "volatile", "null_input": "called"},
            ]),
            expected_rows: &[&["ab", "{\"codes\":[97,98],\"name\":\"AB\"}", "AB"]],
        },
        PythonRunTestCase {
            description: "a RETURNS NULL ON NULL INPUT function gives NULL for NULL without calling the handler",
            sql: "SELECT label_or_null(sku) AS l FROM (VALUES ('p'), (NULL)) AS v(sku) ORDER BY sku NULLS LAST",
            handlers: &[("LABEL_OR_NULL", "def handle(sku):\n    return 'called'\n")],
            expected_refused_without_runtime: Some(false),
            expected_functions: serde_json::json!([
                {"name": "LABEL_OR_NULL", "abi": "scalar-v1", "arguments": ["VARCHAR"], "returns": "VARCHAR",
                 "volatility": "volatile", "null_input": "returns_null"},
            ]),
            expected_rows: &[&["called"], &[""]],
        },
    ];
    for test_case in test_cases {
        let refused = python_report(test_case.sql, &[]);
        let report = python_report(test_case.sql, &["--python-runtime"]);
        assert_eq!(
            refused["converted"].as_bool(),
            test_case.expected_refused_without_runtime,
            "{}",
            test_case.description
        );
        assert_eq!(
            report["python_functions"], test_case.expected_functions,
            "{}",
            test_case.description
        );
        assert_eq!(
            python_rows(&report, test_case.handlers),
            test_case.expected_rows,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_volatile_python_function_when_called_repeatedly_then_duckdb_never_folds_or_shares_calls()
{
    let test_cases = [
        CountedCallsTestCase {
            description: "a call without arguments is made once per row",
            sql: "SELECT next_ticket() AS a FROM (VALUES ('p'), ('q'), ('r')) AS v(sku)",
            handlers: &[("NEXT_TICKET", COUNTING_HANDLER)],
            expected_distinct_values: 3,
        },
        CountedCallsTestCase {
            description: "a constant-argument call and two equal calls in one row are each made per row",
            sql: "SELECT ticket_for('x') AS a, ticket_for(sku) AS b, ticket_for(sku) AS c \
                  FROM (VALUES ('p'), ('q'), ('r')) AS v(sku)",
            handlers: &[("TICKET_FOR", COUNTING_HANDLER)],
            expected_distinct_values: 9,
        },
    ];
    for test_case in test_cases {
        let report = python_report(test_case.sql, &["--python-runtime"]);
        let values: std::collections::BTreeSet<String> = python_rows(&report, test_case.handlers)
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(
            values.len(),
            test_case.expected_distinct_values,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_python_call_the_conversion_would_repeat_or_batch_when_converting_then_it_is_refused() {
    let test_cases = [
        PythonRefusalTestCase {
            description: "DIV0 writes its dividend twice, which would call a volatile function twice",
            sql: "SELECT DIV0(next_ticket(), 2) AS d",
            expected_mention: "more often than the statement does",
        },
        PythonRefusalTestCase {
            description: "a string literal spelling the name does not count as a call",
            sql: "SELECT DIV0(next_ticket(), 2) AS d, \
                  'next_ticket next_ticket next_ticket next_ticket' AS note",
            expected_mention: "more often than the statement does",
        },
        PythonRefusalTestCase {
            description: "a comment spelling the name does not count as a call",
            sql: "SELECT DIV0(next_ticket(), 2) AS d /* next_ticket() next_ticket() */",
            expected_mention: "more often than the statement does",
        },
        PythonRefusalTestCase {
            description: "a column whose name starts with the function's does not count as a call",
            sql: "SELECT DIV0(next_ticket(), 2) AS d, next_ticket_source FROM orders",
            expected_mention: "more often than the statement does",
        },
        PythonRefusalTestCase {
            description: "a vectorized handler is refused until recorded",
            sql: "SELECT batch_label(sku) AS b FROM orders",
            expected_mention: "vectorized",
        },
    ];
    for test_case in test_cases {
        let report = python_report(test_case.sql, &["--python-runtime"]);
        assert!(
            report["refusals"]
                .to_string()
                .contains(test_case.expected_mention),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_python_result_no_recording_shows_when_running_then_the_runtime_raises_the_crossing() {
    let test_cases = [
        CrossingTestCase {
            description: "a datetime with a time zone inside a VARIANT",
            sql: "SELECT TO_JSON(sku_parts('ab')) AS parts",
            handler: (
                "SKU_PARTS",
                "import datetime\ndef handle(sku):\n    \
                return {'at': datetime.datetime(2024, 1, 2, tzinfo=datetime.timezone.utc)}\n",
            ),
            expected_mention: "python_value_crossing: a Python datetime with a time zone",
        },
        CrossingTestCase {
            description: "a naive datetime without a fraction of a second inside a VARIANT",
            sql: "SELECT TO_JSON(sku_parts('ab')) AS parts",
            handler: (
                "SKU_PARTS",
                "import datetime\ndef handle(sku):\n    \
                return {'at': datetime.datetime(2024, 1, 2, 3, 4, 5)}\n",
            ),
            expected_mention: "python_value_crossing: a Python datetime without a fraction",
        },
        CrossingTestCase {
            description: "a Decimal in exponent form inside a VARIANT",
            sql: "SELECT TO_JSON(sku_parts('ab')) AS parts",
            handler: (
                "SKU_PARTS",
                "import decimal\ndef handle(sku):\n    \
                return {'qty': decimal.Decimal('1E+2')}\n",
            ),
            expected_mention: "python_value_crossing: a Python Decimal in exponent form",
        },
        CrossingTestCase {
            description: "an int beyond 2^53 returned as FLOAT",
            sql: "SELECT unit_weight('ab') AS w",
            handler: ("UNIT_WEIGHT", "def handle(sku):\n    return 2**53 + 1\n"),
            expected_mention: "python_value_crossing: a Python int",
        },
        CrossingTestCase {
            description: "a Decimal in exponent form returned as NUMBER",
            sql: "SELECT unit_count('ab') AS n",
            handler: (
                "UNIT_COUNT",
                "import decimal\ndef handle(sku):\n    \
                return decimal.Decimal('1E+2')\n",
            ),
            expected_mention: "python_value_crossing: a Python Decimal",
        },
    ];
    for test_case in test_cases {
        let report = python_report(test_case.sql, &["--python-runtime"]);
        let error = python_outcome(&report, &[test_case.handler])
            .err()
            .unwrap_or_default();
        assert!(
            error.contains(test_case.expected_mention),
            "{}: {error}",
            test_case.description
        );
    }
}

#[test]
fn given_a_user_override_when_converting_then_its_macro_stands_in_for_the_function_it_replaces() {
    let definition = "FUNCTION (\n  arguments (s VARCHAR),\n  returns VARCHAR,\n  \
                      replaces REVERSE,\n  dialect duckdb,\n);\nupper(reverse(s))\n";
    let sql = "SELECT REVERSE(sku) AS r FROM orders ORDER BY order_id LIMIT 1";
    let body_sha256: String = Sha256::digest(b"upper(reverse(s))")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let test_cases = [
        OverrideTestCase {
            description: "the default mode calls the override, relying on its assumption",
            strict: false,
            verified_sha256: None,
            expected_value: Some("1-BA"),
            expected_refusal: None,
        },
        OverrideTestCase {
            description: "strict mode refuses an override no recorded case verifies",
            strict: true,
            verified_sha256: None,
            expected_value: None,
            expected_refusal: Some("until recorded cases verify that body"),
        },
        OverrideTestCase {
            description: "strict mode refuses an override verified for another body",
            strict: true,
            verified_sha256: Some(OTHER_BODY_SHA256),
            expected_value: None,
            expected_refusal: Some("until recorded cases verify that body"),
        },
        OverrideTestCase {
            description: "strict mode allows an override verified for the body as loaded",
            strict: true,
            verified_sha256: Some(""),
            expected_value: Some("1-BA"),
            expected_refusal: None,
        },
    ];
    for test_case in test_cases {
        let verified = test_case.verified_sha256.map(|sha256| {
            format!(
                "reversed_text={}",
                Some(sha256)
                    .filter(|given| !given.is_empty())
                    .unwrap_or(&body_sha256)
            )
        });
        let flags: Vec<&str> = test_case
            .strict
            .then_some("--strict")
            .into_iter()
            .chain(
                verified
                    .iter()
                    .flat_map(|named| ["--verified-override", named.as_str()]),
            )
            .collect();
        let (report, value) = override_outcome(sql, definition, &flags);
        assert_eq!(
            value.as_deref(),
            test_case.expected_value,
            "{}",
            test_case.description
        );
        assert_eq!(
            test_case
                .expected_refusal
                .map(|mention| report["refusals"].to_string().contains(mention)),
            test_case.expected_refusal.map(|_| true),
            "{}",
            test_case.description
        );
        assert_eq!(
            (
                report["assumptions"]
                    .to_string()
                    .contains("user_override:REVERSED_TEXT"),
                report["overrides"][0]["sha256"].as_str(),
                report["overrides"][0]["replaces"].as_str(),
            ),
            (
                test_case.expected_value.is_some(),
                Some(body_sha256.as_str()),
                Some("REVERSE")
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_override_whose_body_is_not_one_expression_when_converting_then_loading_fails() {
    let header = "FUNCTION (\n  arguments (s VARCHAR),\n  returns VARCHAR,\n  replaces REVERSE,\n  \
                  dialect duckdb,\n);\n";
    let test_cases = [
        OverrideFailureTestCase {
            description: "a body closing the macro and running SET and CREATE MACRO statements",
            body: "s); SET TimeZone = 'Asia/Tokyo'; CREATE OR REPLACE MACRO hacked(a) AS (a",
            expected_mention: "its body is not one DuckDB expression",
        },
        OverrideFailureTestCase {
            description: "a body adding a FROM and WHERE beyond its parentheses",
            body: "s) FROM orders WHERE (1",
            expected_mention: "reaches past its parentheses",
        },
    ];
    for test_case in test_cases {
        let (exit_code, stderr) = override_failure(
            "SELECT REVERSE('ab') AS r",
            &format!("{header}{}\n", test_case.body),
        );
        assert_eq!(
            (exit_code, stderr.contains(test_case.expected_mention)),
            (Some(2), true),
            "{}: {stderr}",
            test_case.description
        );
    }
}

#[test]
fn given_an_override_of_a_name_the_rules_type_when_converting_then_it_is_honoured_or_refused() {
    let test_cases = [
        DedicatedOverrideTestCase {
            description: "UPPER, which a built-in rule types, calls the override instead",
            replaces: "UPPER",
            body: "lower(s)",
            sql: "SELECT UPPER(sku) AS r FROM orders ORDER BY order_id LIMIT 1",
            expected_value: Some("ab-1"),
            expected_failure: None,
        },
        DedicatedOverrideTestCase {
            description: "COALESCE, which a built-in rule types, calls the override instead",
            replaces: "COALESCE",
            body: "'override'",
            sql: "SELECT COALESCE(sku) AS r FROM orders ORDER BY order_id LIMIT 1",
            expected_value: Some("override"),
            expected_failure: None,
        },
        DedicatedOverrideTestCase {
            description: "an aggregate cannot be overridden and fails at load",
            replaces: "MEDIAN",
            body: "s",
            sql: "SELECT MEDIAN(order_id) AS r FROM orders",
            expected_value: None,
            expected_failure: Some("an aggregate or window function"),
        },
    ];
    for test_case in test_cases {
        let definition = format!(
            "FUNCTION (\n  arguments (s VARCHAR),\n  returns VARCHAR,\n  replaces {},\n  \
             dialect duckdb,\n);\n{}\n",
            test_case.replaces, test_case.body
        );
        let (exit_code, stderr) = override_failure(test_case.sql, &definition);
        let value = test_case
            .expected_value
            .and_then(|_| override_outcome(test_case.sql, &definition, &[]).1);
        assert_eq!(
            (
                value.as_deref(),
                test_case
                    .expected_failure
                    .map(|mention| stderr.contains(mention)),
                exit_code == Some(2),
            ),
            (
                test_case.expected_value,
                test_case.expected_failure.map(|_| true),
                test_case.expected_failure.is_some(),
            ),
            "{}: {stderr}",
            test_case.description
        );
    }
}
