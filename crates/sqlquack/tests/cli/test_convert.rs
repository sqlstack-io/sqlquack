use crate::helpers::{
    convert_outcome, convert_streams, converted_column_names, converted_first_value,
    converted_run_error, converted_value_over, first_refusal_help, override_failure,
    override_outcome, policy_free_exit_code, python_outcome, python_report, python_rows,
    run_convert, run_with_input, schema_failure,
};
use crate::test_types::{
    ColumnNameTestCase, ConvertDiagnosticTestCase, ConvertOutcome, ConvertTestCase,
    ConvertedRunTestCase, CountedCallsTestCase, CrossingTestCase, DedicatedOverrideTestCase,
    DeepChainTestCase, DivergenceHelpTestCase, FailureKindTestCase, OutputFormatTestCase,
    OverrideFailureTestCase, OverrideTestCase, PolicyFreeTestCase, PythonRefusalTestCase,
    PythonRunTestCase, RunErrorTestCase, SchemaErrorTestCase, ValueTestCase,
};
use sha2::{Digest, Sha256};

const OTHER_BODY_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const COUNTING_HANDLER: &str = "import itertools\nTICKETS = itertools.count(1)\ndef handle(*arguments):\n    return next(TICKETS)\n";
const QUOTED_ALIAS: &str =
    "WITH totals AS (SELECT 1 AS \"orderTotal\") SELECT ordertotal FROM totals";
const PLACED_DATE: &str = "SELECT TO_DATE(placed_at) AS placed_on FROM orders";
const DECLARED_SCHEMA: &str = "tests/cli/fixtures/declared_offsets.json";
const QUALIFIED_SCHEMA: &str = "tests/cli/fixtures/qualified_relations.json";
const NAIVE_TO_ZONED: &str =
    "SELECT '2024-03-10 22:30:00'::TIMESTAMP_NTZ(6)::TIMESTAMP_TZ AS placed_at";

