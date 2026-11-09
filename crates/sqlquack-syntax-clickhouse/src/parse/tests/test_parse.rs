//! Verdicts and trees of the ClickHouse parser, each verdict confirmed against ClickHouse 26.1.

use serde_json::Value;

use crate::parse::main::parse::parse;
use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::{
    DepthTestCase, ExpectedVerdict, SMALL_STACK, ValueTestCase, VerdictTestCase,
};

#[test]
fn given_sql_when_parsing_then_the_verdict_is_clickhouses() {
    let test_cases = [
        VerdictTestCase {
            description: "the ternary operator is a call to if",
            sql: "SELECT a ? b : c FROM t",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a lambda body cannot itself be a bare lambda",
            sql: "SELECT x -> x -> 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a lambda over a parameter tuple is a function argument",
            sql: "SELECT arrayMap((x, y) -> x + y, a, b) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a parenthesised VALUES list is not a table source",
            sql: "SELECT * FROM (VALUES (1, 2))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an at sign does not start an expression",
            sql: "SELECT @'relation' AS r",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "LIKE takes no ESCAPE clause",
            sql: "SELECT x LIKE 'a' ESCAPE 'b' FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a backslash escapes a quote, so this string never closes",
            sql: "SELECT 'a' LIKE '\\_%' ESCAPE '\\'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an interval string must name known units",
            sql: "SELECT INTERVAL '1 fortnight'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an interval string may hold several units",
            sql: "SELECT INTERVAL '1 day 2 hour'",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a keyword is a valid alias after AS",
            sql: "SELECT 1 AS from FROM t",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "FINAL and SAMPLE ... OFFSET follow a table",
            sql: "SELECT * FROM t FINAL SAMPLE 1/10 OFFSET 1/2",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a parametric aggregate takes its parameters first",
            sql: "SELECT quantiles(0.5, 0.9)(x) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a cast type may be a string, and :: takes a nested type",
            sql: "SELECT CAST(1, 'Nullable(Int8)'), 1::Array(Nullable(Int8))",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "PREWHERE comes before WHERE, and QUALIFY after it",
            sql: "SELECT 1 FROM t AS t2 PREWHERE a WHERE b QUALIFY c",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "FORMAT and SETTINGS may close a query in either order",
            sql: "SELECT 1 FORMAT JSON SETTINGS max_threads = 1; SELECT 1 SETTINGS max_threads = 1 FORMAT JSON",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a MergeTree table with column codecs, keys, TTL and settings",
            sql: "CREATE TABLE t (a Nullable(Int64), b String DEFAULT 'x' COMMENT 'c', c LowCardinality(String) CODEC(ZSTD(1))) ENGINE = ReplacingMergeTree(v) PARTITION BY toYYYYMM(d) ORDER BY (a, b) TTL d + INTERVAL 1 DAY SETTINGS index_granularity = 8192",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a lambda function definition with a nullable cast",
            sql: "CREATE OR REPLACE FUNCTION f AS (x) -> CAST((x) AS Nullable(Float64))",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "partition actions are a list",
            sql: "ALTER TABLE t DROP PARTITION ID '1', DROP PARTITION ID '2'",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "EXCHANGE TABLES, KILL QUERY and DROP ... SYNC",
            sql: "EXCHANGE TABLES a AND b; KILL QUERY WHERE query_id IN ('x') ASYNC; DROP TABLE IF EXISTS a.b SYNC",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "INSERT may end at its FORMAT, the data following outside the query",
            sql: "INSERT INTO t FORMAT CSV",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a lightweight DELETE takes SETTINGS",
            sql: "DELETE FROM t WHERE x IN (1) SETTINGS transform_null_in = 1",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a qualified USING column is accepted",
            sql: "SELECT * FROM a JOIN b USING (a.k)",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "a USING expression is accepted, but the shared tree has no place for it",
            sql: "SELECT * FROM a JOIN b USING (k + 1)",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("using_expression"),
            },
        },
        VerdictTestCase {
            description: "a LIMIT after LIMIT BY is accepted, but the shared tree has one limit",
            sql: "SELECT a FROM t ORDER BY a LIMIT 1 BY a LIMIT 5",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("limit_after_limit_by"),
            },
        },
        VerdictTestCase {
            description: "a query parameter needs a binding, so it is a gap",
            sql: "SELECT {p:String}",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("query_parameter"),
            },
        },
        VerdictTestCase {
            description: "GLOBAL and ANY on a join are a gap",
            sql: "SELECT a, b FROM t GLOBAL ANY LEFT JOIN u USING a",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("join_locality_or_strictness"),
            },
        },
        VerdictTestCase {
            description: "a statement SQLBuild never generates is a gap",
            sql: "SYSTEM FLUSH LOGS",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("unsupported_statement"),
            },
        },
        VerdictTestCase {
            description: "a ternary may not be followed by another ternary or OR without brackets",
            sql: "SELECT a ? 1 : 0 ? 1 : 2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "OR after a ternary's else operand is refused",
            sql: "SELECT a ? 1 : 0 OR b",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a bracketed ternary may be a condition, and AND binds into the else operand",
            sql: "SELECT (a ? 1 : 0) ? 1 : 2, a ? 1 : 0 AND b, b OR a ? 1 : 0",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "one trailing semicolon, then only a comment",
            sql: "SELECT 1; -- c",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "an empty statement between semicolons is an empty query",
            sql: "SELECT 1; /* c */ ; SELECT 2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a leading semicolon is an empty query",
            sql: ";SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a comment alone is an empty query",
            sql: "-- only a comment",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a query cannot stop after SELECT",
            sql: "SELECT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TRUNCATE's TABLE keyword is optional",
            sql: "TRUNCATE analytics.orders",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "TRUNCATE keeps IF EXISTS, ON CLUSTER and SYNC",
            sql: "TRUNCATE TABLE IF EXISTS orders ON CLUSTER main SYNC",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "TRUNCATE takes one table",
            sql: "TRUNCATE TABLE orders, customers",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TRUNCATE takes no CASCADE",
            sql: "TRUNCATE TABLE orders CASCADE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a table name has at most two parts",
            sql: "TRUNCATE TABLE a.b.c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an ALTER command starts with a keyword",
            sql: "ALTER TABLE a.b.c DELETE WHERE 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TRUNCATE DATABASE is a gap",
            sql: "TRUNCATE DATABASE analytics",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("unsupported_statement"),
            },
        },
        VerdictTestCase {
            description: "TRUNCATE ... NO DELAY is a gap",
            sql: "TRUNCATE TABLE orders NO DELAY",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("truncate_no_delay"),
            },
        },
        VerdictTestCase {
            description: "SETTINGS after a TRUNCATE is a gap",
            sql: "TRUNCATE TABLE orders SETTINGS a = 1",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("statement_output"),
            },
        },
        VerdictTestCase {
            description: "OPTIMIZE takes FINAL, DEDUPLICATE and CLEANUP in that order",
            sql: "OPTIMIZE TABLE orders PARTITION ID '202401' FINAL DEDUPLICATE CLEANUP",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "OPTIMIZE needs TABLE",
            sql: "OPTIMIZE orders FINAL",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "OPTIMIZE takes FINAL or FORCE, not both",
            sql: "OPTIMIZE TABLE orders FINAL FORCE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "OPTIMIZE takes DEDUPLICATE after FINAL",
            sql: "OPTIMIZE TABLE orders DEDUPLICATE FINAL",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "OPTIMIZE ... DEDUPLICATE BY is a gap",
            sql: "OPTIMIZE TABLE orders DEDUPLICATE BY * EXCEPT ts",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("optimize_deduplicate_by"),
            },
        },
        VerdictTestCase {
            description: "a temporary table can be created from a query",
            sql: "CREATE OR REPLACE TEMPORARY TABLE scratch ENGINE = Memory AS SELECT * FROM orders",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "CREATE TEMPORARY needs TABLE or VIEW",
            sql: "CREATE TEMPORARY scratch AS SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("unsupported_statement"),
            },
        },
        VerdictTestCase {
            description: "ALTER ... DELETE takes a subquery in its predicate",
            sql: "ALTER TABLE orders DELETE WHERE id IN (SELECT id FROM analytics.customers)",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "ALTER ... DELETE needs WHERE",
            sql: "ALTER TABLE orders DELETE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a mutation predicate takes no alias, even in brackets",
            sql: "ALTER TABLE orders DELETE WHERE (1 AS x)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER ... DELETE IN PARTITION is a gap",
            sql: "ALTER TABLE orders DELETE IN PARTITION 202401 WHERE id = 1",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("alter_in_partition"),
            },
        },
        VerdictTestCase {
            description: "ALTER ... UPDATE assigns several columns, with subqueries",
            sql: "ALTER TABLE orders UPDATE a = 1, b = (SELECT 2) WHERE c IN (SELECT d FROM e)",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "ALTER ... UPDATE needs WHERE",
            sql: "ALTER TABLE orders UPDATE status = 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER ... UPDATE assigns to a bare column name",
            sql: "ALTER TABLE orders UPDATE o.status = 'x' WHERE 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SETTINGS after an ALTER is a gap",
            sql: "ALTER TABLE orders DELETE WHERE id = 1 SETTINGS mutations_sync = 2",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("statement_output"),
            },
        },
        VerdictTestCase {
            description: "INTO OUTFILE takes APPEND, TRUNCATE, AND STDOUT and COMPRESSION in that order",
            sql: "SELECT * FROM orders INTO OUTFILE 'x.csv.gz' APPEND TRUNCATE AND STDOUT COMPRESSION 'gzip' LEVEL -1 FORMAT CSV",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "INTO OUTFILE takes AND STDOUT after APPEND",
            sql: "SELECT * FROM orders INTO OUTFILE 'x.csv' AND STDOUT APPEND",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "INTO OUTFILE takes a string",
            sql: "SELECT * FROM orders INTO OUTFILE x",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "INTO OUTFILE comes before FORMAT",
            sql: "SELECT * FROM orders FORMAT CSV INTO OUTFILE 'x.csv'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a compression level is a number",
            sql: "SELECT * FROM orders INTO OUTFILE 'x.csv' COMPRESSION 'gzip' LEVEL 'a'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "INTO OUTFILE after a set operation is a gap",
            sql: "SELECT 1 UNION ALL SELECT 2 INTO OUTFILE 'x.csv'",
            expected_verdict: ExpectedVerdict::Accepted {
                gap: Some("into_outfile"),
            },
        },
        VerdictTestCase {
            description: "a table in FROM has at most two parts",
            sql: "SELECT * FROM a.b.c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a joined table has at most two parts",
            sql: "SELECT * FROM t JOIN a.b.c ON 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a table function may have a three-part name",
            sql: "SELECT * FROM a.b.c(1)",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "an ARRAY JOIN expression may have three parts",
            sql: "SELECT * FROM t ARRAY JOIN a.b.c",
            expected_verdict: ExpectedVerdict::Accepted { gap: None },
        },
        VerdictTestCase {
            description: "DESCRIBE takes a table of at most two parts",
            sql: "DESCRIBE a.b.c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "INSERT takes a table of at most two parts",
            sql: "INSERT INTO a.b.c VALUES (1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DROP takes a table of at most two parts",
            sql: "DROP TABLE a.b.c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
    ];
    for test_case in test_cases {
        let verdict: ExpectedVerdict = parse(test_case.sql)
            .map(|parsed| ExpectedVerdict::Accepted {
                gap: parsed.gaps.first().map(|gap| gap.construct),
            })
            .unwrap_or_else(|error| ExpectedVerdict::Rejected {
                syntax: error.syntax,
            });
        assert_eq!(
            verdict, test_case.expected_verdict,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_parsing_then_the_tree_keeps_what_clickhouse_parsed() {
    let test_cases = [
        ValueTestCase {
            description: "* binds tighter than +",
            sql: "SELECT 1 + 2 * 3",
            pointer: "/0/select/expressions/0/add/right/mul/right/literal/value",
            expected_json: "\"3\"",
        },
        ValueTestCase {
            description: "NOT binds looser than =",
            sql: "SELECT NOT a = b",
            pointer: "/0/select/expressions/0/not/this/eq/right/column/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "the ternary operator is a call to if",
            sql: "SELECT a ? b : c",
            pointer: "/0/select/expressions/0/function/args/2/column/name/name",
            expected_json: "\"c\"",
        },
        ValueTestCase {
            description: "a keyword alias after AS keeps its name",
            sql: "SELECT 1 AS from FROM t",
            pointer: "/0/select/expressions/0/alias/alias/name",
            expected_json: "\"from\"",
        },
        ValueTestCase {
            description: "FINAL is kept on its table",
            sql: "SELECT * FROM t FINAL",
            pointer: "/0/select/from/expressions/0/table/final_",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a parametric aggregate keeps its parameters apart from its arguments",
            sql: "SELECT quantiles(0.5)(x) FROM t",
            pointer: "/0/select/expressions/0/parameterized_agg/params/0/literal/value",
            expected_json: "\"0.5\"",
        },
        ValueTestCase {
            description: "a :: cast keeps its type as written",
            sql: "SELECT 1::Array(Nullable(Int8))",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"Array(Nullable(Int8))\"",
        },
        ValueTestCase {
            description: "LIMIT n BY keeps its columns",
            sql: "SELECT a FROM t ORDER BY a LIMIT 1 BY a",
            pointer: "/0/select/limit_by/0/column/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "DELETE keeps its settings",
            sql: "DELETE FROM t WHERE x = 1 SETTINGS lightweight_deletes_sync = 2",
            pointer: "/0/delete/settings/0/eq/left/column/name/name",
            expected_json: "\"lightweight_deletes_sync\"",
        },
        ValueTestCase {
            description: "INSERT ... FORMAT keeps the format as its query",
            sql: "INSERT INTO t FORMAT CSV",
            pointer: "/0/insert/query/command/this",
            expected_json: "\"FORMAT CSV\"",
        },
        ValueTestCase {
            description: "a qualified USING column keeps its last part",
            sql: "SELECT * FROM a JOIN b USING (a.k)",
            pointer: "/0/select/joins/0/using/0/name",
            expected_json: "\"k\"",
        },
        ValueTestCase {
            description: "TRUNCATE keeps its table",
            sql: "TRUNCATE TABLE analytics.orders",
            pointer: "/0/truncate/table/schema/name",
            expected_json: "\"analytics\"",
        },
        ValueTestCase {
            description: "a temporary table is marked temporary",
            sql: "CREATE TEMPORARY TABLE scratch AS SELECT * FROM orders",
            pointer: "/0/create_table/temporary",
            expected_json: "true",
        },
        ValueTestCase {
            description: "ALTER ... DELETE keeps its predicate",
            sql: "ALTER TABLE orders DELETE WHERE id = 1",
            pointer: "/0/alter_table/actions/0/Delete/where_clause/eq/left/column/name/name",
            expected_json: "\"id\"",
        },
        ValueTestCase {
            description: "INTO OUTFILE keeps its file",
            sql: "SELECT * FROM orders INTO OUTFILE 'x.csv' AND STDOUT",
            pointer: "/0/select/into/this/literal/value",
            expected_json: "\"x.csv\"",
        },
        ValueTestCase {
            description: "INTO OUTFILE keeps its modifiers",
            sql: "SELECT * FROM orders INTO OUTFILE 'x.csv' AND STDOUT",
            pointer: "/0/select/into/expressions/0/var/this",
            expected_json: "\"AND STDOUT\"",
        },
        ValueTestCase {
            description: "OPTIMIZE reads whole as a command",
            sql: "OPTIMIZE TABLE orders FINAL",
            pointer: "/0/command/this",
            expected_json: "\"OPTIMIZE TABLE orders FINAL\"",
        },
        ValueTestCase {
            description: "an ALTER with an UPDATE reads whole as a command",
            sql: "ALTER TABLE orders UPDATE a = 1 WHERE 1",
            pointer: "/0/command/this",
            expected_json: "\"ALTER TABLE orders UPDATE a = 1 WHERE 1\"",
        },
    ];
    for test_case in test_cases {
        let parsed = parse_strict(test_case.sql).expect(test_case.description);
        assert!(parsed.gaps.is_empty(), "{}", test_case.description);
        let tree: Value = serde_json::to_value(&parsed.statements).expect(test_case.description);
        assert_eq!(
            tree.pointer(test_case.pointer)
                .map(Value::to_string)
                .as_deref(),
            Some(test_case.expected_json),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_nesting_on_a_2_mb_stack_when_parsing_and_serializing_then_deep_input_is_refused_cleanly() {
    let test_cases = [
        DepthTestCase {
            description: "parentheses, 60 deep",
            build: |n| format!("SELECT {}1{}", "(".repeat(n), ")".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "parentheses, 1500 deep",
            build: |n| format!("SELECT {}1{}", "(".repeat(n), ")".repeat(n)),
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "array literals, 60 deep",
            build: |n| format!("SELECT {}1{}", "[".repeat(n), "]".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "array literals, 1500 deep",
            build: |n| format!("SELECT {}1{}", "[".repeat(n), "]".repeat(n)),
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "function calls, 60 deep",
            build: |n| format!("SELECT {}1{}", "f(".repeat(n), ")".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "function calls, 1500 deep",
            build: |n| format!("SELECT {}1{}", "f(".repeat(n), ")".repeat(n)),
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "alternating NOT and minus, 60 deep",
            build: |n| format!("SELECT {}1", "NOT - ".repeat(n / 2)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "alternating NOT and minus, 1500 deep",
            build: |n| format!("SELECT {}1", "NOT - ".repeat(n / 2)),
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "subqueries, 60 deep",
            build: |n| {
                format!(
                    "SELECT * FROM {}SELECT 1{}",
                    "(SELECT * FROM (".repeat(n),
                    "))".repeat(n)
                )
            },
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "subqueries, 1500 deep",
            build: |n| {
                format!(
                    "SELECT * FROM {}SELECT 1{}",
                    "(SELECT * FROM (".repeat(n),
                    "))".repeat(n)
                )
            },
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "CASE in CASE, 60 deep",
            build: |n| {
                format!(
                    "SELECT {}1{}",
                    "CASE WHEN 1 THEN ".repeat(n),
                    " END".repeat(n)
                )
            },
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "CASE in CASE, 1500 deep",
            build: |n| {
                format!(
                    "SELECT {}1{}",
                    "CASE WHEN 1 THEN ".repeat(n),
                    " END".repeat(n)
                )
            },
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "alternating Array and Nullable types, 60 deep",
            build: |n| {
                format!(
                    "SELECT CAST(1 AS {}Int8{})",
                    "Array(Nullable(".repeat(n / 2),
                    "))".repeat(n / 2)
                )
            },
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "alternating Array and Nullable types, 1500 deep",
            build: |n| {
                format!(
                    "SELECT CAST(1 AS {}Int8{})",
                    "Array(Nullable(".repeat(n / 2),
                    "))".repeat(n / 2)
                )
            },
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "alternating Array and Tuple types after ::, 60 deep",
            build: |n| {
                format!(
                    "SELECT 1::{}Int8{}",
                    "Array(Tuple(".repeat(n / 2),
                    "))".repeat(n / 2)
                )
            },
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "alternating Array and Tuple types after ::, 1500 deep",
            build: |n| {
                format!(
                    "SELECT 1::{}Int8{}",
                    "Array(Tuple(".repeat(n / 2),
                    "))".repeat(n / 2)
                )
            },
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "named Tuple column types, 60 deep",
            build: |n| {
                format!(
                    "CREATE TABLE t (c {}Int8{}) ENGINE = Memory",
                    "Tuple(a ".repeat(n),
                    ")".repeat(n)
                )
            },
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "named Tuple column types, 1500 deep",
            build: |n| {
                format!(
                    "CREATE TABLE t (c {}Int8{}) ENGINE = Memory",
                    "Tuple(a ".repeat(n),
                    ")".repeat(n)
                )
            },
            depth: 1500,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "parentheses, 100000 deep",
            build: |n| format!("SELECT {}1{}", "(".repeat(n), ")".repeat(n)),
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "alternating Array and Nullable types, 100000 deep",
            build: |n| {
                format!(
                    "SELECT CAST(1 AS {}Int8{})",
                    "Array(Nullable(".repeat(n / 2),
                    "))".repeat(n / 2)
                )
            },
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "named Tuple column types, 100000 deep",
            build: |n| {
                format!(
                    "CREATE TABLE t (c {}Int8{}) ENGINE = Memory",
                    "Tuple(a ".repeat(n),
                    ")".repeat(n)
                )
            },
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of parenthesised ternaries, 60 deep",
            build: |n| format!("SELECT {}0", "(a ? 1 : ".repeat(n)) + &")".repeat(n),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a chain of parenthesised ternaries, 5000 deep",
            build: |n| format!("SELECT {}0", "(a ? 1 : ".repeat(n)) + &")".repeat(n),
            depth: 5000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of parenthesised ternaries, 100000 deep",
            build: |n| format!("SELECT {}0", "(a ? 1 : ".repeat(n)) + &")".repeat(n),
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of tuple indexes, 60 deep",
            build: |n| format!("SELECT x{}", ".1".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a chain of tuple indexes, 5000 deep",
            build: |n| format!("SELECT x{}", ".1".repeat(n)),
            depth: 5000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of tuple indexes, 100000 deep",
            build: |n| format!("SELECT x{}", ".1".repeat(n)),
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of :: casts, 60 deep",
            build: |n| format!("SELECT 1{}", "::Int32".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a chain of :: casts, 5000 deep",
            build: |n| format!("SELECT 1{}", "::Int32".repeat(n)),
            depth: 5000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of :: casts, 100000 deep",
            build: |n| format!("SELECT 1{}", "::Int32".repeat(n)),
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of subscripts, 60 deep",
            build: |n| format!("SELECT x{}", "[1]".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a chain of subscripts, 5000 deep",
            build: |n| format!("SELECT x{}", "[1]".repeat(n)),
            depth: 5000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of subscripts, 100000 deep",
            build: |n| format!("SELECT x{}", "[1]".repeat(n)),
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of IS NULL tests, 60 deep",
            build: |n| format!("SELECT a{}", " IS NULL".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a chain of IS NULL tests, 5000 deep",
            build: |n| format!("SELECT a{}", " IS NULL".repeat(n)),
            depth: 5000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of IS NULL tests, 100000 deep",
            build: |n| format!("SELECT a{}", " IS NULL".repeat(n)),
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of UNION ALL, 60 deep",
            build: |n| format!("SELECT 1{}", " UNION ALL SELECT 1".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a chain of UNION ALL, 5000 deep",
            build: |n| format!("SELECT 1{}", " UNION ALL SELECT 1".repeat(n)),
            depth: 5000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of UNION ALL, 100000 deep",
            build: |n| format!("SELECT 1{}", " UNION ALL SELECT 1".repeat(n)),
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a dotted column name, 60 deep",
            build: |n| format!("SELECT x{}", ".a".repeat(n)),
            depth: 60,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a dotted column name, 5000 deep",
            build: |n| format!("SELECT x{}", ".a".repeat(n)),
            depth: 5000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a dotted column name, 100000 deep",
            build: |n| format!("SELECT x{}", ".a".repeat(n)),
            depth: 100_000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a flat chain of +, 150 deep",
            build: |n| format!("SELECT 1{}", " + 1".repeat(n)),
            depth: 150,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a flat chain of AND, 150 deep",
            build: |n| format!("SELECT 1{}", " AND 1".repeat(n)),
            depth: 150,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a chain of 10 :: casts in each of 10 nested parentheses",
            build: |n| {
                format!(
                    "SELECT {}x{}",
                    "(".repeat(n),
                    format!("{})", "::Int32".repeat(n)).repeat(n)
                )
            },
            depth: 10,
            expected_outcome: Ok(1),
        },
        DepthTestCase {
            description: "a chain of 60 :: casts in each of 60 nested parentheses",
            build: |n| {
                format!(
                    "SELECT {}x{}",
                    "(".repeat(n),
                    format!("{})", "::Int32".repeat(n)).repeat(n)
                )
            },
            depth: 60,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of 120 tuple indexes in each of 30 nested parentheses",
            build: |n| {
                format!(
                    "SELECT {}x{}",
                    "(".repeat(n / 4),
                    format!("{})", ".1".repeat(n)).repeat(n / 4)
                )
            },
            depth: 120,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of 120 subscripts in each of 30 nested parentheses",
            build: |n| {
                format!(
                    "SELECT {}x{}",
                    "(".repeat(n / 4),
                    format!("{})", "[1]".repeat(n)).repeat(n / 4)
                )
            },
            depth: 120,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of 120 IS NULL tests in each of 30 nested parentheses",
            build: |n| {
                format!(
                    "SELECT {}x{}",
                    "(".repeat(n / 4),
                    format!("{})", " IS NULL".repeat(n)).repeat(n / 4)
                )
            },
            depth: 120,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a chain of 120 UNION ALL in each of 30 nested parenthesised queries",
            build: |n| {
                let mut sql: String = "SELECT 1".to_owned();
                for _ in 0..n / 4 {
                    sql = format!("({sql}){}", " UNION ALL SELECT 1".repeat(n));
                }
                sql
            },
            depth: 120,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "position(x IN s) over 60 :: casts in each of 60 nested parentheses",
            build: |n| {
                let deep: String = format!(
                    "{}x{}",
                    "(".repeat(n),
                    format!("{})", "::Int32".repeat(n)).repeat(n)
                );
                format!("SELECT position({deep} IN 'a') FROM t")
            },
            depth: 60,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "an aggregate FILTER over 60 :: casts in each of 60 nested parentheses",
            build: |n| {
                let deep: String = format!(
                    "{}x{}",
                    "(".repeat(n),
                    format!("{})", "::Int32".repeat(n)).repeat(n)
                );
                format!("SELECT count() FILTER (WHERE {deep}) FROM t")
            },
            depth: 60,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
        DepthTestCase {
            description: "a derived table of 2000 UNION ALL branches",
            build: |n| {
                format!(
                    "SELECT * FROM (SELECT 1{}) s",
                    " UNION ALL SELECT 1".repeat(n)
                )
            },
            depth: 2000,
            expected_outcome: Err("Maximum parse depth exceeded"),
        },
    ];
    for test_case in test_cases {
        let sql: String = (test_case.build)(test_case.depth);
        let outcome: Result<usize, String> = std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn(move || {
                parse(&sql)
                    .map(|parsed| {
                        serde_json::to_string(&parsed.statements)
                            .map_or(0, |_| parsed.statements.len())
                    })
                    .map_err(|error| error.message)
            })
            .expect(test_case.description)
            .join()
            .expect(test_case.description);
        assert_eq!(
            outcome,
            test_case.expected_outcome.map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
