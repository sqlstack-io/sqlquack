//! Verdicts and trees of the Databricks SQL parser, following Spark 4.2.0's SqlBaseParser.g4.

use serde_json::Value;
use sqlquack_syntax::{Gap, Parsed};

use crate::parse::errors::StrictFailure;
use crate::parse::main::parse::parse;
use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::{
    ChainTestCase, ExpectedVerdict, GapTestCase, NestingTestCase, RefusalTestCase,
    StrictFailureTestCase, StrictTestCase, ValueTestCase, VerdictTestCase,
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
            sql: "CACHE SELECT a FROM t",
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
            description: "NOCOPY * parses, kept as a gap",
            sql: "CREATE OR REPLACE VIEW v NOCOPY * AS SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
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
            description: "ALTER TABLE DROP FEATURE is a gap",
            sql: "ALTER TABLE t DROP FEATURE deletionVectors TRUNCATE HISTORY",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
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
            description: "WATERMARK after TABLESAMPLE, before an alias, is a gap",
            sql: "SELECT * FROM t TABLESAMPLE ( 50 PERCENT ) WATERMARK c DELAY OF INTERVAL 1 MINUTE a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "WATERMARK on a LATERAL subquery is a gap",
            sql: "SELECT * FROM LATERAL ( SELECT 1 ) WATERMARK c DELAY OF INTERVAL 1 MINUTE a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
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
            description: "a column mask and a row filter on CREATE TABLE are gaps",
            sql: "CREATE TABLE t ( c INT MASK f USING COLUMNS ( 1 ) ) WITH ROW FILTER g ON ( c )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "ENABLE NOVALIDATE follows a primary key in CREATE TABLE",
            sql: "CREATE TABLE t ( c INT NOT NULL , PRIMARY KEY ( c ) ENABLE NOVALIDATE )",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
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
            description: "an unquoted REFRESH resource path without blanks is a gap",
            sql: "REFRESH /data/x",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
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
            description: "a compound block is checked and kept as a gap",
            sql: "BEGIN DECLARE c1 CURSOR FOR SELECT 1 ; OPEN c1 ; CLOSE c1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a labelled loop may LEAVE its label",
            sql: "BEGIN l1 : LOOP LEAVE l1 ; END LOOP l1 ; END",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
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
fn given_sql_when_parsing_then_the_tree_keeps_what_lineage_reads() {
    let test_cases = [
        ValueTestCase {
            description: "a column keeps its qualifier",
            sql: "SELECT t.a FROM s.t AS t",
            pointer: "/0/select/expressions/0/column/table/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "a call without a quantifier records none",
            sql: "SELECT sum(a) FROM t",
            pointer: "/0/select/expressions/0/function/set_quantifier",
            expected_json: "\"none\"",
        },
        ValueTestCase {
            description: "TABLE with named arguments after its name is a call of a function named TABLE",
            sql: "SELECT * FROM AI_PREDICT_CLASS ( input => TABLE ( v1 , target_col => v1 ) )",
            pointer: "/0/select/from/expressions/0/function/args/0/named_argument/value/function/name",
            expected_json: "\"TABLE\"",
        },
        ValueTestCase {
            description: "TABLE around one name stays a table argument",
            sql: "SELECT * FROM AI_PREDICT_CLASS ( input => TABLE ( v1 ) , target_col => v1 )",
            pointer: "/0/select/from/expressions/0/function/args/0/named_argument/value/table_argument/prefix",
            expected_json: "\"TABLE\"",
        },
        ValueTestCase {
            description: "a pipe operator is kept as written",
            sql: "FROM t1 |> AGGREGATE sum ( c1 )",
            pointer: "/0/select/pipe_operators",
            expected_json: "[\"|> AGGREGATE\"]",
        },
        ValueTestCase {
            description: "a piped set operation is kept as written",
            sql: "FROM t1 |> UNION SELECT 1",
            pointer: "/0/select/pipe_operators",
            expected_json: "[\"|> UNION\"]",
        },
        ValueTestCase {
            description: "a pipe operator after an unpiped set operation is kept as written",
            sql: "SELECT 1 UNION SELECT 2 |> LIMIT 1",
            pointer: "/0/select/pipe_operators",
            expected_json: "[\"|> LIMIT\"]",
        },
        ValueTestCase {
            description: "a bare | before a pipe operator is kept apart from |>",
            sql: "FROM t1 | WHERE 1 = 1",
            pointer: "/0/select/pipe_operators",
            expected_json: "[\"| WHERE\"]",
        },
        ValueTestCase {
            description: "a trailing |> with no operation is kept",
            sql: "SELECT c1 FROM t1 |>",
            pointer: "/0/select/pipe_operators",
            expected_json: "[\"|>\"]",
        },
        ValueTestCase {
            description: "pipe operators folded into one select are kept in order",
            sql: "FROM t1 |> WHERE c1 = 1 |> DROP c2",
            pointer: "/0/select/pipe_operators",
            expected_json: "[\"|> WHERE\",\"|> DROP\"]",
        },
        ValueTestCase {
            description: "SUBSTR written with FROM keeps the FROM spelling",
            sql: "SELECT SUBSTR ( 1 FROM v1 FOR v1 )",
            pointer: "/0/select/expressions/0/function/from_separator",
            expected_json: "true",
        },
        ValueTestCase {
            description: "SUBSTRING written with FROM keeps the FROM spelling",
            sql: "SELECT SUBSTRING ( 1 FROM v1 )",
            pointer: "/0/select/expressions/0/function/from_separator",
            expected_json: "true",
        },
        ValueTestCase {
            description: "DROP DATABASE keeps DATABASE",
            sql: "DROP DATABASE s1",
            pointer: "/0/drop_schema/keyword",
            expected_json: "\"DATABASE\"",
        },
        ValueTestCase {
            description: "DROP NAMESPACE keeps NAMESPACE",
            sql: "DROP NAMESPACE IF EXISTS s1 CASCADE",
            pointer: "/0/drop_schema/keyword",
            expected_json: "\"NAMESPACE\"",
        },
        ValueTestCase {
            description: "CREATE DATABASE keeps DATABASE",
            sql: "CREATE DATABASE s1",
            pointer: "/0/create_schema/keyword",
            expected_json: "\"DATABASE\"",
        },
        ValueTestCase {
            description: "ALTER DATABASE keeps DATABASE",
            sql: "ALTER DATABASE s1 SET DBPROPERTIES ( 'k' = 'v' )",
            pointer: "/0/alter_object/kind",
            expected_json: "\"DATABASE\"",
        },
        ValueTestCase {
            description: "a BD literal with an exponent keeps the precision its digits write",
            sql: "SELECT 12E-1BD",
            pointer: "/0/select/expressions/0/cast/to/precision",
            expected_json: "2",
        },
        ValueTestCase {
            description: "a BD literal with an exponent keeps the scale its digits write",
            sql: "SELECT 12E-1BD",
            pointer: "/0/select/expressions/0/cast/to/scale",
            expected_json: "1",
        },
        ValueTestCase {
            description: "a BD literal's precision is at least its scale",
            sql: "SELECT .5BD",
            pointer: "/0/select/expressions/0/cast/to/precision",
            expected_json: "1",
        },
        ValueTestCase {
            description: "a BD literal with leading zeros counts its scale as precision",
            sql: "SELECT 0.01BD",
            pointer: "/0/select/expressions/0/cast/to/precision",
            expected_json: "2",
        },
        ValueTestCase {
            description: "a BD literal with a positive exponent has scale 0 and the exponent's digits",
            sql: "SELECT 12E3BD",
            pointer: "/0/select/expressions/0/cast/to/precision",
            expected_json: "5",
        },
        ValueTestCase {
            description: "a BD literal never has a negative scale",
            sql: "SELECT 12E3BD",
            pointer: "/0/select/expressions/0/cast/to/scale",
            expected_json: "0",
        },
        ValueTestCase {
            description: "TIMESTAMP_LTZ keeps its spelling of TIMESTAMP",
            sql: "SELECT CAST ( NULL AS TIMESTAMP_LTZ ( 6 ) )",
            pointer: "/0/select/expressions/0/cast/to/spelling",
            expected_json: "\"TIMESTAMP_LTZ\"",
        },
        ValueTestCase {
            description: "SET CATALOG with a name chooses the catalog, as USE CATALOG does",
            sql: "SET CATALOG v1",
            pointer: "/0/use/kind",
            expected_json: "\"Catalog\"",
        },
        ValueTestCase {
            description: "SET CATALOG keeps SET",
            sql: "SET CATALOG v1",
            pointer: "/0/use/keyword",
            expected_json: "\"SET\"",
        },
        ValueTestCase {
            description: "SET CATALOG with a string keeps the string spelling",
            sql: "SET CATALOG ' catalog_name '",
            pointer: "/0/use/string_name",
            expected_json: "true",
        },
        ValueTestCase {
            description: "USE NAMESPACE keeps NAMESPACE",
            sql: "USE NAMESPACE s1",
            pointer: "/0/use/keyword",
            expected_json: "\"NAMESPACE\"",
        },
        ValueTestCase {
            description: "GROUP BY ... WITH ROLLUP keeps the WITH spelling",
            sql: "SELECT c1 FROM t1 GROUP BY c1 WITH ROLLUP",
            pointer: "/0/select/group_by/with_modifier",
            expected_json: "true",
        },
        ValueTestCase {
            description: "GROUP BY ... WITH CUBE keeps the WITH spelling",
            sql: "SELECT c1 FROM t1 GROUP BY c1 WITH CUBE",
            pointer: "/0/select/group_by/with_modifier",
            expected_json: "true",
        },
        ValueTestCase {
            description: "ON CATALOG before TO is a catalog left unnamed, not a securable named CATALOG",
            sql: "GRANT SELECT ON CATALOG TO `u1`",
            pointer: "/0/grant/kind",
            expected_json: "\"CATALOG\"",
        },
        ValueTestCase {
            description: "an unnamed catalog has no securable name",
            sql: "REVOKE SELECT ON CATALOG FROM `u1`",
            pointer: "/0/revoke/securable",
            expected_json: "null",
        },
        ValueTestCase {
            description: "METASTORE is a securable kind with no name",
            sql: "GRANT SELECT ON METASTORE TO `u1`",
            pointer: "/0/grant/kind",
            expected_json: "\"METASTORE\"",
        },
        ValueTestCase {
            description: "ANY FILE has no securable name, not an empty one",
            sql: "GRANT SELECT ON ANY FILE TO `u1`",
            pointer: "/0/grant/securable",
            expected_json: "null",
        },
        ValueTestCase {
            description: "a catalog named after ON CATALOG keeps its name",
            sql: "GRANT SELECT ON CATALOG c1 TO `u1`",
            pointer: "/0/grant/securable/name",
            expected_json: "\"c1\"",
        },
        ValueTestCase {
            description: "a bare LOCALTIME is the function, as the warehouse reads it",
            sql: "SELECT ( LOCALTIME )",
            pointer: "/0/select/expressions/0/paren/this/function/no_parens",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a CACHE TABLE option written without = keeps its position",
            sql: "CACHE TABLE t1 OPTIONS ( 'storageLevel' 'DISK_ONLY' )",
            pointer: "/0/cache/options_without_equals",
            expected_json: "[0]",
        },
        ValueTestCase {
            description: "CREATE's RETAIN DROPPED FOR keeps the count apart from the unit",
            sql: "CREATE DATABASE d1 RETAIN DROPPED FOR 1 HOUR",
            pointer: "/0/create_schema/properties/0/property/value/interval/this/literal/value",
            expected_json: "\"1\"",
        },
        ValueTestCase {
            description: "ALTER's RETAIN DROPPED TO keeps the unit as written",
            sql: "ALTER DATABASE d1 RETAIN DROPPED TO 2 DAYS",
            pointer: "/0/alter_object/operands/0/interval/unit/use_plural",
            expected_json: "true",
        },
        ValueTestCase {
            description: "an explicit ALL is recorded, not folded into none",
            sql: "SELECT sum(ALL a) FROM t",
            pointer: "/0/select/expressions/0/function/set_quantifier",
            expected_json: "\"all\"",
        },
        ValueTestCase {
            description: "ALL survives on an aggregate with FILTER",
            sql: "SELECT sum(ALL a) FILTER (WHERE a > 0) FROM t",
            pointer: "/0/select/expressions/0/aggregate_function/set_quantifier",
            expected_json: "\"all\"",
        },
        ValueTestCase {
            description: "DISTINCT is recorded on the aggregate",
            sql: "SELECT sum(DISTINCT a) FROM t",
            pointer: "/0/select/expressions/0/aggregate_function/set_quantifier",
            expected_json: "\"distinct\"",
        },
        ValueTestCase {
            description: "a table keeps its schema",
            sql: "SELECT t.a FROM s.t AS t",
            pointer: "/0/select/from/expressions/0/table/schema/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "a table keeps its alias",
            sql: "SELECT t.a FROM s.t AS t",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "LATERAL VIEW is a lateral view on the select, with its generator",
            sql: "SELECT x FROM t LATERAL VIEW OUTER explode(arr) e AS x",
            pointer: "/0/select/lateral_views/0/this/function/name",
            expected_json: "\"explode\"",
        },
        ValueTestCase {
            description: "LATERAL VIEW keeps its output column names",
            sql: "SELECT x FROM t LATERAL VIEW OUTER explode(arr) e AS x",
            pointer: "/0/select/lateral_views/0/column_aliases/0/name",
            expected_json: "\"x\"",
        },
        ValueTestCase {
            description: "LATERAL VIEW keeps its table alias and OUTER",
            sql: "SELECT x FROM t LATERAL VIEW OUTER explode(arr) e AS x",
            pointer: "/0/select/lateral_views/0/outer",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a qualified star keeps its table and EXCEPT list",
            sql: "SELECT t.* EXCEPT (b) FROM t",
            pointer: "/0/select/expressions/0/star/except/0/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "a qualified star keeps its table",
            sql: "SELECT t.* EXCEPT (b) FROM t",
            pointer: "/0/select/expressions/0/star/table/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "an inline table keeps its column aliases",
            sql: "SELECT * FROM VALUES (1, 2) AS v(a, b)",
            pointer: "/0/select/from/expressions/0/values/column_aliases/1/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "a derived table keeps its alias",
            sql: "SELECT q.a FROM (SELECT a FROM t) q(a)",
            pointer: "/0/select/from/expressions/0/subquery/alias/name",
            expected_json: "\"q\"",
        },
        ValueTestCase {
            description: "a derived table keeps its column aliases",
            sql: "SELECT q.a FROM (SELECT a FROM t) q(a)",
            pointer: "/0/select/from/expressions/0/subquery/column_aliases/0/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "a CTE is on the select's WITH",
            sql: "WITH c AS (SELECT 1 AS a) SELECT a FROM c",
            pointer: "/0/select/with/ctes/0/alias/name",
            expected_json: "\"c\"",
        },
        ValueTestCase {
            description: "LEFT ANTI JOIN is the LeftAnti join kind",
            sql: "SELECT * FROM a LEFT ANTI JOIN b ON a.x = b.x",
            pointer: "/0/select/joins/0/kind",
            expected_json: "\"LeftAnti\"",
        },
        ValueTestCase {
            description: "a lambda keeps its parameter",
            sql: "SELECT transform(arr, x -> x + 1)",
            pointer: "/0/select/expressions/0/function/args/1/lambda/parameters/0/name",
            expected_json: "\"x\"",
        },
        ValueTestCase {
            description: "MERGE keeps its target",
            sql: "MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN UPDATE SET * \
                  WHEN NOT MATCHED BY SOURCE THEN DELETE",
            pointer: "/0/merge/this/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "MERGE's NOT MATCHED BY SOURCE is a when clause on the source",
            sql: "MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN UPDATE SET * \
                  WHEN NOT MATCHED BY SOURCE THEN DELETE",
            pointer: "/0/merge/whens/whens/expressions/1/when/source/boolean/value",
            expected_json: "true",
        },
        ValueTestCase {
            description: "CREATE TABLE USING is a file format property",
            sql: "CREATE TABLE t (a INT) USING DELTA PARTITIONED BY (a)",
            pointer: "/0/create_table/properties/0/file_format_property/this/identifier/name",
            expected_json: "\"DELTA\"",
        },
        ValueTestCase {
            description: "CREATE TABLE PARTITIONED BY keeps its columns",
            sql: "CREATE TABLE t (a INT) USING DELTA PARTITIONED BY (a)",
            pointer: "/0/create_table/properties/1/partitioned_by_property/this/tuple/expressions/0/column/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "CREATE TABLE keeps its column names",
            sql: "CREATE TABLE t (a INT) USING DELTA PARTITIONED BY (a)",
            pointer: "/0/create_table/columns/0/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "COPY * keeps the replaced view's grants",
            sql: "CREATE OR REPLACE VIEW v COPY * AS SELECT 1",
            pointer: "/0/create_view/copy_grants",
            expected_json: "true",
        },
        ValueTestCase {
            description: "ALTER TABLE ADD COLUMNS adds each column",
            sql: "ALTER TABLE t ADD COLUMNS (a INT, b STRING)",
            pointer: "/0/alter_table/actions/1/AddColumn/column/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "ALTER TABLE RENAME COLUMN keeps both names",
            sql: "ALTER TABLE t RENAME COLUMN a TO b",
            pointer: "/0/alter_table/actions/0/RenameColumn/new_name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "ALTER COLUMN TYPE sets the type",
            sql: "ALTER TABLE t ALTER COLUMN a TYPE BIGINT",
            pointer: "/0/alter_table/actions/0/AlterColumn/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "ALTER TABLE ADD CONSTRAINT keeps the check",
            sql: "ALTER TABLE t ADD CONSTRAINT k CHECK (a > 0)",
            pointer: "/0/alter_table/actions/0/AddConstraint/Check/name/name",
            expected_json: "\"k\"",
        },
        ValueTestCase {
            description: "ALTER VIEW RENAME keeps the new name",
            sql: "ALTER VIEW v RENAME TO w",
            pointer: "/0/alter_view/actions/0/Rename/name/name",
            expected_json: "\"w\"",
        },
        ValueTestCase {
            description: "DESCRIBE keeps its table and EXTENDED",
            sql: "DESCRIBE TABLE EXTENDED s.t",
            pointer: "/0/describe/extended",
            expected_json: "true",
        },
        ValueTestCase {
            description: "DESCRIBE HISTORY is a describe of style HISTORY",
            sql: "DESCRIBE HISTORY s.t",
            pointer: "/0/describe/style",
            expected_json: "\"HISTORY\"",
        },
        ValueTestCase {
            description: "SHOW TABLES keeps its scope",
            sql: "SHOW TABLES IN s",
            pointer: "/0/show/scope/identifier/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "SET keeps the configuration key",
            sql: "SET a.b = 64",
            pointer: "/0/set_statement/items/0/name/identifier/name",
            expected_json: "\"a.b\"",
        },
        ValueTestCase {
            description: "USE SCHEMA keeps the schema",
            sql: "USE SCHEMA s",
            pointer: "/0/use/this/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "COMMENT ON TABLE keeps the table",
            sql: "COMMENT ON TABLE s.t IS 'x'",
            pointer: "/0/comment/this/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "TRUNCATE keeps the table",
            sql: "TRUNCATE TABLE s.t",
            pointer: "/0/truncate/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "COPY INTO keeps its target",
            sql: "COPY INTO s.t FROM '/x' FILEFORMAT = JSON",
            pointer: "/0/copy/this/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "COPY INTO keeps its file format",
            sql: "COPY INTO s.t FROM '/x' FILEFORMAT = JSON",
            pointer: "/0/copy/params/0/value/var/this",
            expected_json: "\"JSON\"",
        },
        ValueTestCase {
            description: "EXPLAIN wraps the query",
            sql: "EXPLAIN SELECT a FROM t",
            pointer: "/0/describe/target/select/from/expressions/0/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "GRANT keeps the privilege",
            sql: "GRANT SELECT ON TABLE t TO `a`",
            pointer: "/0/grant/privileges/0/name",
            expected_json: "\"SELECT\"",
        },
        ValueTestCase {
            description: "DECLARE keeps the variable",
            sql: "DECLARE OR REPLACE VARIABLE v DATE DEFAULT current_date()",
            pointer: "/0/declare/expressions/0/declare_item/this/identifier/name",
            expected_json: "\"v\"",
        },
        ValueTestCase {
            description: "CACHE TABLE keeps its query",
            sql: "CACHE TABLE c AS SELECT a FROM t",
            pointer: "/0/cache/query/select/from/expressions/0/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "CREATE TABLE LIKE keeps its source",
            sql: "CREATE TABLE a LIKE b",
            pointer: "/0/create_table/constraints/0/Like/source/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "CREATE TABLE CLONE keeps its source",
            sql: "CREATE TABLE a SHALLOW CLONE s.b",
            pointer: "/0/create_table/clone_source/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "CREATE SCHEMA keeps its name",
            sql: "CREATE SCHEMA c.s",
            pointer: "/0/create_schema/name/1/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "a SQL function keeps its RETURN query",
            sql: "CREATE FUNCTION f(p INT) RETURNS TABLE (x INT) RETURN SELECT x FROM t",
            pointer: "/0/create_function/body/Return/select/from/expressions/0/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "MATCH_RECOGNIZE keeps its input table",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PATTERN ( a+ ) DEFINE a AS TRUE ) AS r",
            pointer: "/0/select/from/expressions/0/match_recognize/this/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "MATCH_RECOGNIZE keeps its pattern as written",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PATTERN ( a+ ) DEFINE a AS TRUE ) AS r",
            pointer: "/0/select/from/expressions/0/match_recognize/pattern",
            expected_json: "\"a+\"",
        },
        ValueTestCase {
            description: "MATCH_RECOGNIZE keeps its alias",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( PATTERN ( a+ ) DEFINE a AS TRUE ) AS r",
            pointer: "/0/select/from/expressions/0/match_recognize/alias/name",
            expected_json: "\"r\"",
        },
        ValueTestCase {
            description: "MATCH_RECOGNIZE keeps its measure names",
            sql: "SELECT * FROM t MATCH_RECOGNIZE ( MEASURES count(*) AS n PATTERN ( a ) DEFINE a AS TRUE )",
            pointer: "/0/select/from/expressions/0/match_recognize/measures/0/this/alias/alias/name",
            expected_json: "\"n\"",
        },
        ValueTestCase {
            description: "a macro call is a method call on the column @name",
            sql: "SELECT id, @py.analysis_region() AS region FROM t",
            pointer: "/0/select/expressions/1/alias/this/method_call/this/column/name/name",
            expected_json: "\"@py\"",
        },
        ValueTestCase {
            description: "a macro call keeps its method name",
            sql: "SELECT @py.analysis_region() FROM t",
            pointer: "/0/select/expressions/0/method_call/method/name",
            expected_json: "\"analysis_region\"",
        },
        ValueTestCase {
            description: "a macro call's double-quoted arguments are strings",
            sql: "SELECT @py.line_total_cents(\"w.price_cents\", \"o.quantity\") FROM t",
            pointer: "/0/select/expressions/0/method_call/args/1/literal/value",
            expected_json: "\"o.quantity\"",
        },
        ValueTestCase {
            description: "a dotted macro name is a qualified column before the call",
            sql: "SELECT @py.a.b(1)",
            pointer: "/0/select/expressions/0/method_call/this/column/table/name",
            expected_json: "\"@py\"",
        },
        ValueTestCase {
            description: "a parenthesized DESCRIBE HISTORY is a subquery relation",
            sql: "SELECT max(timestamp) FROM (DESCRIBE HISTORY `main`.`analytics`.`orders`)",
            pointer: "/0/select/from/expressions/0/subquery/this/describe/style",
            expected_json: "\"HISTORY\"",
        },
        ValueTestCase {
            description: "a DESCRIBE subquery keeps its alias",
            sql: "SELECT * FROM (DESCRIBE HISTORY t) h",
            pointer: "/0/select/from/expressions/0/subquery/alias/name",
            expected_json: "\"h\"",
        },
        ValueTestCase {
            description: "a path wildcard stays in the path text",
            sql: "SELECT c1:item[*].price FROM VALUES ('x') AS T(c1)",
            pointer: "/0/select/expressions/0/j_s_o_n_extract/expression/literal/value",
            expected_json: "\"item[*].price\"",
        },
        ValueTestCase {
            description: "SHOW GRANTS ON keeps the securable kind",
            sql: "SHOW GRANTS ON TABLE x",
            pointer: "/0/show/scope_kind",
            expected_json: "\"ON TABLE\"",
        },
        ValueTestCase {
            description: "SHOW GRANTS keeps its principal",
            sql: "SHOW GRANTS `a@b.c` ON VIEW v",
            pointer: "/0/show/for_target/identifier/name",
            expected_json: "\"a@b.c\"",
        },
        ValueTestCase {
            description: "a streaming read keeps the relation it reads",
            sql: "SELECT * FROM STREAM t a",
            pointer: "/0/select/from/expressions/0/stream/this/table/alias/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "RESET of a key is a ResetStatement naming the key",
            sql: "RESET STATEMENT_TIMEOUT",
            pointer: "/0/reset_statement/name/identifier/name",
            expected_json: "\"STATEMENT_TIMEOUT\"",
        },
        ValueTestCase {
            description: "RESET keeps a dotted key whole",
            sql: "RESET spark.sql.ansi.enabled",
            pointer: "/0/reset_statement/name/identifier/name",
            expected_json: "\"spark.sql.ansi.enabled\"",
        },
        ValueTestCase {
            description: "a bare RESET names no key, scope or ALL",
            sql: "RESET",
            pointer: "/0/reset_statement",
            expected_json: "{}",
        },
        ValueTestCase {
            description: "EXPLAIN RESET holds the RESET",
            sql: "EXPLAIN RESET STATEMENT_TIMEOUT",
            pointer: "/0/describe/target/reset_statement/name/identifier/name",
            expected_json: "\"STATEMENT_TIMEOUT\"",
        },
        ValueTestCase {
            description: "TABLE (word) names a table, even when the word starts a query",
            sql: "SELECT * FROM EVENT_LOG ( TABLE ( SELECT ) )",
            pointer: "/0/select/from/expressions/0/function/args/0/table_argument/this/table/name/name",
            expected_json: "\"SELECT\"",
        },
        ValueTestCase {
            description: "SHOW TBLPROPERTIES keeps the key it shows",
            sql: "SHOW TBLPROPERTIES t ('delta.columnMapping.mode')",
            pointer: "/0/show/scope/literal/value",
            expected_json: "\"delta.columnMapping.mode\"",
        },
        ValueTestCase {
            description: "an UNPIVOT keeps ANTI as its alias",
            sql: "SELECT * FROM t UNPIVOT ( v FOR k IN ( a , b ) ) ANTI",
            pointer: "/0/select/from/expressions/0/unpivot/alias/name",
            expected_json: "\"ANTI\"",
        },
        ValueTestCase {
            description: "a documented statement's kind keeps the variant written",
            sql: "ALTER STORAGE CREDENTIAL v1 RENAME TO v2",
            pointer: "/0/utility_statement/kind",
            expected_json: "\"ALTER STORAGE CREDENTIAL\"",
        },
        ValueTestCase {
            description: "IF NOT EXISTS is a clause of its own and the name stands alone",
            sql: "CREATE CATALOG IF NOT EXISTS c",
            pointer: "/0/utility_statement/operands/1/table/name/name",
            expected_json: "\"c\"",
        },
        ValueTestCase {
            description: "an optional SET stays among its clause's words",
            sql: "ALTER CATALOG c SET OWNER TO `u 1`",
            pointer: "/0/utility_statement/operands/1/property/this/var/this",
            expected_json: "\"SET OWNER TO\"",
        },
        ValueTestCase {
            description: "a clause's identifier keeps its quoting",
            sql: "ALTER CATALOG c SET OWNER TO `u 1`",
            pointer: "/0/utility_statement/operands/1/property/value/identifier/quoted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "each repeated item keeps its own keyword clause",
            sql: "EXECUTE IMMEDIATE 'SELECT ?' USING 1 AS v1 , 2 AS v2",
            pointer: "/0/utility_statement/operands/1/property/value/tuple/expressions/1/tuple/expressions/1/property/value/identifier/name",
            expected_json: "\"v2\"",
        },
        ValueTestCase {
            description: "an optional flag ends its clause before the next optional part",
            sql: "REFRESH STREAMING TABLE t1 FULL WHERE c1 = 1",
            pointer: "/0/utility_statement/operands/1/property/this/var/this",
            expected_json: "\"FULL\"",
        },
        ValueTestCase {
            description: "an optional LIKE introduces the pattern after it",
            sql: "SHOW SHARES LIKE 'm'",
            pointer: "/0/utility_statement/operands/0/property/value/literal/value",
            expected_json: "\"m\"",
        },
        ValueTestCase {
            description: "a CALL keeps a named argument's name",
            sql: "CALL p ( a => 1 )",
            pointer: "/0/utility_statement/operands/1/tuple/expressions/0/named_argument/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "TRIM keeps LEADING as its position",
            sql: "SELECT TRIM(LEADING 'x' FROM s)",
            pointer: "/0/select/expressions/0/trim/position",
            expected_json: "\"Leading\"",
        },
        ValueTestCase {
            description: "TRIM keeps the characters apart from the string they trim",
            sql: "SELECT TRIM(LEADING 'x' FROM s)",
            pointer: "/0/select/expressions/0/trim/characters/literal/value",
            expected_json: "\"x\"",
        },
        ValueTestCase {
            description: "TRIM without a position records that none was written",
            sql: "SELECT TRIM('x' FROM s)",
            pointer: "/0/select/expressions/0/trim/position_explicit",
            expected_json: "false",
        },
        ValueTestCase {
            description: "EXTRACT keeps its field as written",
            sql: "SELECT EXTRACT(YRS FROM d)",
            pointer: "/0/select/expressions/0/extract/field/Custom",
            expected_json: "\"YRS\"",
        },
        ValueTestCase {
            description: "TABLESAMPLE REPEATABLE is not recorded as SEED",
            sql: "SELECT * FROM t TABLESAMPLE (50 PERCENT) REPEATABLE (1)",
            pointer: "/0/select/from/expressions/0/table/table_sample/use_seed_keyword",
            expected_json: "false",
        },
        ValueTestCase {
            description: "EXPLAIN EXTENDED sets the extended flag",
            sql: "EXPLAIN EXTENDED SELECT 1",
            pointer: "/0/describe/extended",
            expected_json: "true",
        },
        ValueTestCase {
            description: "EXPLAIN FORMATTED sets the formatted flag",
            sql: "EXPLAIN FORMATTED SELECT 1",
            pointer: "/0/describe/formatted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "SHOW STATISTICS keeps FOR ALL COLUMNS apart from AS JSON",
            sql: "SHOW STATISTICS t FOR ALL COLUMNS AS JSON",
            pointer: "/0/utility_statement/operands/2/property/this/var/this",
            expected_json: "\"AS JSON\"",
        },
        ValueTestCase {
            description: "LATERAL VIEW without a table identifier reads AS as the column aliases",
            sql: "SELECT x FROM t LATERAL VIEW explode(arr) AS x",
            pointer: "/0/select/lateral_views/0/table_alias",
            expected_json: "null",
        },
        ValueTestCase {
            description: "COLLATE keeps a backticked collation's quoting",
            sql: "SELECT s COLLATE `UTF8_BINARY`",
            pointer: "/0/select/expressions/0/collation/quoted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a named parameter marker keeps its backticks",
            sql: "SELECT :`p 1`",
            pointer: "/0/select/expressions/0/parameter/quoted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "DECFLOAT keeps its precision",
            sql: "SELECT CAST(1 AS DECFLOAT(34))",
            pointer: "/0/select/expressions/0/cast/to/precision",
            expected_json: "34",
        },
        ValueTestCase {
            description: "TIMESTAMP_NTZ keeps its precision",
            sql: "SELECT CAST(NULL AS TIMESTAMP_NTZ(7))",
            pointer: "/0/select/expressions/0/cast/to/precision",
            expected_json: "7",
        },
        ValueTestCase {
            description: "LIMIT ALL keeps ALL",
            sql: "SELECT a FROM t LIMIT ALL",
            pointer: "/0/select/limit/this/var/this",
            expected_json: "\"ALL\"",
        },
        ValueTestCase {
            description: "an inline constraint keeps its name",
            sql: "CREATE TABLE t (a INT CONSTRAINT k PRIMARY KEY NOT ENFORCED)",
            pointer: "/0/create_table/columns/0/constraint_names/0/1/name",
            expected_json: "\"k\"",
        },
        ValueTestCase {
            description: "an inline constraint keeps NOT ENFORCED",
            sql: "CREATE TABLE t (a INT REFERENCES u NOT ENFORCED)",
            pointer: "/0/create_table/columns/0/constraint_modifiers/0/1/enforced",
            expected_json: "false",
        },
        ValueTestCase {
            description: "a VALUES item that is the word DEFAULT is the column's default",
            sql: "INSERT INTO t VALUES (DEFAULT, `DEFAULT`)",
            pointer: "/0/insert/values/0/0",
            expected_json: "{\"default_keyword\":null}",
        },
        ValueTestCase {
            description: "a backticked DEFAULT in VALUES stays a column",
            sql: "INSERT INTO t VALUES (DEFAULT, `DEFAULT`)",
            pointer: "/0/insert/values/0/1/column/name/quoted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "UPDATE SET to DEFAULT is the column's default",
            sql: "UPDATE t SET a = DEFAULT",
            pointer: "/0/update/set/0/1",
            expected_json: "{\"default_keyword\":null}",
        },
        ValueTestCase {
            description: "a column alias list without AS",
            sql: "SELECT explode(m) (k, v)",
            pointer: "/0/select/expressions/0/alias/column_aliases/1/name",
            expected_json: "\"v\"",
        },
        ValueTestCase {
            description: "a column alias list without AS records that none was written",
            sql: "SELECT explode(m) (k, v)",
            pointer: "/0/select/expressions/0/alias/alias_explicit_as",
            expected_json: "false",
        },
        ValueTestCase {
            description: "Hive GROUPING SETS keeps the group expressions before it",
            sql: "SELECT a FROM t GROUP BY a, b GROUPING SETS (a, b)",
            pointer: "/0/select/group_by/expressions/1/column/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "Hive GROUPING SETS is not the call a comma before it writes",
            sql: "SELECT a FROM t GROUP BY a, b GROUPING SETS (a, b)",
            pointer: "/0/select/group_by/expressions/2/grouping_sets/expressions/1/column/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "CACHE TABLE records the AS before its query",
            sql: "CACHE TABLE t AS SELECT 1",
            pointer: "/0/cache/query_as",
            expected_json: "true",
        },
        ValueTestCase {
            description: "REPLACE TABLE without CREATE OR replaces only an existing table",
            sql: "REPLACE TABLE t CLONE u",
            pointer: "/0/create_table/replace_only",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a MATCH_RECOGNIZE pattern opens with its ^ anchor",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( ^ a b* | PERMUTE ( a , b ) {1,} $ ) DEFINE a AS TRUE , b AS TRUE )",
            pointer: "/0/select/from/expressions/0/match_recognize/row_pattern/Alternation/0/Sequence/0",
            expected_json: "\"Start\"",
        },
        ValueTestCase {
            description: "a pattern variable keeps its * quantifier",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( ^ a b* | PERMUTE ( a , b ) {1,} $ ) DEFINE a AS TRUE , b AS TRUE )",
            pointer: "/0/select/from/expressions/0/match_recognize/row_pattern/Alternation/0/Sequence/2/Quantified/quantifier",
            expected_json: "\"ZeroOrMore\"",
        },
        ValueTestCase {
            description: "PERMUTE keeps its members",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( ^ a b* | PERMUTE ( a , b ) {1,} $ ) DEFINE a AS TRUE , b AS TRUE )",
            pointer: "/0/select/from/expressions/0/match_recognize/row_pattern/Alternation/1/Sequence/0/Quantified/this/Permute/1/Symbol/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "{1,} is a quantifier with a minimum only",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( ^ a b* | PERMUTE ( a , b ) {1,} $ ) DEFINE a AS TRUE , b AS TRUE )",
            pointer: "/0/select/from/expressions/0/match_recognize/row_pattern/Alternation/1/Sequence/0/Quantified/quantifier/Between/min",
            expected_json: "1",
        },
        ValueTestCase {
            description: "a pattern closes with its $ anchor",
            sql: "SELECT * FROM t1 MATCH_RECOGNIZE ( ORDER BY c1 PATTERN ( ^ a b* | PERMUTE ( a , b ) {1,} $ ) DEFINE a AS TRUE , b AS TRUE )",
            pointer: "/0/select/from/expressions/0/match_recognize/row_pattern/Alternation/1/Sequence/1",
            expected_json: "\"End\"",
        },
        ValueTestCase {
            description: "DEFAULT COLLATION keeps a backticked name's quoting",
            sql: "CREATE TABLE t (a INT) DEFAULT COLLATION `UTF8_BINARY`",
            pointer: "/0/create_table/properties/0/property/value/identifier/quoted",
            expected_json: "true",
        },
    ];
    for test_case in test_cases {
        let parsed: Parsed = parse_strict(test_case.sql).expect(test_case.description);
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
            sql: "CACHE SELECT a FROM t",
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
    let test_cases = [GapTestCase {
        description: "a statement with no shared node is a gap from its first word to its end",
        sql: "SELECT 1; CACHE SELECT a FROM t",
        expected_gaps: vec![Gap {
            construct: "cache_select",
            start: 10,
            end: 31,
        }],
    }];
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

#[test]
fn given_deep_nesting_when_parsing_on_a_small_stack_then_it_is_refused_not_aborted() {
    let too_deep: Option<&str> = Some("the statement nests too deeply");
    let test_cases = [
        NestingTestCase {
            description: "1,500 parentheses around a table in FROM",
            sql: || format!("SELECT * FROM {}t{}", "(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "3,000 parenthesised joins in FROM",
            sql: || {
                let open: String = "(".repeat(3000);
                format!("SELECT * FROM {open}t{}", " JOIN u ON TRUE)".repeat(3000))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "3,000 STREAM reads of one table",
            sql: || format!("SELECT * FROM {}t", "STREAM ".repeat(3000)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 subqueries nested in FROM",
            sql: || {
                let open: String = "(SELECT * FROM ".repeat(1500);
                format!("SELECT * FROM {open}t{}", ")".repeat(1500))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 scalar subqueries",
            sql: || format!("SELECT {}1{}", "(SELECT ".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 EXISTS subqueries",
            sql: || {
                let open: String = "EXISTS (SELECT * FROM t WHERE ".repeat(1500);
                format!("SELECT * FROM t WHERE {open}TRUE{}", ")".repeat(1500))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 IN subqueries",
            sql: || {
                format!(
                    "SELECT 1 IN {}(1){}",
                    "(SELECT 1 IN ".repeat(1500),
                    ")".repeat(1500)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 parentheses around a query",
            sql: || format!("{}SELECT 1{}", "(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 parentheses around a set operation operand",
            sql: || {
                format!(
                    "{}SELECT 1{} UNION SELECT 2",
                    "(".repeat(1500),
                    ")".repeat(1500)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 parentheses around an expression",
            sql: || format!("SELECT {}1{}", "(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 nested calls",
            sql: || format!("SELECT {}1{}", "abs(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "6,000 nested subscripts",
            sql: || format!("SELECT {}0{}", "a[".repeat(6000), "]".repeat(6000)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 nested ARRAY types",
            sql: || {
                format!(
                    "SELECT CAST(NULL AS {}INT{})",
                    "ARRAY<".repeat(1500),
                    ">".repeat(1500)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 nested STRUCT types",
            sql: || {
                format!(
                    "SELECT CAST(NULL AS {}INT{})",
                    "STRUCT<a: ".repeat(1500),
                    ">".repeat(1500)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "past the limit, every other reading fails at once instead of retrying",
            sql: || {
                format!(
                    "SELECT 1 IN {}(1){}",
                    "(SELECT 1 IN ".repeat(150),
                    ")".repeat(150)
                )
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "40 parentheses around a table in FROM parse",
            sql: || format!("SELECT * FROM {}t{}", "(".repeat(40), ")".repeat(40)),
            expected_refusal: None,
        },
        NestingTestCase {
            description: "40 nested subqueries in FROM parse",
            sql: || {
                let open: String = "(SELECT * FROM ".repeat(40);
                format!("SELECT * FROM {open}t{}", ")".repeat(40))
            },
            expected_refusal: None,
        },
        NestingTestCase {
            description: "40 nested subscripts parse",
            sql: || format!("SELECT {}0{}", "a[".repeat(40), "]".repeat(40)),
            expected_refusal: None,
        },
    ];
    for test_case in test_cases {
        let sql: String = (test_case.sql)();
        let refusal: Option<String> = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || parse(&sql).err().map(|error| error.message))
            .expect("the parse thread starts")
            .join()
            .expect("the parse thread finishes");
        assert_eq!(
            refusal.as_deref(),
            test_case.expected_refusal,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_nested_chains_when_parsing_on_a_small_stack_then_trees_past_the_limit_are_refused() {
    let too_deep: Option<&str> = Some("the statement nests too deeply");
    let test_cases = [
        ChainTestCase {
            description: "10 parenthesised levels of 60 links",
            levels: 10,
            length: 60,
            expected_refusal: None,
        },
        ChainTestCase {
            description: "60 parenthesised levels of 60 links",
            levels: 60,
            length: 60,
            expected_refusal: None,
        },
        ChainTestCase {
            description: "130 parenthesised levels of 60 links, just inside the tree limit",
            levels: 130,
            length: 60,
            expected_refusal: None,
        },
        ChainTestCase {
            description: "60 parenthesised levels of 200 links",
            levels: 60,
            length: 200,
            expected_refusal: too_deep,
        },
        ChainTestCase {
            description: "150 parenthesised levels of 60 links",
            levels: 150,
            length: 60,
            expected_refusal: too_deep,
        },
    ];
    let links: [(&str, &str, &str); 7] = [
        ("SELECT ", "x", "::INT"),
        ("SELECT ", "x", " + 1"),
        ("SELECT ", "x", " || 'a'"),
        ("SELECT ", "x", " AND TRUE"),
        ("SELECT ", "x", " OR TRUE"),
        ("SELECT ", "x", "[1]"),
        ("", "SELECT 1", " UNION ALL SELECT 1"),
    ];
    for test_case in test_cases {
        for (prefix, base, link) in links {
            let level: String = format!("{})", link.repeat(test_case.length));
            let sql: String = format!(
                "{prefix}{}{base}{}",
                "(".repeat(test_case.levels),
                level.repeat(test_case.levels)
            );
            let refusal: Option<String> = std::thread::Builder::new()
                .stack_size(2 << 20)
                .spawn(move || parse(&sql).err().map(|error| error.message))
                .expect("the parse thread starts")
                .join()
                .expect("the parse thread finishes");
            assert_eq!(
                refusal.as_deref(),
                test_case.expected_refusal,
                "{} of `{link}`",
                test_case.description
            );
        }
    }
}