#[test]
fn given_snowflake_sql_when_converting_then_duckdb_sql_or_named_refusals_are_printed() {
    let test_cases = [
        ConvertTestCase {
            description: "a repeated result name refuses with help to alias each column",
            sql: "SELECT 1, 1",
            flags: &[],
            mention: "give each result column its own alias",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "EQUAL_NULL of a VARIANT and text refuses: no recording shows a JSON null",
            sql: "SELECT EQUAL_NULL(PARSE_JSON('null'), '1') AS e",
            flags: &[],
            mention: "how a JSON null compares",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "IS DISTINCT FROM between text and a VARIANT refuses the same way",
            sql: "SELECT '1' IS DISTINCT FROM PARSE_JSON('null') AS e",
            flags: &[],
            mention: "how a JSON null compares",
            expected_outcome: convert_outcome(1, false),
        },
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
            description: "SYSTEM$TYPEOF text that reaches the result with its storage suffix refuses",
            sql: "WITH t AS (SELECT SYSTEM$TYPEOF(2.5) AS kind) SELECT kind FROM t",
            flags: &[],
            mention: "SYSTEM$TYPEOF text read before",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "SYSTEM$TYPEOF text read by a function before its suffix is removed refuses",
            sql: "WITH t AS (SELECT LENGTH(SYSTEM$TYPEOF(2.5)) AS n) SELECT n FROM t",
            flags: &[],
            mention: "SYSTEM$TYPEOF text read before",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "ordering by SYSTEM$TYPEOF text beside another output before LIMIT refuses",
            sql: "WITH s AS (SELECT 1 AS k, 2.5 AS v), \
                  t AS (SELECT SYSTEM$TYPEOF(s.v) AS kind, s.k FROM s ORDER BY kind LIMIT 1) \
                  SELECT REGEXP_REPLACE(t.kind, '\\\\[.*\\\\]$', '') AS kind, t.k FROM t",
            flags: &[],
            mention: "SYSTEM$TYPEOF text read before",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "SYSTEM$TYPEOF text stripped in a later CTE converts",
            sql: "WITH s AS (SELECT 2.5 AS v), \
                  t AS (SELECT SYSTEM$TYPEOF(s.v) AS kind FROM s ORDER BY kind LIMIT 1) \
                  SELECT REGEXP_REPLACE(t.kind, '\\\\[.*\\\\]$', '') AS kind FROM t",
            flags: &[],
            mention: "done, converted",
            expected_outcome: convert_outcome(0, true),
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
            description: "a refused statement prints no SQL, renders the refusal at its place and exits 1",
            sql: QUOTED_ALIAS,
            flags: &[],
            mention: " --> <stdin>:1:",
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
            mention: "casting to DATE of TIMESTAMP_TZ(9), which reads the local time of an offset",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "the suggested rewrite through a zone column converts the local date",
            sql: "SELECT CONVERT_TIMEZONE(ship_zone, shipped_at)::TIMESTAMP_NTZ::DATE AS shipped_on FROM orders",
            flags: &["--schema", DECLARED_SCHEMA],
            mention: "relies on known divergence time_zone_spelling at fn:CONVERT_TIMEZONE",
            expected_outcome: convert_outcome(0, true),
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
            description: "the built-in default takes a UTC TIMEZONE and result-neutral parameters",
            sql: "SELECT 1 AS total",
            flags: &[
                "--builtin-policy",
                "--session",
                "TIMEZONE=UTC",
                "--session",
                "query_tag=nightly",
            ],
            mention: "converted under the built-in default",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "the built-in default refuses a parameter that can change a result, by name",
            sql: "SELECT 1 AS total",
            flags: &["--builtin-policy", "--session", "WEEK_START=1"],
            mention: "not WEEK_START; pass --policy",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a subquery reading a lambda's parameter refuses: it cannot be bound outside",
            sql: "SELECT TRANSFORM(SPLIT('a,b', ','), x VARCHAR -> (SELECT UPPER(x))) AS t",
            flags: &[],
            mention: "a subquery that reads a lambda's parameter",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a subquery calling a volatile function refuses where a lambda wraps it",
            sql: "SELECT DATEADD(day, -3, (SELECT MAX(occurred_at) FROM events \
                  WHERE event_id > LOOKBACK())) AS d",
            flags: &["--schema", "tests/cli/fixtures/volatile_lookback.json"],
            mention: "a subquery calling a function whose result can change between calls",
            expected_outcome: convert_outcome(1, false),
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
            arguments: &["convert", "now.sql"],
            policy_file: None,
            expected_exit_code: Some(1),
        },
        PolicyFreeTestCase {
            description: "a sqlquack.toml in the directory is still read",
            arguments: &["convert", "now.sql"],
            policy_file: Some("sqlquack.toml"),
            expected_exit_code: Some(0),
        },
        PolicyFreeTestCase {
            description: "--builtin-policy still refuses where a sqlquack.toml allows it",
            arguments: &["convert", "--builtin-policy", "now.sql"],
            policy_file: Some("sqlquack.toml"),
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
fn given_qualified_table_names_when_converting_then_they_read_the_declared_relation() {
    let staged = (
        "{\"ANALYTICS.STAGING.STG_ORDERS\": [[\"STATUS\", \"VARCHAR\"]], \
          \"ANALYTICS.MARTS.STG_ORDERS\": [[\"STATUS\", \"VARCHAR\"]], \
          \"PAYMENTS\": [[\"STATUS\", \"VARCHAR\"]]}",
        "ATTACH ':memory:' AS analytics; CREATE SCHEMA analytics.staging; \
         CREATE SCHEMA analytics.marts; \
         CREATE TABLE analytics.staging.stg_orders (status VARCHAR); \
         CREATE TABLE analytics.marts.stg_orders (status VARCHAR); \
         CREATE TABLE payments (status VARCHAR); \
         INSERT INTO analytics.staging.stg_orders VALUES ('staged'); \
         INSERT INTO analytics.marts.stg_orders VALUES ('marted'); \
         INSERT INTO payments VALUES ('paid')",
    );
    let test_cases = [
        ValueTestCase {
            description: "compiled dbt's database.schema.table reads that table in the attached database",
            sql: "with source as (select * from analytics.staging.stg_orders) select status from source",
            expected_value: "staged",
        },
        ValueTestCase {
            description: "unquoted parts fold to upper case, so a mixed-case spelling is the same table",
            sql: "SELECT o.status FROM Analytics.Marts.Stg_Orders AS o",
            expected_value: "marted",
        },
        ValueTestCase {
            description: "a quoted upper-case name is the declared one",
            sql: "SELECT status FROM \"ANALYTICS\".\"MARTS\".\"STG_ORDERS\"",
            expected_value: "marted",
        },
        ValueTestCase {
            description: "schema.table matches the one database that declares it",
            sql: "SELECT stg_orders.status FROM staging.stg_orders",
            expected_value: "staged",
        },
        ValueTestCase {
            description: "a qualified name reads a relation declared without qualifiers",
            sql: "SELECT status FROM analytics.raw.payments",
            expected_value: "paid",
        },
    ];
    for test_case in test_cases {
        let value = converted_value_over(test_case.sql, staged);
        assert_eq!(value, test_case.expected_value, "{}", test_case.description);
    }
    let refusals = [
        ConvertTestCase {
            description: "a name matching tables in two schemas, with no session schema, names both",
            sql: "SELECT order_id FROM stg_orders",
            flags: &["--schema", QUALIFIED_SCHEMA],
            mention: "relation `STG_ORDERS` matches `ANALYTICS.MARTS.STG_ORDERS`, \
                      `ANALYTICS.STAGING.STG_ORDERS` in the schema",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a quoted lower-case part is a different table from the declared one",
            sql: "SELECT order_id FROM analytics.\"staging\".stg_orders",
            flags: &["--schema", QUALIFIED_SCHEMA],
            mention: "relation `ANALYTICS.staging.STG_ORDERS` matches `ANALYTICS.STAGING.STG_ORDERS` only without regard to case",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a database the schema does not declare refuses as unknown",
            sql: "SELECT order_id FROM other.staging.stg_orders",
            flags: &["--schema", QUALIFIED_SCHEMA],
            mention: "relation `OTHER.STAGING.STG_ORDERS` has no known columns",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "two qualifiers of one undeclared-qualifier relation refuse instead of reading it twice",
            sql: "SELECT a.order_id FROM sales.orders a JOIN archive.orders b USING (order_id)",
            flags: &["--schema", DECLARED_SCHEMA],
            mention: "`SALES.ORDERS` and `ARCHIVE.ORDERS` both read the schema relation `ORDERS`",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a qualified name a CTE of the same name would capture refuses",
            sql: "WITH orders AS (SELECT 1 AS order_id) SELECT order_id FROM sales.orders",
            flags: &["--schema", DECLARED_SCHEMA],
            mention: "which a CTE of the same name hides",
            expected_outcome: convert_outcome(1, false),
        },
    ];
    for test_case in refusals {
        assert_eq!(
            run_convert(test_case.sql, test_case.flags, test_case.mention),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
    let sessions = [
        (
            &["SCHEMA=marts"][..],
            "\"ANALYTICS\".\"MARTS\".\"STG_ORDERS\"",
        ),
        (
            &["DATABASE=analytics", "SCHEMA=staging"][..],
            "\"ANALYTICS\".\"STAGING\".\"STG_ORDERS\"",
        ),
    ];
    for (parameters, expected_relation) in sessions {
        let mut flags = vec!["--builtin-policy", "--schema", QUALIFIED_SCHEMA];
        for parameter in parameters {
            flags.extend(["--session", parameter]);
        }
        let (stdout, _) = convert_streams("SELECT order_id FROM stg_orders", &flags);
        assert!(
            stdout.contains(&format!("FROM {expected_relation}")),
            "the session's {parameters:?} settles which table an unqualified name reads: {stdout}"
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
        assert_eq!(
            report["refusals"][0]["span"]["line"].as_u64(),
            Some(1),
            "{}: the refusal points into the statement",
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

#[test]
fn given_flatten_outer_over_empty_and_missing_inputs_when_running_then_columns_are_as_recorded() {
    let schema = r#"{"relations": {"DOCS": [["K", "NUMBER(4,0)"], ["J", "VARCHAR"]]}}"#;
    let setup = "SET TimeZone = 'UTC'; CREATE TABLE \"DOCS\" (\"K\" INTEGER, \"J\" VARCHAR); \
                 INSERT INTO \"DOCS\" VALUES (1, '{\"lines\": []}'), (2, '{\"lines\": {}}'), \
                 (3, '{\"lines\": null}'), (4, '{}'), (5, '{\"lines\": [7]}')";
    let test_cases = [
        ValueTestCase {
            description: "OUTER: PATH '' and THIS the input for an empty container, NULL otherwise",
            sql: "SELECT LISTAGG(d.k::VARCHAR || '=' || COALESCE(f.path, 'N') || '/' \
                  || COALESCE(TO_JSON(f.this), 'N'), ',') WITHIN GROUP (ORDER BY d.k) AS r \
                  FROM docs d, LATERAL FLATTEN(input => PARSE_JSON(d.j):lines, outer => TRUE) f",
            expected_value: "1=/[],2=/{},3=N/N,4=N/N,5=[0]/[7]",
        },
        ValueTestCase {
            description: "OUTER with MODE 'ARRAY': an empty object is a skipped kind, all NULL",
            sql: "SELECT LISTAGG(d.k::VARCHAR || '=' || COALESCE(f.path, 'N'), ',') \
                  WITHIN GROUP (ORDER BY d.k) AS r FROM docs d, \
                  LATERAL FLATTEN(input => PARSE_JSON(d.j):lines, outer => TRUE, mode => 'ARRAY') f",
            expected_value: "1=,2=N,3=N,4=N,5=[0]",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_value_over(test_case.sql, (schema, setup)),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_mode_or_a_zone_spelling_without_a_recorded_answer_when_running_then_it_raises() {
    let test_cases = [
        RunErrorTestCase {
            description: "MODE without GROUP BY over no rows, which Snowflake answers by plan",
            sql: "SELECT MODE(x) AS m FROM (SELECT 1 AS x) AS t WHERE x > 1",
            expected_mention: "mode_of_no_rows",
        },
        RunErrorTestCase {
            description: "a zone column holding a lower-case name Snowflake rejects",
            sql: "SELECT CONVERT_TIMEZONE(z, 'UTC', '2024-07-01 12:00:00'::TIMESTAMP_NTZ) AS c \
                  FROM (SELECT column1 AS z FROM VALUES ('utc')) AS t",
            expected_mention: "time_zone_spelling",
        },
    ];
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
fn given_recorded_shapes_when_converting_then_they_convert_or_refuse_as_recorded() {
    let test_cases = [
        PythonRefusalTestCase {
            description: "a lower-case zone literal, which Snowflake rejects",
            sql: "SELECT CONVERT_TIMEZONE('utc', 'Europe/London', '2024-07-01 12:00:00'::TIMESTAMP_NTZ) AS c",
            expected_mention: "no recording shows Snowflake accepting",
        },
        PythonRefusalTestCase {
            description: "a lower-case IANA zone literal",
            sql: "SELECT CONVERT_TIMEZONE('UTC', 'europe/london', '2024-07-01 12:00:00'::TIMESTAMP_NTZ) AS c",
            expected_mention: "no recording shows Snowflake accepting",
        },
    ];
    for test_case in test_cases {
        let refusals = python_report(test_case.sql, &[])["refusals"].to_string();
        assert!(
            refusals.contains(test_case.expected_mention),
            "{}: {refusals}",
            test_case.description
        );
    }
}

#[test]
fn given_a_subquery_operand_when_converting_and_running_then_it_is_bound_outside_lambdas() {
    let test_cases = [
        ValueTestCase {
            description: "an incremental lookback from the latest row keeps the last three days",
            sql: "WITH events AS (SELECT 1 AS event_id, '2026-01-10 00:00:00'::TIMESTAMP_NTZ AS \
                  occurred_at UNION ALL SELECT 2, '2026-01-02 00:00:00'::TIMESTAMP_NTZ UNION ALL \
                  SELECT 3, '2026-01-08 12:00:00'::TIMESTAMP_NTZ) SELECT COUNT(*)::VARCHAR AS n FROM events \
                  WHERE occurred_at >= DATEADD(day, -3, (SELECT MAX(occurred_at) FROM events))",
            expected_value: "2",
        },
        ValueTestCase {
            description: "a DATEADD of a DATEADD of a subquery binds it once, outside both",
            sql: "WITH events AS (SELECT 1 AS event_id, '2026-01-10 00:00:00'::TIMESTAMP_NTZ AS \
                  occurred_at UNION ALL SELECT 2, '2026-01-02 00:00:00'::TIMESTAMP_NTZ UNION ALL \
                  SELECT 3, '2026-01-08 12:00:00'::TIMESTAMP_NTZ) SELECT COUNT(*)::VARCHAR AS n FROM events \
                  WHERE occurred_at >= \
                  DATEADD(hour, 37, DATEADD(day, -3, (SELECT MAX(occurred_at) FROM events)))",
            expected_value: "1",
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
fn given_connect_by_root_of_a_qualified_column_when_running_then_it_reads_the_root_value() {
    let test_cases = [ValueTestCase {
        description: "CONNECT_BY_ROOT of a qualified column reads the root row's value",
        sql: "WITH l AS (SELECT * FROM VALUES ('A', NULL, 'x'), ('B', 'A', 'y') AS v(id, p, n)) \
                  SELECT LISTAGG(id || CONNECT_BY_ROOT k.n, ',') WITHIN GROUP (ORDER BY id) AS r \
                  FROM l k START WITH p IS NULL CONNECT BY p = PRIOR id",
        expected_value: "Ax,Bx",
    }];
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
fn given_an_inner_join_using_keys_of_two_widths_when_running_then_a_wide_unmatched_key_does_not_fail()
 {
    let schema = r#"{"relations": {"NARROW": [["K", "NUMBER(18,0)"]], "WIDE": [["K", "NUMBER(38,0)"], ["V", "VARCHAR"]]}}"#;
    let setup = "SET TimeZone = 'UTC'; CREATE TABLE \"NARROW\" (\"K\" DECIMAL(18,0)); \
                 CREATE TABLE \"WIDE\" (\"K\" DECIMAL(38,0), \"V\" VARCHAR); \
                 INSERT INTO \"NARROW\" VALUES (7); \
                 INSERT INTO \"WIDE\" VALUES (7, 'seven'), (99999999999999999999999, 'wide')";
    let test_cases = [
        ValueTestCase {
            description: "the wide row matches nothing and the narrow key's row joins",
            sql: "SELECT LISTAGG(k::VARCHAR || '=' || v, ',') WITHIN GROUP (ORDER BY k) AS r \
                  FROM narrow JOIN wide USING (k)",
            expected_value: "7=seven",
        },
        ValueTestCase {
            description: "a filter on the joined key, which DuckDB may push below the join",
            sql: "SELECT LISTAGG(wide.k::VARCHAR || '=' || v, ',') WITHIN GROUP (ORDER BY wide.k) \
                  AS r FROM narrow JOIN wide USING (k) WHERE wide.k > 1",
            expected_value: "7=seven",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_value_over(test_case.sql, (schema, setup)),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_zoned_text_when_converting_and_running_then_wall_time_parts_hold_its_utc_offset() {
    let schema = r#"{"relations": {"EVENTS": [["AT_TEXT", "VARCHAR"]]}}"#;
    let setup = "SET TimeZone = 'UTC'; CREATE TABLE \"EVENTS\" (\"AT_TEXT\" VARCHAR); \
                 INSERT INTO \"EVENTS\" VALUES ('2023-12-31 22:30:15 +00:00')";
    let test_cases = [
        ValueTestCase {
            description: "YEAR of a text column cast to TIMESTAMP_TZ, the year at its +00:00 offset",
            sql: "SELECT YEAR(CAST(at_text AS TIMESTAMP_TZ))::VARCHAR AS y FROM events",
            expected_value: "2023",
        },
        ValueTestCase {
            description: "TO_DATE of TO_TIMESTAMP_TZ of a text column",
            sql: "SELECT TO_DATE(TO_TIMESTAMP_TZ(at_text))::VARCHAR AS d FROM events",
            expected_value: "2023-12-31",
        },
        ValueTestCase {
            description: "the wall time of a TIMESTAMP_LTZ read from text, in the UTC session",
            sql: "SELECT CAST(CAST(at_text AS TIMESTAMP_LTZ) AS TIMESTAMP_NTZ)::VARCHAR AS w \
                  FROM events",
            expected_value: "2023-12-31 22:30:15.000",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_value_over(test_case.sql, (schema, setup)),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_zoned_text_with_another_offset_when_running_then_its_wall_time_raises() {
    let test_cases = [RunErrorTestCase {
        description: "an ISO literal at +09:00, whose wall time DuckDB's instant does not keep",
        sql: "SELECT YEAR('2023-12-31T22:30:15+09:00'::TIMESTAMP_TZ) AS y",
        expected_mention: "timestamp_tz_offset",
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
fn given_a_name_the_conversion_does_not_scope_when_converting_then_the_refusal_says_why() {
    let test_cases = [
        PythonRefusalTestCase {
            description: "SEQ of FLATTEN is a known column no rule converts, not an unresolved one",
            sql: "SELECT f.seq AS q FROM orders, LATERAL FLATTEN(input => PARSE_JSON(orders.sku)) f",
            expected_mention: "SEQ of FLATTEN or SPLIT_TO_TABLE",
        },
        PythonRefusalTestCase {
            description: "SEQ of SPLIT_TO_TABLE",
            sql: "SELECT seg.seq AS q FROM orders, LATERAL SPLIT_TO_TABLE(orders.sku, '-') seg",
            expected_mention: "SEQ of FLATTEN or SPLIT_TO_TABLE",
        },
        PythonRefusalTestCase {
            description: "a GROUP BY key naming only a select alias",
            sql: "SELECT UPPER(sku) AS family, COUNT(*) AS n FROM orders GROUP BY family",
            expected_mention: "names only a select alias",
        },
        PythonRefusalTestCase {
            description: "a FROM subquery reading a column of the enclosing query",
            sql: "SELECT order_id, (SELECT q FROM (SELECT order_id + 1 AS q)) AS next_id \
                  FROM orders",
            expected_mention: "reading a column of an enclosing query",
        },
        PythonRefusalTestCase {
            description: "an ORDER BY alias over a column two FROM relations hold",
            sql: "SELECT COALESCE(o.sku, p.sku) AS sku FROM orders AS o \
                  JOIN orders AS p ON o.order_id = p.order_id ORDER BY sku",
            expected_mention: "several FROM relations hold",
        },
    ];
    for test_case in test_cases {
        let report = python_report(test_case.sql, &[]);
        let refusals = report["refusals"].to_string();
        assert_eq!(
            (
                refusals.contains(test_case.expected_mention),
                refusals.contains("unresolved name")
            ),
            (true, false),
            "{}: {refusals}",
            test_case.description
        );
    }
}

#[test]
fn given_variant_keys_when_converting_and_running_then_they_order_and_compare_as_recorded() {
    let schema = r#"{"relations": {"DOCS": [["K", "NUMBER(4,0)"], ["J", "VARCHAR"]]}}"#;
    let setup = "SET TimeZone = 'UTC'; CREATE TABLE \"DOCS\" (\"K\" INTEGER, \"J\" VARCHAR); \
                 INSERT INTO \"DOCS\" VALUES (0, '\"b\"'), (1, '10'), (2, 'true'), (3, 'null'), \
                 (4, '2'), (5, NULL)";
    let test_cases = [
        ValueTestCase {
            description: "ORDER BY a VARIANT: booleans, numbers by value, strings, JSON null, NULL",
            sql: "SELECT LISTAGG(k::VARCHAR, ',') WITHIN GROUP (ORDER BY PARSE_JSON(j), k) AS o \
                  FROM docs",
            expected_value: "2,4,1,0,3,5",
        },
        ValueTestCase {
            description: "MIN and MAX of a VARIANT, JSON null greatest",
            sql: "SELECT TO_JSON(MIN(PARSE_JSON(j))) || ' ' || TO_JSON(MAX(PARSE_JSON(j))) AS m \
                  FROM docs",
            expected_value: "true null",
        },
        ValueTestCase {
            description: "MAX_BY a VARIANT key",
            sql: "SELECT MAX_BY(k, PARSE_JSON(j))::VARCHAR AS m FROM docs WHERE k <> 3",
            expected_value: "0",
        },
        ValueTestCase {
            description: "EQUAL_NULL of a VARIANT and NULL",
            sql: "SELECT EQUAL_NULL(PARSE_JSON(j), NULL)::VARCHAR AS e FROM docs WHERE k = 5",
            expected_value: "true",
        },
        ValueTestCase {
            description: "TO_DECIMAL of a VARIANT number",
            sql: "SELECT TO_DECIMAL(PARSE_JSON(j), 10, 2)::VARCHAR AS d FROM docs WHERE k = 1",
            expected_value: "10.00",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_value_over(test_case.sql, (schema, setup)),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_zoned_column_and_text_or_a_by_name_union_when_running_then_values_are_as_recorded() {
    let schema = r#"{"relations": {"EVENTS": [["AT", "TIMESTAMP_TZ(9)"]]}}"#;
    let setup = "SET TimeZone = 'UTC'; CREATE TABLE \"EVENTS\" (\"AT\" TIMESTAMPTZ); \
                 INSERT INTO \"EVENTS\" VALUES ('2023-12-31 22:30:15-05:30'), \
                 ('2024-01-01 08:59:59+09:00')";
    let test_cases = [
        ValueTestCase {
            description: "a zoned column against date text compares instants, the text at UTC",
            sql: "SELECT LISTAGG((at >= '2024-01-01')::VARCHAR, ',') WITHIN GROUP (ORDER BY at) \
                  AS r FROM events",
            expected_value: "false,true",
        },
        ValueTestCase {
            description: "a zoned column against T-separated text without an offset, at UTC",
            sql: "SELECT LISTAGG((at < '2024-01-01T00:00:00')::VARCHAR, ',') \
                  WITHIN GROUP (ORDER BY at) AS r FROM events",
            expected_value: "true,false",
        },
        ValueTestCase {
            description: "UNION ALL BY NAME with a column NULL in every branch",
            sql: "SELECT COUNT(*)::VARCHAR AS n FROM (SELECT 1 AS a, NULL AS b \
                  UNION ALL BY NAME SELECT 2 AS a) AS u WHERE b IS NULL",
            expected_value: "2",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            converted_value_over(test_case.sql, (schema, setup)),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_union_of_a_float_variant_and_parsed_json_when_running_then_types_hold() {
    let schema =
        r#"{"relations": {"READINGS": [["K", "NUMBER(2,0)"], ["F", "FLOAT"], ["J", "VARCHAR"]]}}"#;
    let setup = "CREATE TABLE \"READINGS\" (\"K\" INTEGER, \"F\" DOUBLE, \"J\" VARCHAR); \
                 INSERT INTO \"READINGS\" VALUES (1, 1.5, '7'), (2, -0.25, '{\"a\": \"x\"}')";
    let test_cases = [ValueTestCase {
        description: "a FLOAT keeps DOUBLE beside parsed JSON, whose whole number stays INTEGER",
        sql: "WITH b AS (SELECT k AS o, TO_VARIANT(f) AS v FROM readings UNION ALL \
              SELECT k + 10 AS o, PARSE_JSON(j) AS v FROM readings) \
              SELECT LISTAGG(o::VARCHAR || ':' || TYPEOF(v) || '=' || v::FLOAT::VARCHAR, ',') \
              WITHIN GROUP (ORDER BY o) AS r FROM b WHERE TYPEOF(v) <> 'OBJECT'",
        expected_value: "1:DOUBLE=1.5,2:DOUBLE=-0.25,11:INTEGER=7",
    }];
    for test_case in test_cases {
        assert_eq!(
            converted_value_over(test_case.sql, (schema, setup)),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_union_of_a_float_variant_and_a_json_decimal_when_running_then_it_raises() {
    let test_cases = [
        RunErrorTestCase {
            description: "a JSON decimal beside a FLOAT, which DuckDB's VARIANT would retype",
            sql: "WITH b AS (SELECT 1 AS o, TO_VARIANT(1.5::FLOAT) AS v UNION ALL \
                  SELECT 2, PARSE_JSON('2.5')) SELECT o, TYPEOF(v) AS t FROM b",
            expected_mention: "json_fidelity",
        },
        RunErrorTestCase {
            description: "a JSON null beside a FLOAT, which DuckDB's VARIANT would make SQL NULL",
            sql: "WITH b AS (SELECT 1 AS o, TO_VARIANT(1.5::FLOAT) AS v UNION ALL \
                  SELECT 2, PARSE_JSON('null')) SELECT o, TYPEOF(v) AS t FROM b",
            expected_mention: "json_fidelity",
        },
    ];
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
fn given_refused_sql_when_converting_then_text_and_json_carry_the_same_coded_diagnostic() {
    let test_cases = [
        ConvertDiagnosticTestCase {
            description: "a parse failure is rendered at the parser's position with its code",
            sql: "SELECT a,\n  FROM orders WHERE (b = 1\n",
            flags: &[],
            expected_text: "error[SQ0001]: the SQL does not parse\n --> <stdin>:2:26\n  |\n2 |   FROM \
                orders WHERE (b = 1\n  |                          ^ Expected RParen, got end of input \
                ('') near [b = 1]\nsqlquack convert: refused, 1 refusal under the built-in default\n",
            expected_json: ("SQ0001", "parse_fidelity", 2, 26),
        },
        ConvertDiagnosticTestCase {
            description: "an unknown relation is underlined with the fix as help",
            sql: "SELECT a FROM orders",
            flags: &[],
            expected_text: "error[SQ0004]: a name does not resolve\n --> <stdin>:1:15\n  |\n1 | SELECT a \
                FROM orders\n  |               ^^^^^^ relation `ORDERS` has no known columns\n  |\n  \
                = help: declare the relation and its columns in the schema (`--schema` on the command \
                line)\nsqlquack convert: refused, 1 refusal under the built-in default\n",
            expected_json: ("SQ0004", "unresolved_name", 1, 15),
        },
        ConvertDiagnosticTestCase {
            description: "SQL Snowflake parses but rejects is refused with the rejection's code, at its clause",
            sql: "SELECT 1 QUALIFY TRUE",
            flags: &[],
            expected_text: "error[SQ0203]: QUALIFY needs a window function\n --> <stdin>:1:10\n  |\n1 | \
                SELECT 1 QUALIFY TRUE\n  |          ^^^^^^^ QUALIFY is used without a window \
                function in the SELECT list or QUALIFY\n  |\n  = help: move the condition to WHERE or \
                HAVING, or filter on a window function's result\nsqlquack convert: refused, 1 refusal \
                under the built-in default\n",
            expected_json: ("SQ0203", "qualify_without_window", 1, 10),
        },
        ConvertDiagnosticTestCase {
            description: "an unaliased item has its own code, its own span and an alias as help",
            sql: "SELECT 'a' AS a, UPPER('é')",
            flags: &[],
            expected_text: concat!(
                "error[SQ0009]: this column has no alias, and SQLQuack has not verified the name ",
                "Snowflake gives it\n --> <stdin>:1:18\n  |\n1 | SELECT 'a' AS a, UPPER('é')\n",
                "  |                  ^^^^^^^^^^ Snowflake names this `function` column after its text\n",
                "  |\n  = help: add an alias, e.g. `AS one`\n",
                "sqlquack convert: refused, 1 refusal under the built-in default\n",
            ),
            expected_json: ("SQ0009", "unaliased_output_name", 1, 18),
        },
        ConvertDiagnosticTestCase {
            description: "a divergence the policy refuses has its own code and help naming the policy",
            sql: "SELECT HASH(1) AS h",
            flags: &[],
            expected_text: "error[SQ0008]: the conversion needs a known divergence the policy refuses: \
                `fn:HASH`\n --> <stdin>:1:8\n  |\n1 | SELECT HASH(1) AS h\n  |        ^^^^^^^ fn:HASH \
                needs known divergence `hash`, which the policy refuses in this mode\n  |\n  = help: \
                the built-in policy refuses every known divergence; pass --policy with a policy \
                file that allows `hash`, such as the repository's sqlquack.toml\nsqlquack convert: refused, 1 refusal under the built-in default\n",
            expected_json: ("SQ0008", "divergence_refused", 1, 8),
        },
        ConvertDiagnosticTestCase {
            description: "a TIMESTAMP_TZ local-time reading explains the dropped offset and suggests the rewrite",
            sql: "SELECT DATEDIFF('day', shipped_at, paid_at) AS days FROM orders",
            flags: &["--schema", DECLARED_SCHEMA],
            expected_text: "error[SQ0003]: the operand types fall outside every rule's proven \
                conditions: `fn:DATEDIFF`\n --> <stdin>:1:8\n  |\n1 | SELECT DATEDIFF('day', shipped_at, \
                paid_at) AS days FROM orders\n  |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ DATEDIFF in \
                DAY of TIMESTAMP_TZ(9), which reads the local time of an offset DuckDB's TIMESTAMPTZ \
                drops\n  |\n  = note: a Snowflake TIMESTAMP_TZ stores each value's own UTC offset, and \
                local dates, times, day boundaries and formats read that offset; DuckDB's TIMESTAMPTZ \
                keeps only the instant, so SQLQuack cannot reproduce them and adds no columns or hidden \
                state to keep the offset\n  = help: convert the value to a wall time first: \
                `CONVERT_TIMEZONE('UTC', col)::TIMESTAMP_NTZ` for UTC, or `CONVERT_TIMEZONE(<zone \
                column>, col)::TIMESTAMP_NTZ` for local day boundaries in a known zone\nsqlquack \
                convert: refused, 1 refusal under the built-in default\n",
            expected_json: ("SQ0003", "type_condition", 1, 8),
        },
        ConvertDiagnosticTestCase {
            description: "a schema key matching only ignoring case says unquoted names fold to upper case",
            sql: "SELECT order_id FROM orders",
            flags: &["--schema", "tests/cli/fixtures/lowercase_relation.json"],
            expected_text: "error[SQ0004]: a name does not resolve\n --> <stdin>:1:22\n  |\n1 | SELECT \
                order_id FROM orders\n  |                      ^^^^^^ relation `ORDERS` matches `orders` \
                only without regard to case\n  |\n  = help: Snowflake folds unquoted names to upper \
                case, so declare the relation or column in the schema as the SQL resolves it (`ORDERS` \
                for `FROM orders`), or quote the name in the SQL to keep its case\nsqlquack convert: \
                refused, 1 refusal under the built-in default\n",
            expected_json: ("SQ0004", "unresolved_name", 1, 22),
        },
    ];
    for test_case in test_cases {
        let (stdout, stderr) = convert_streams(
            test_case.sql,
            &[&["--builtin-policy"], test_case.flags].concat(),
        );
        assert_eq!(stdout, "", "{}", test_case.description);
        assert_eq!(
            stderr.trim_start_matches("sqlquack convert: converting…\n"),
            test_case.expected_text,
            "{}",
            test_case.description
        );
        let (json, _) = convert_streams(
            test_case.sql,
            &[&["--builtin-policy", "--json"], test_case.flags].concat(),
        );
        let report: serde_json::Value = serde_json::from_str(&json).expect("a JSON report");
        let refusal = &report["refusals"][0];
        assert_eq!(
            (
                refusal["code"].as_str(),
                refusal["name"].as_str(),
                refusal["span"]["line"].as_u64(),
                refusal["span"]["column"].as_u64(),
            ),
            (
                Some(test_case.expected_json.0),
                Some(test_case.expected_json.1),
                Some(test_case.expected_json.2),
                Some(test_case.expected_json.3),
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_output_flags_when_running_then_format_json_and_json_agree_and_help_lists_exit_status() {
    let test_cases = [
        OutputFormatTestCase {
            description: "--format json prints the JSON report",
            arguments: &["convert", "--builtin-policy", "--format", "json"],
            expected_exit_code: Some(0),
            expected_stdout_prefix: "{\n  \"converted\": true,",
        },
        OutputFormatTestCase {
            description: "--json is the short form and prints the same report",
            arguments: &["convert", "--builtin-policy", "--json"],
            expected_exit_code: Some(0),
            expected_stdout_prefix: "{\n  \"converted\": true,",
        },
        OutputFormatTestCase {
            description: "--format text prints the DuckDB SQL",
            arguments: &["convert", "--builtin-policy", "--format", "text"],
            expected_exit_code: Some(0),
            expected_stdout_prefix: "SELECT",
        },
        OutputFormatTestCase {
            description: "asking for both is a usage error",
            arguments: &["convert", "--builtin-policy", "--format", "text", "--json"],
            expected_exit_code: Some(2),
            expected_stdout_prefix: "",
        },
        OutputFormatTestCase {
            description: "the top-level help explains each exit status",
            arguments: &["--help"],
            expected_exit_code: Some(0),
            expected_stdout_prefix: "Snowflake-to-DuckDB SQL conversion that refuses to guess",
        },
    ];
    let directory = tempfile::tempdir().expect("temporary directory");
    for test_case in test_cases {
        let (exit_code, stdout, _) = run_with_input(
            directory.path(),
            test_case.arguments,
            "SELECT 1 + 2 AS total",
        );
        assert_eq!(
            (
                exit_code,
                stdout.starts_with(test_case.expected_stdout_prefix)
            ),
            (test_case.expected_exit_code, true),
            "{}: {stdout}",
            test_case.description
        );
        assert!(
            test_case.arguments != ["--help"]
                || stdout.contains(
                    "census and index exit 0 when they finish, refused files included, and 2 \
                     when they cannot run.",
                ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_loud_divergences_only_when_converting_then_value_divergences_refuse_and_loud_ones_convert()
{
    let test_cases = [
        FailureKindTestCase {
            description: "CURRENT_DATE relies on value divergences, and --json says so",
            sql: "SELECT CURRENT_DATE() AS d",
            flags: &[],
            expected_converted: true,
            expected_failures: &[("current_time", "value"), ("session_timezone_utc", "value")],
        },
        FailureKindTestCase {
            description: "--loud-divergences-only refuses CURRENT_DATE",
            sql: "SELECT CURRENT_DATE() AS d",
            flags: &["--loud-divergences-only"],
            expected_converted: false,
            expected_failures: &[],
        },
        FailureKindTestCase {
            description: "HASH is refused by default, as its value is never Snowflake's",
            sql: "SELECT HASH(x) AS h FROM (SELECT 1 AS x) AS t",
            flags: &[],
            expected_converted: false,
            expected_failures: &[],
        },
        FailureKindTestCase {
            description: "MODE relies only on loud divergences, so it still converts",
            sql: "SELECT MODE(x) AS m FROM (SELECT 1 AS x) AS t",
            flags: &["--loud-divergences-only"],
            expected_converted: true,
            expected_failures: &[("mode_of_no_rows", "loud"), ("mode_ties", "loud")],
        },
    ];
    for test_case in test_cases {
        let report = python_report(test_case.sql, test_case.flags);
        let expected: serde_json::Map<String, serde_json::Value> = test_case
            .expected_failures
            .iter()
            .map(|(id, failure)| ((*id).to_owned(), serde_json::json!(failure)))
            .collect();
        assert_eq!(
            (
                report["converted"].as_bool(),
                report["divergence_failures"].clone()
            ),
            (
                Some(test_case.expected_converted),
                serde_json::Value::Object(expected)
            ),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_numbers_beyond_38_digits_when_running_then_duckdb_raises_a_numeric_range_error() {
    let test_cases = [
        RunErrorTestCase {
            description: "a SUM near 2^127 overflows DuckDB's HUGEINT addition",
            sql: "SELECT SUM(x) AS s FROM (SELECT \
                  CAST('99999999999999999999999999999999999999' AS NUMBER(38,0)) AS x \
                  UNION ALL SELECT \
                  CAST('99999999999999999999999999999999999999' AS NUMBER(38,0))) AS t",
            expected_mention: "Out of Range Error: Overflow in HUGEINT addition",
        },
        RunErrorTestCase {
            description: "NUMBER(38,0) compared with NUMBER(10,5) casts to DECIMAL(38,5), which raises",
            sql: "SELECT a = b AS same FROM (SELECT \
              CAST('99999999999999999999999999999999999999' AS NUMBER(38,0)) AS a, \
              CAST('1.5' AS NUMBER(10,5)) AS b) AS t",
            expected_mention: "to type DECIMAL(38,5) failed: value is out of range!",
        },
    ];
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
fn given_a_long_flat_chain_when_converting_then_it_converts_or_is_refused_within_budget() {
    let test_cases = [
        DeepChainTestCase {
            description: "a long chain of additions is refused for its depth, not a stack overflow",
            operator: " + ",
            operand: "x",
            terms: 3_000,
            expected_exit: Some(1),
            expected_mention: "SQ0006",
        },
        DeepChainTestCase {
            description: "a chain far past the depth limit is refused before conversion recurses",
            operator: " - ",
            operand: "x",
            terms: 50_000,
            expected_exit: Some(1),
            expected_mention: "SQ0006",
        },
        DeepChainTestCase {
            description: "a long chain of || is refused for its depth, without quadratic memory",
            operator: " || ",
            operand: "s",
            terms: 3_000,
            expected_exit: Some(1),
            expected_mention: "SQ0006",
        },
        DeepChainTestCase {
            description: "a || chain within the depth limit still converts",
            operator: " || ",
            operand: "s",
            terms: 400,
            expected_exit: Some(0),
            expected_mention: "done, converted",
        },
        DeepChainTestCase {
            description: "a long AND chain, which the parser balances, still converts",
            operator: " AND ",
            operand: "b",
            terms: 3_000,
            expected_exit: Some(0),
            expected_mention: "done, converted",
        },
    ];
    for test_case in test_cases {
        let sql = format!(
            "SELECT {} AS y FROM (SELECT 1 AS x, 'a' AS s, TRUE AS b)",
            vec![test_case.operand; test_case.terms].join(test_case.operator)
        );
        let started = std::time::Instant::now();

        let outcome = run_convert(&sql, &["--builtin-policy"], test_case.expected_mention);

        assert_eq!(
            (outcome.exit_code, outcome.stderr_mentions),
            (test_case.expected_exit, true),
            "{}",
            test_case.description
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(20),
            "{}: took {:?}",
            test_case.description,
            started.elapsed()
        );
    }
}

#[test]
fn given_a_refused_divergence_when_converting_then_its_help_names_it_and_only_fixes_that_work() {
    let test_cases = [
        DivergenceHelpTestCase {
            description: "strict mode refusing what the default allows: --utc-session cannot help",
            sql: "SELECT CURRENT_TIMESTAMP AS t",
            flags: &["--strict", "--utc-session"],
            expected_help: "to rely on `current_time`: drop --strict, or set strict = \"warn\" \
                under [divergences.current_time] in <policy>",
        },
        DivergenceHelpTestCase {
            description: "refused by default: only the policy file can allow it",
            sql: "SELECT HASH(1) AS h",
            flags: &[],
            expected_help: "to rely on `hash`: set default = \"warn\" under [divergences.hash] in \
                <policy>",
        },
        DivergenceHelpTestCase {
            description: "refused in both modes under --strict: dropping --strict is not enough",
            sql: "SELECT HASH(1) AS h",
            flags: &["--strict"],
            expected_help: "to rely on `hash`: set default and strict = \"warn\" under \
                [divergences.hash] in <policy>",
        },
        DivergenceHelpTestCase {
            description: "a divergence a rule's type check needs is still SQ0008, with policy help",
            sql: "SELECT CAST(1 AS NUMBER(38,0)) - CAST(1 AS NUMBER(38,2)) AS d",
            flags: &["--strict"],
            expected_help: "to rely on `numeric_range`: drop --strict, or set strict = \"warn\" \
                under [divergences.numeric_range] in <policy>",
        },
        DivergenceHelpTestCase {
            description: "a value divergence under --loud-divergences-only",
            sql: "SELECT CURRENT_TIMESTAMP AS t",
            flags: &["--loud-divergences-only"],
            expected_help: "`current_time` can return a different value rather than raise, which \
                --loud-divergences-only refuses; drop that flag",
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            first_refusal_help(test_case.sql, test_case.flags),
            test_case.expected_help,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_malformed_schema_when_converting_then_the_error_names_what_is_wrong_in_schema_terms() {
    let test_cases = [
        SchemaErrorTestCase {
            description: "a column without a type",
            schema: r#"{"ORDERS": [["A"]]}"#,
            expected_error: "relation ORDERS, column 1 (A) has no type",
        },
        SchemaErrorTestCase {
            description: "an unknown key beside the sections",
            schema: r#"{"relations": {}, "tables": {}}"#,
            expected_error: "unknown top-level key `tables`",
        },
        SchemaErrorTestCase {
            description: "an unknown key in a column object",
            schema: r#"{"ORDERS": [{"name": "A", "type": "NUMBER", "nullable": true}]}"#,
            expected_error: "relation ORDERS, column 1 has unknown key `nullable`",
        },
        SchemaErrorTestCase {
            description: "relations not listed as columns",
            schema: r#"{"ORDERS": {"A": "NUMBER"}}"#,
            expected_error: "relation ORDERS must list its columns",
        },
    ];
    for test_case in test_cases {
        let (exit_code, stderr) = schema_failure(test_case.schema);

        assert_eq!(exit_code, Some(2), "{}: {stderr}", test_case.description);
        assert!(
            stderr.contains(test_case.expected_error) && !stderr.contains("untagged"),
            "{}: {stderr}",
            test_case.description
        );
    }
}
