//! Verdicts and trees of the Databricks SQL parser, following Spark 4.2.0's SqlBaseParser.g4.

use sqlquack_syntax::{Gap, Parsed};

use crate::parse::errors::StrictFailure;
use crate::parse::main::parse::parse;
use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::{
    ExpectedVerdict, GapTestCase, RefusalTestCase, StrictFailureTestCase, StrictTestCase,
    VerdictTestCase,
};

#[test]
fn given_sql_when_parsing_then_the_verdict_is_databricks() {
    let test_cases = [
        VerdictTestCase {
            description: "an empty input is refused, as PARSE_EMPTY_STATEMENT",
            sql: "",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TIMESTAMP takes one time zone clause, not WITHOUT and then WITH LOCAL",
            sql: "SELECT CAST ( v1 AS TIMESTAMP WITHOUT TIME ZONE WITH LOCAL TIME ZONE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TIMESTAMP WITHOUT TIME ZONE alone",
            sql: "SELECT CAST ( v1 AS TIMESTAMP WITHOUT TIME ZONE )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "TIMESTAMP WITH LOCAL TIME ZONE alone",
            sql: "SELECT CAST ( v1 AS TIMESTAMP WITH LOCAL TIME ZONE )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "STREAM's parentheses hold a table name or a table function call, with no alias",
            sql: "SELECT * FROM STREAM ( t1 a1 )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "STREAM's parentheses hold no query",
            sql: "SELECT * FROM STREAM ( SELECT 1 )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "STREAM's parentheses may hold a table function call (STREAM ( tableFunctionCall ))",
            sql: "SELECT * FROM STREAM ( range ( 1 ) )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a table function call in STREAM's parentheses takes the alias after them",
            sql: "SELECT * FROM STREAM ( read_files ( '/x' ) ) a1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "STREAM's parentheses may hold an IDENTIFIER clause",
            sql: "SELECT * FROM STREAM ( IDENTIFIER ( 't1' ) )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a WATERMARK after a table function call in STREAM's parentheses is part of the tree",
            sql: "SELECT * FROM STREAM ( range ( 1 ) ) WATERMARK c1 DELAY OF INTERVAL 1 SECOND",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ANTI is strict non-reserved, so it cannot be a bare table alias",
            sql: "SELECT * FROM t anti",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a non-reserved word such as DATE is a bare table alias",
            sql: "SELECT * FROM t date",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "LEFT ANTI JOIN and LEFT SEMI JOIN are joins",
            sql: "SELECT * FROM a LEFT ANTI JOIN b ON a.x = b.x LEFT SEMI JOIN c ON a.x = c.x",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a pattern quantifier needs a minimum or a maximum",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( a { , } ) DEFINE a AS TRUE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "MINUS is EXCEPT",
            sql: "SELECT 1 MINUS SELECT 2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "pipe operators chain onto a query",
            sql: "FROM t |> WHERE a > 1 |> SELECT a, b |> ORDER BY a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "?:: is a try cast",
            sql: "SELECT a?::INT FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "numbers take the L, S, Y, D, F and BD suffixes",
            sql: "SELECT 1L, 2S, 3Y, 4D, 5.0F, 6BD",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a hint comment heads the select list",
            sql: "SELECT /*+ BROADCAST(b) */ a.x FROM a JOIN b ON a.x = b.x",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "lambdas, struct access, subscripts and FILTER are expressions",
            sql: "SELECT filter(arr, x -> x > 0), s.f, m['k'], arr[0], \
                  count(DISTINCT a) FILTER (WHERE b) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "GROUP BY ALL, ROLLUP and GROUPING SETS, with QUALIFY",
            sql: "SELECT a, b, sum(c) OVER (PARTITION BY a ORDER BY b ROWS BETWEEN \
                  UNBOUNDED PRECEDING AND CURRENT ROW) AS s FROM t GROUP BY GROUPING SETS ((a), (a, b)) \
                  QUALIFY s > 0",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REGEXP is RLIKE, and both take NOT",
            sql: "SELECT a REGEXP b, a NOT RLIKE b, a NOT ILIKE ANY ('x%', 'y%') FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "PIVOT names its aggregate and its IN list",
            sql: "SELECT * FROM t PIVOT (sum(v) FOR k IN ('a', 'b'))",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an empty TABLESAMPLE is refused",
            sql: "SELECT * FROM t TABLESAMPLE ( )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "INSERT REPLACE WHERE takes a query",
            sql: "INSERT INTO t REPLACE WHERE d = '2024-01-01' SELECT * FROM s",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MERGE needs at least one WHEN clause",
            sql: "MERGE INTO t USING s ON t.id = s.id",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "MERGE takes every WHEN clause kind",
            sql: "MERGE INTO t USING s ON t.id = s.id WHEN MATCHED AND s.d THEN DELETE \
                  WHEN MATCHED THEN UPDATE SET * WHEN NOT MATCHED THEN INSERT * \
                  WHEN NOT MATCHED BY SOURCE THEN DELETE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE takes its clauses in any order",
            sql: "CREATE OR REPLACE TABLE c.s.t (a INT NOT NULL COMMENT 'x', b ARRAY<STRUCT<f: \
                  DECIMAL(10, 2)>>) USING DELTA CLUSTER BY AUTO COMMENT 'y' \
                  TBLPROPERTIES ('k' = 'v') LOCATION '/p'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE refuses a duplicate clause",
            sql: "CREATE TABLE t (a INT) COMMENT 'x' COMMENT 'y'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CTAS takes no partition column types",
            sql: "CREATE TABLE t PARTITIONED BY (c2 INT) AS SELECT 1 AS c2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a temporary view takes no TBLPROPERTIES",
            sql: "CREATE TEMPORARY VIEW v TBLPROPERTIES ('k' = 'v') AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a view cannot be both OR REPLACE and IF NOT EXISTS",
            sql: "CREATE OR REPLACE VIEW IF NOT EXISTS v AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DROP TABLE IF EXISTS is structured",
            sql: "DROP TABLE IF EXISTS c.s.t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DROP of an unknown object kind is refused",
            sql: "DROP TEMP t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a statement the shared tree has no node for is kept as a gap",
            sql: "REPAIR PRIVILEGES t1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a documented CREATE with no node of its own is a tree of its clauses",
            sql: "CREATE CATALOG IF NOT EXISTS c",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a dollar-placeholder is not Databricks SQL",
            sql: "SELECT ${1} FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unknown data type is a parser check, not a syntax refusal",
            sql: "SELECT CAST(a AS NOSUCHTYPE) FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "FROM is non-reserved, so a bare FROM is a column",
            sql: "SELECT FROM",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "WHERE needs a condition",
            sql: "SELECT a FROM t WHERE a =",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a view takes Databricks' COPY * clause",
            sql: "CREATE OR REPLACE VIEW v COPY * AS SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "NOCOPY * parses and is kept",
            sql: "CREATE OR REPLACE VIEW v NOCOPY * AS SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a metric view refuses COPY *",
            sql: "CREATE OR REPLACE VIEW v WITH METRICS LANGUAGE YAML COPY * AS $$x$$",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a temporary view refuses IF NOT EXISTS",
            sql: "CREATE TEMPORARY VIEW IF NOT EXISTS v AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ALTER TABLE adds columns, renames and sets properties",
            sql: "ALTER TABLE t ADD COLUMNS (a INT COMMENT 'x', b STRING)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER TABLE ADD needs COLUMN before a column",
            sql: "ALTER TABLE t ADD c INT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER COLUMN needs an action",
            sql: "ALTER TABLE t ALTER COLUMN c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER TABLE DROP FEATURE keeps its feature and TRUNCATE HISTORY",
            sql: "ALTER TABLE t DROP FEATURE deletionVectors TRUNCATE HISTORY",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DROP FEATURE TRUNCATE needs HISTORY",
            sql: "ALTER TABLE t DROP FEATURE deletionVectors TRUNCATE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER TABLE SET TBLPROPERTIES needs values",
            sql: "ALTER TABLE t SET TBLPROPERTIES ('k')",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER VIEW takes a new query",
            sql: "ALTER VIEW v AS SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER VIEW refuses a bare TAGS",
            sql: "ALTER VIEW v TAGS ('k' = 'v')",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER SCHEMA sets properties",
            sql: "ALTER SCHEMA s SET DBPROPERTIES ('k' = 'v')",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DESCRIBE HISTORY and DESCRIBE QUERY are describes",
            sql: "DESCRIBE HISTORY t; DESCRIBE QUERY SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SHOW TABLES takes a scope and a pattern",
            sql: "SHOW TABLES IN s LIKE 'o*'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SET takes a configuration, the time zone and variables",
            sql: "SET a.b = 1; SET TIME ZONE 'UTC'; SET VARIABLE v = 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "USE CATALOG and COMMENT ON parse",
            sql: "USE CATALOG main; COMMENT ON TABLE t IS 'x'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "TRUNCATE needs TABLE",
            sql: "TRUNCATE t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COPY INTO checks its clauses",
            sql: "COPY INTO t FROM '/x' FILEFORMAT = PARQUET COPY_OPTIONS ('mergeSchema' = 'true')",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "COPY INTO refuses an empty column list",
            sql: "COPY INTO t ( ) FROM '/x' FILEFORMAT = PARQUET",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COPY INTO VALIDATE n needs ROWS",
            sql: "COPY INTO t FROM '/x' FILEFORMAT = PARQUET VALIDATE 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "OPTIMIZE is a documented form",
            sql: "OPTIMIZE t WHERE d > 1 ZORDER BY (a)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "VACUUM DRY needs RUN",
            sql: "VACUUM t DRY",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "RESTORE needs AS OF",
            sql: "RESTORE TABLE t TO VERSION 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "EXPLAIN wraps a statement",
            sql: "EXPLAIN EXTENDED SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "GRANT and REVOKE take privileges, a securable and principals",
            sql: "GRANT SELECT, MODIFY ON TABLE t TO `a`; REVOKE ALL PRIVILEGES ON SCHEMA s FROM `a`",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DECLARE needs a type or a default",
            sql: "DECLARE v",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "CACHE TABLE takes a query",
            sql: "CACHE LAZY TABLE c AS SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE LIKE and CLONE are tables",
            sql: "CREATE TABLE a LIKE b; CREATE TABLE c SHALLOW CLONE d VERSION AS OF 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CLONE refuses a duplicate clause",
            sql: "CREATE TABLE c CLONE d LOCATION '/a' LOCATION '/b'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CREATE SCHEMA and CREATE FUNCTION are built",
            sql: "CREATE SCHEMA IF NOT EXISTS s COMMENT 'x'; CREATE FUNCTION f(a INT) RETURNS INT RETURN a + 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a Python function keeps its body",
            sql: "CREATE FUNCTION f(a INT) RETURNS INT LANGUAGE PYTHON AS $$return a$$",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "INSERT OVERWRITE DIRECTORY needs a path once",
            sql: "INSERT OVERWRITE DIRECTORY USING PARQUET VALUES (1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a primary key takes one TIMESERIES column",
            sql: "CREATE TABLE t (a INT, b INT, PRIMARY KEY (a TIMESERIES, b TIMESERIES))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "MATCH_RECOGNIZE takes every documented clause",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PARTITION BY s ORDER BY ts MEASURES LAST(down.ts) AS b ONE ROW PER MATCH AFTER MATCH SKIP PAST LAST ROW PATTERN ( strt down+ ) DEFINE down AS p < PREV(p) ) AS r",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MATCH_RECOGNIZE patterns take anchors, groups, alternation, PERMUTE and quantifiers",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PATTERN ( ^ a (b* | c{2,}) PERMUTE(d, e?) f{,3} g{4} $ ) DEFINE a AS TRUE )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MATCH_RECOGNIZE pattern variables need no DEFINE entry, which is semantic",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PATTERN ( m ) DEFINE a AS TRUE )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a MATCH_RECOGNIZE measure must be aliased with AS",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( MEASURES 1 m PATTERN ( a ) DEFINE a AS TRUE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "MATCH_RECOGNIZE needs PATTERN",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( DEFINE a AS TRUE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "MATCH_RECOGNIZE refuses an empty pattern",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PATTERN ( ) DEFINE a AS TRUE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a DEFINE entry needs AS",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PATTERN ( a ) DEFINE a TRUE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an empty quantifier is refused",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PATTERN ( a{} ) DEFINE a AS TRUE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "WATERMARK after TABLESAMPLE, before an alias, is part of the table's tree",
            sql: "SELECT * FROM t TABLESAMPLE ( 50 PERCENT ) WATERMARK c DELAY OF INTERVAL 1 MINUTE a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "WATERMARK on a LATERAL subquery is part of the subquery's tree",
            sql: "SELECT * FROM LATERAL ( SELECT 1 ) WATERMARK c DELAY OF INTERVAL 1 MINUTE a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "WATERMARK needs an interval after DELAY OF",
            sql: "SELECT * FROM t WATERMARK c DELAY OF 5",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ANTI may alias an UNPIVOT, whose alias is an identifier",
            sql: "SELECT * FROM t UNPIVOT ( v FOR k IN ( a , b ) ) ANTI",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ANTI may alias a PIVOT",
            sql: "SELECT * FROM t PIVOT ( sum(v) FOR k IN ( 1 , 2 ) ) ANTI",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ANTI may alias a select item and a pipe AS",
            sql: "SELECT 1 ANTI; FROM t |> AS ANTI",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ANTI cannot alias a VALUES table",
            sql: "VALUES 1 ANTI",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ANTI JOIN after an UNPIVOT is still a join",
            sql: "SELECT * FROM t UNPIVOT ( v FOR k IN ( a , b ) ) ANTI JOIN u ON 1 = 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an account statement is checked against its documented syntax and kept as a tree",
            sql: "UNSET TAG ON COLUMN t1 . c1 k",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "UNSET TAG needs the tag key after the securable",
            sql: "UNSET TAG ON v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET TAG ON a schema takes an optional value",
            sql: "SET TAG ON SCHEMA s1 k = 'v'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a schema's tag names are strings, not identifiers",
            sql: "ALTER SCHEMA s1 SET TAGS ( m1 = 'v' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER SCHEMA takes RETAIN DROPPED TO",
            sql: "ALTER SCHEMA s1 SET RETAIN DROPPED TO 7 DAYS",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a key-only TBLPROPERTIES on CREATE TABLE is refused",
            sql: "CREATE TABLE t ( c INT ) TBLPROPERTIES ( 'k' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a key-only OPTIONS on CREATE TABLE is refused",
            sql: "CREATE TABLE t ( c INT ) USING DELTA OPTIONS ( 'k' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "adjacent strings in OPTIONS are a key and its value",
            sql: "CREATE TABLE t ( c INT ) USING DELTA OPTIONS ( 'k' 'v' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a column mask and a row filter on CREATE TABLE are kept",
            sql: "CREATE TABLE t ( c INT MASK f USING COLUMNS ( 1 ) ) WITH ROW FILTER g ON ( c )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ENABLE NOVALIDATE follows a primary key in CREATE TABLE",
            sql: "CREATE TABLE t ( c INT NOT NULL , PRIMARY KEY ( c ) ENABLE NOVALIDATE )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ENABLE NOVALIDATE may not follow a UNIQUE constraint",
            sql: "CREATE TABLE t ( c INT , UNIQUE ( c ) ENABLE NOVALIDATE )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CLUSTER BY NONE is refused on CREATE TABLE",
            sql: "CREATE TABLE t ( c INT ) CLUSTER BY NONE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a SQL function's body after its parameters is a $$ string, not a quoted one",
            sql: "CREATE FUNCTION f ( ) AS 'body'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "streaming table AUTO CDC flows are checked",
            sql: "CREATE OR REFRESH STREAMING TABLE st FLOW AUTO CDC FROM s KEYS ( k ) SEQUENCE BY v STORED AS SCD TYPE 2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SYSTEM SEQUENCE BY needs STORED AS BITEMPORAL",
            sql: "CREATE OR REFRESH STREAMING TABLE st FLOW AUTO CDC FROM s KEYS ( k ) SEQUENCE BY v SYSTEM SEQUENCE BY w",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "snapshot AUTO CDC keys are names, not strings",
            sql: "CREATE OR REFRESH STREAMING TABLE st FLOW AUTO CDC FROM SNAPSHOT ( SELECT 1 ) KEYS ( 'k' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a materialized view's AS is optional",
            sql: "CREATE MATERIALIZED VIEW mv SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER STREAMING TABLE needs an action",
            sql: "ALTER STREAMING TABLE st",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DROP INDEX is refused as an invalid statement",
            sql: "DROP INDEX ON t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unquoted REFRESH resource path may not hold a blank",
            sql: "REFRESH VIEW t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unquoted REFRESH resource path without blanks is its operand",
            sql: "REFRESH /data/x",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REFRESH MATERIALIZED VIEW takes no WHERE",
            sql: "REFRESH MATERIALIZED VIEW mv WHERE c = 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a macro call's arguments are expressions, not free text",
            sql: "SELECT @py.m(`a)b` x)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a macro name follows its @ without a blank",
            sql: "SELECT @ py.f()",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a DATE literal must hold a date",
            sql: "SELECT DATE 'm'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a DATE literal may hold a year and month only",
            sql: "SELECT DATE '2024-02'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a binary literal holds hex digits",
            sql: "SELECT X'num'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an interval string holds a number",
            sql: "SELECT INTERVAL 'm'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "IDENTIFIER takes an argument",
            sql: "SELECT IDENTIFIER()",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET with text no pattern reads is refused",
            sql: "SET = 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET RECIPIENT name is a documented form",
            sql: "SET RECIPIENT r",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SET PATH needs a schema of two parts",
            sql: "SET PATH = s",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET TIME ZONE needs a zone",
            sql: "SET TIME ZONE v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ADD names a file, jar or archive",
            sql: "ADD v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ANALYZE is checked against its syntax",
            sql: "ANALYZE TABLE t1 COMPUTE STATISTICS FOR v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SYNC TABLE names the same table on both sides",
            sql: "SYNC TABLE t1 FROM v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unknown SHOW is refused",
            sql: "SHOW t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DROP TEMPORARY VARIABLE is a tree of its clauses",
            sql: "DROP TEMPORARY VARIABLE IF EXISTS v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a compound block with a cursor is read as a tree",
            sql: "BEGIN DECLARE c1 CURSOR FOR SELECT 1 ; OPEN c1 ; CLOSE c1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a labelled loop may LEAVE its label",
            sql: "BEGIN l1 : LOOP LEAVE l1 ; END LOOP l1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a compound body holds statements, not names",
            sql: "BEGIN WHILE v1 DO v1 ; END WHILE ; END",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a whole compound statement takes no label",
            sql: "l1 : BEGIN END",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "BEGIN alone is refused",
            sql: "BEGIN",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SIGNAL SQLSTATE takes a string",
            sql: "BEGIN SIGNAL SQLSTATE v1 ; END",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "RESET takes a key Spark's pattern reads",
            sql: "RESET a b",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SHOW GRANTS needs ON before the securable",
            sql: "SHOW GRANTS TABLE t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "IN needs at least one value",
            sql: "SELECT ( v1 IN ( ) )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "LIKE ANY needs at least one pattern",
            sql: "SELECT ( v1 LIKE ANY ( ) )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
    ];
    for test_case in test_cases {
        let verdict: ExpectedVerdict = parse(test_case.sql)
            .map(|parsed| ExpectedVerdict::Accepted {
                opaque: !parsed.gaps.is_empty(),
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
fn given_documented_statements_when_parsing_then_the_verdict_is_the_warehouse_receipt() {
    let test_cases = [
        VerdictTestCase {
            description: "a catalog name may be a string literal",
            sql: "ALTER CATALOG 'v1' DEFAULT COLLATION UTF8_BINARY",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DROP CONNECTION takes no RESTRICT or FORCE",
            sql: "ALTER CATALOG v1 DROP CONNECTION",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a catalog name may be a string literal when created",
            sql: "CREATE CATALOG 'v1'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a catalog name may be a string literal when dropped",
            sql: "DROP CATALOG 'v1'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a catalog name may be a string literal in COMMENT ON",
            sql: "COMMENT ON CATALOG 'v1' IS 'x'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an owner change may leave out the connection name",
            sql: "ALTER CONNECTION OWNER TO v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a governed tag's value list may be empty",
            sql: "CREATE GOVERNED TAG v1 VALUES ( )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a governed tag's values may be set to an empty list",
            sql: "ALTER GOVERNED TAG v1 SET VALUES ( )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "recipient properties may be keys without values",
            sql: "ALTER RECIPIENT v1 SET PROPERTIES ( 'v' )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a shared object removed without its kind is a name, even a kind's word",
            sql: "ALTER SHARE v1 REMOVE MODEL",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a policy's securable may be a bare name",
            sql: "DROP POLICY v1 ON t1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a policy's securable kind alone is a name",
            sql: "DROP POLICY v1 ON CATALOG",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a tag's securable may be a bare relation and column",
            sql: "SET TAG ON t1 . c1 v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an unset tag's securable may be a bare name",
            sql: "UNSET TAG ON v1 v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a column's bare relation name may be the word COLUMN",
            sql: "SET TAG ON COLUMN . c1 v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "EXTERNAL METADATA is a securable",
            sql: "GRANT SELECT ON EXTERNAL METADATA v1 TO `u1`",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ANY FILE is a securable with no name",
            sql: "GRANT SELECT ON ANY FILE TO `u1`",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a share is granted to a recipient",
            sql: "GRANT SELECT ON SHARE v1 TO RECIPIENT v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a group is created with users and subgroups",
            sql: "CREATE GROUP v1 WITH USER v1 , v2 GROUP v3",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a group takes one ADD or DROP, not both",
            sql: "ALTER GROUP v1 ADD USER v1 DROP GROUP v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a group is dropped by name",
            sql: "DROP GROUP v1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REMOVE takes a volume path",
            sql: "REMOVE '/Volumes/workspace/s1/v1/f1.csv'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "VACUUM keeps its options in its tree",
            sql: "VACUUM t1 RETAIN 168 HOURS DRY RUN",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "OPTIMIZE keeps its predicate and Z-order columns",
            sql: "OPTIMIZE t1 WHERE c1 = 1 ZORDER BY ( c1 , c2 )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MSCK REPAIR TABLE keeps its partition action",
            sql: "MSCK REPAIR TABLE t1 ADD PARTITIONS",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REPAIR TABLE keeps SYNC METADATA",
            sql: "REPAIR TABLE t1 SYNC METADATA",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a transaction is begun, committed and rolled back",
            sql: "BEGIN TRANSACTION; COMMIT WORK; ROLLBACK",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SET COLLATION names its collation",
            sql: "SET COLLATION UTF8_BINARY",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DEFAULT COLLATION needs the catalog name",
            sql: "ALTER CATALOG DEFAULT COLLATION UTF8_BINARY",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET MANAGED LOCATION needs the catalog name",
            sql: "ALTER CATALOG SET MANAGED LOCATION 'm'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CREATE CATALOG takes no OPTIONS",
            sql: "CREATE CATALOG v1 OPTIONS ( v1 = 'v' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a foreign catalog needs CATALOG",
            sql: "CREATE FOREIGN v1 USING CONNECTION v1 OPTIONS ( v1 = 'v' )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a shared partition column needs a value or a pattern",
            sql: "ALTER SHARE sh1 ADD TABLE t1 PARTITION ( v1 )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ADD needs USER or GROUP and a member",
            sql: "ALTER GROUP v1 ADD",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a group change needs ADD or DROP",
            sql: "ALTER GROUP v1 GROUP v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "WITH needs USER or GROUP",
            sql: "CREATE GROUP v1 WITH",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DROP GROUP needs the group",
            sql: "DROP GROUP",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a share is granted only to a RECIPIENT",
            sql: "GRANT SELECT ON SHARE v1 TO v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TO RECIPIENT needs a share",
            sql: "GRANT SELECT ON v1 TO RECIPIENT v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "FROM RECIPIENT needs the recipient",
            sql: "REVOKE SELECT ON SHARE v1 FROM RECIPIENT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "REMOVE takes a path string, not a name",
            sql: "REMOVE m1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "REMOVE needs a path",
            sql: "REMOVE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a securable kind's word before a name opens that kind, so the tag key is missing",
            sql: "SET TAG ON TABLE v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "the same holds when a tag is unset",
            sql: "UNSET TAG ON VOLUME v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
    ];
    for test_case in test_cases {
        let verdict: ExpectedVerdict = parse(test_case.sql)
            .map(|parsed| ExpectedVerdict::Accepted {
                opaque: !parsed.gaps.is_empty(),
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
fn given_a_table_argument_and_a_call_both_failing_when_parsing_then_the_further_error_is_kept() {
    let test_cases = [
        RefusalTestCase {
            description: "a TABLE argument's PARTITION BY with no expression fails at its )",
            sql: "SELECT * FROM f1 ( TABLE ( v1 ) PARTITION BY )",
            expected_position: 45,
        },
        RefusalTestCase {
            description: "a TABLE call missing its ) fails at the end of input",
            sql: "SELECT * FROM AI_PREDICT_VALUE ( input => TABLE ( v1 , target_col => v1 , feature_cols => v1 )",
            expected_position: 94,
        },
    ];
    for test_case in test_cases {
        let error = parse(test_case.sql).expect_err(test_case.description);
        assert_eq!(
            error.position, test_case.expected_position,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_parsing_strictly_then_any_error_refuses_it() {
    let test_cases = [
        StrictTestCase {
            description: "two statements split on semicolons",
            sql: "SELECT 1; SELECT 2;",
            expected_outcome: Ok((2, 0)),
        },
        StrictTestCase {
            description: "a gap is kept, not refused",
            sql: "REPAIR PRIVILEGES t1",
            expected_outcome: Ok((1, 1)),
        },
        StrictTestCase {
            description: "a syntax error refuses the whole input",
            sql: "SELECT 1; SELECT (",
            expected_outcome: Err(Some(true)),
        },
        StrictTestCase {
            description: "a parser check refuses it too",
            sql: "SELECT CAST(a AS NOSUCHTYPE)",
            expected_outcome: Err(Some(false)),
        },
    ];
    for test_case in test_cases {
        let outcome: Result<(usize, usize), Option<bool>> = parse_strict(test_case.sql)
            .map(|parsed| (parsed.statements.len(), parsed.gaps.len()))
            .map_err(|failure| failure.parse_error().map(|error| error.syntax));
        assert_eq!(
            outcome, test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_gap_when_parsing_then_it_names_the_construct_and_its_range() {
    let test_cases = [
        GapTestCase {
            description: "a statement with no shared node is a gap from its first word to its end",
            sql: "SELECT 1; REPAIR PRIVILEGES t1",
            expected_gaps: vec![Gap {
                construct: "repair",
                start: 10,
                end: 30,
            }],
        },
        GapTestCase {
            description: "a directory's SERDE row format, which the tree cannot hold, is a gap",
            sql: "INSERT OVERWRITE DIRECTORY '/tmp/x' ROW FORMAT SERDE 'a.S' SELECT 1",
            expected_gaps: vec![Gap {
                construct: "directory_row_format",
                start: 36,
                end: 58,
            }],
        },
    ];
    for test_case in test_cases {
        let parsed: Parsed = parse(test_case.sql)
            .unwrap_or_else(|error| panic!("{}: {error:?}", test_case.description));
        assert_eq!(
            parsed.gaps, test_case.expected_gaps,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_strict_failures_when_matched_then_only_parse_errors_carry_one() {
    let test_cases = [
        StrictFailureTestCase {
            description: "an uncovered byte carries no parse error",
            failure: StrictFailure::Uncovered { position: 3 },
            expected_syntax: None,
        },
        StrictFailureTestCase {
            description: "a refused input carries its syntax error",
            failure: parse_strict("SELECT (").expect_err("SELECT ( is refused"),
            expected_syntax: Some(true),
        },
    ];
    for test_case in test_cases {
        let syntax: Option<bool> = test_case.failure.parse_error().map(|error| error.syntax);
        assert_eq!(
            syntax, test_case.expected_syntax,
            "{}",
            test_case.description
        );
    }
}
