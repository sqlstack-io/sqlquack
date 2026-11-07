//! Verdicts and trees of the BigQuery GoogleSQL parser, following googlesql.tm at 2026.11.1.

use serde_json::Value;
use sqlquack_syntax::{Gap, Parsed};

use crate::parse::errors::StrictFailure;
use crate::parse::main::parse::parse;
use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::helpers::verdict;
use crate::parse::tests::test_types::{
    ExpectedVerdict, GapTestCase, MatchRecognizeTestCase, StrictFailureTestCase, StrictTestCase,
    TreePairTestCase, ValueTestCase, VerdictTestCase,
};

#[test]
fn given_sql_when_parsing_then_the_verdict_is_googlesql() {
    let test_cases = [
        VerdictTestCase {
            description: "an empty input is refused",
            sql: "",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a reserved keyword is no bare table name",
            sql: "SELECT * FROM SELECT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a backquoted reserved keyword is a name",
            sql: "SELECT * FROM `SELECT`",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a non-reserved word such as DATE is a bare alias",
            sql: "SELECT * FROM t date",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "UNION needs ALL or DISTINCT",
            sql: "SELECT 1 UNION SELECT 2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "UNION ALL BY NAME is a set operation",
            sql: "SELECT a FROM t UNION ALL BY NAME SELECT a FROM u",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a CROSS JOIN in a chain with consecutive conditions takes no condition",
            sql: "SELECT * FROM t1 JOIN t2 CROSS JOIN t3 ON a JOIN t4 JOIN t5 ON b ON c",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a RIGHT JOIN after a comma join must be parenthesised",
            sql: "SELECT * FROM t1, t2 JOIN t3 ON a RIGHT JOIN t4 ON b",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "PIVOT on a join's right side comes before its condition",
            sql: "SELECT * FROM t1 JOIN t2 PIVOT (SUM(a) FOR b IN (1)) ON TRUE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a parenthesised join takes no PIVOT",
            sql: "SELECT * FROM (t1 CROSS JOIN t2) PIVOT (SUM(a) FOR b IN (1))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a chain's BY NAME ON lists must match, in order and without case",
            sql: "SELECT 1 a UNION ALL BY NAME ON (a) SELECT 1 a UNION ALL BY NAME SELECT 1 a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "BY NAME ON lists differing only in case match",
            sql: "SELECT 1 a UNION ALL BY NAME ON (a) SELECT 1 a UNION ALL BY NAME ON (A) SELECT 1 a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an outer mode without BY NAME is left to analysis",
            sql: "SELECT 1 OUTER UNION ALL SELECT 2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "INNER takes no OUTER",
            sql: "SELECT 1 INNER OUTER UNION ALL SELECT 2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "STRICT is refused with an outer mode",
            sql: "SELECT 1 FULL OUTER UNION ALL STRICT CORRESPONDING SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "consecutive ON clauses match the joins before them",
            sql: "SELECT * FROM t1 JOIN t2 JOIN t3 ON TRUE ON TRUE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "more ON clauses than qualified joins",
            sql: "SELECT * FROM t1 JOIN t2 NATURAL JOIN t3 ON TRUE ON TRUE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "consecutive ON clauses after a comma join",
            sql: "SELECT * FROM t1, t2 JOIN t3 JOIN t4 ON TRUE ON TRUE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a comma may follow a parenthesised join with consecutive ON clauses",
            sql: "SELECT * FROM t1 JOIN (t2 JOIN t3 JOIN t4 ON a ON b) ON c, t5",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a comma may come before a parenthesised join with consecutive USING",
            sql: "SELECT * FROM t5, t1 LEFT JOIN (t2 JOIN t3 JOIN t4 USING (a) USING (b)) USING (c)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MATCH_RECOGNIZE with every clause is a clean tree",
            sql: "SELECT * FROM t MATCH_RECOGNIZE (PARTITION BY a ORDER BY b DESC NULLS LAST MEASURES SUM(x.c) AS s AFTER MATCH SKIP TO NEXT ROW PATTERN (^ x+? (y | z){1,3} $) DEFINE x AS c > 1, y AS TRUE, z AS FALSE OPTIONS (use_longest_match = TRUE)) AS mr",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MATCH_RECOGNIZE follows a from item, not a join condition",
            sql: "SELECT * FROM t1 JOIN t2 ON TRUE MATCH_RECOGNIZE (ORDER BY b MEASURES COUNT(*) AS n PATTERN (x) DEFINE x AS TRUE)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a row pattern cannot start with a quantifier",
            sql: "SELECT * FROM t MATCH_RECOGNIZE (ORDER BY b MEASURES COUNT(*) AS n PATTERN (+a) DEFINE a AS TRUE)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a reserved word is not a type",
            sql: "SELECT CAST(1 AS SELECT)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "the select modifier comes before DISTINCT",
            sql: "SELECT DISTINCT WITH DIFFERENTIAL_PRIVACY 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a pipe aggregate's grouping alias, order and trailing comma",
            sql: "FROM t |> AGGREGATE COUNT(*) DESC GROUP AND ORDER BY 1 a1 NULLS FIRST,",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a pipe aggregate item takes no NULLS order without a direction",
            sql: "FROM t |> AGGREGATE COUNT(*) NULLS FIRST",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a statement of pipe operators alone",
            sql: "|> WHERE TRUE |> SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a table's WITH CONNECTION comes after CLUSTER BY",
            sql: "CREATE TABLE t (c INT64) WITH CONNECTION c1 CLUSTER BY c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "IN UNNEST, arrays, parameters and @@ variables are expressions",
            sql: "SELECT @@error.message, a IN UNNEST([1, 2]) FROM t WHERE b = @p",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an empty IN list is refused",
            sql: "SELECT 1 IN ()",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "aggregate calls take IGNORE NULLS, ORDER BY and LIMIT inside",
            sql: "SELECT ARRAY_AGG(a IGNORE NULLS ORDER BY b DESC LIMIT 3) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a window names a base window and adds a frame",
            sql: "SELECT SUM(a) OVER (w ROWS UNBOUNDED PRECEDING) FROM t WINDOW w AS (ORDER BY b)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "TABLESAMPLE needs SYSTEM",
            sql: "SELECT 1 FROM t TABLESAMPLE (10 PERCENT)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SELECT * EXCEPT and REPLACE",
            sql: "SELECT * EXCEPT (a) REPLACE (UPPER(b) AS b) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "pipe operators chain onto a query",
            sql: "FROM t |> WHERE a > 1 |> RENAME a AS b |> AGGREGATE COUNT(*) GROUP BY b",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a pipe with no operator is refused",
            sql: "FROM t |>",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a number with a name straight after it is refused",
            sql: "SELECT 123abc",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unknown type name parses; resolving it is analysis",
            sql: "SELECT CAST(a AS NOSUCHTYPE)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE with PARTITION BY, CLUSTER BY and OPTIONS",
            sql: "CREATE OR REPLACE TABLE d.t (a DATE, b INT64 NOT NULL) PARTITION BY a \
                  CLUSTER BY b OPTIONS (description = 'x') AS SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a primary key must be NOT ENFORCED",
            sql: "CREATE TABLE t (a INT64, PRIMARY KEY (a))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "MERGE with INSERT ROW and NOT MATCHED BY SOURCE",
            sql: "MERGE t USING s ON FALSE WHEN NOT MATCHED BY SOURCE THEN DELETE \
                  WHEN NOT MATCHED THEN INSERT ROW",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a script block is a gap",
            sql: "IF TRUE THEN SELECT CASE WHEN 1 THEN 2 END; END IF",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "ALTER TABLE FUNCTION is refused",
            sql: "ALTER TABLE FUNCTION f SET OPTIONS (a = 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER takes only the alterable schema object kinds",
            sql: "ALTER FUNCTION f SET OPTIONS (a = 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "only an index kind takes ON in ALTER",
            sql: "ALTER VIEW v ON t SET OPTIONS (a = 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER TABLE takes a list of actions",
            sql: "ALTER TABLE t ADD COLUMN a INT64, RENAME COLUMN b TO c",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DROP AGGREGATE FUNCTION is refused",
            sql: "DROP AGGREGATE FUNCTION f",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "only DROP SCHEMA takes a drop mode",
            sql: "DROP SCHEMA s CASCADE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DROP VIEW takes no drop mode",
            sql: "DROP VIEW v CASCADE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DROP TABLE FUNCTION takes no parameters",
            sql: "DROP TABLE FUNCTION f()",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DROP FUNCTION takes a signature",
            sql: "DROP FUNCTION f(INT64, x STRING)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "only DROP FUNCTION takes parameters",
            sql: "DROP MODEL m(INT64)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a drop with ON is checked and left a gap",
            sql: "DROP SEARCH INDEX i ON t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "UNDROP takes only a schema",
            sql: "UNDROP MODEL m",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "UNDROP SCHEMA is a statement",
            sql: "UNDROP SCHEMA s",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "TRUNCATE needs TABLE",
            sql: "TRUNCATE t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unknown entity type is refused",
            sql: "DROP FOO r",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an octal escape above 377 is refused",
            sql: "SELECT '\\400'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a hex escape takes no sign",
            sql: "SELECT '\\x+1'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "bytes take no unicode escape",
            sql: "SELECT b'\\u0041'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "string and bytes chunks do not concatenate",
            sql: "SELECT b'a' 'b'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "concatenated chunks need a separator",
            sql: "SELECT 'a''b'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a typed literal takes concatenated chunks",
            sql: "SELECT DATE '2014' \"-01-01\"",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a typed literal takes no bytes",
            sql: "SELECT DATE b'2014-01-01'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a number is a later path part",
            sql: "SELECT a.b.1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an interval's step is any expression",
            sql: "SELECT INTERVAL x + 1 DAY",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "STRING takes a collation string",
            sql: "SELECT CAST(NULL AS STRING COLLATE 'und:ci')",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DEFAULT is a DML value",
            sql: "INSERT t VALUES (DEFAULT, 2)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MERGE inserts ROW after a column list",
            sql: "MERGE t USING s ON t.k = s.k WHEN NOT MATCHED THEN INSERT (a) ROW",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "GROUP BY takes the empty grouping set",
            sql: "SELECT 1 FROM t GROUP BY ()",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "quantified LIKE takes UNNEST",
            sql: "SELECT x LIKE ANY UNNEST(a)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a function body is an expression, not a query",
            sql: "CREATE FUNCTION f() AS (SELECT 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "RETURNS takes no templated type",
            sql: "CREATE FUNCTION f() RETURNS ANY TYPE AS (1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a procedure's statements end with semicolons",
            sql: "CREATE PROCEDURE p() BEGIN SELECT 1 END",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a schema has no scope",
            sql: "CREATE TEMP SCHEMA s",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CREATE EXTERNAL SCHEMA requires OPTIONS",
            sql: "CREATE EXTERNAL SCHEMA s WITH CONNECTION `p.us.c`",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "only a table or schema has a snapshot",
            sql: "CREATE SNAPSHOT VIEW d.s CLONE d.t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CREATE TABLE FUNCTION is a statement",
            sql: "CREATE TABLE FUNCTION f(x INT64) RETURNS TABLE<a INT64> AS SELECT x AS a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE MODEL takes aliased training queries",
            sql: "CREATE MODEL d.m OPTIONS (model_type = 'x') AS (training_data AS (SELECT 1), custom_holiday AS (SELECT 2))",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE CLONE is a statement",
            sql: "CREATE TABLE d.t CLONE d.s FOR SYSTEM_TIME AS OF x",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a leading semicolon is refused",
            sql: "; SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a FROM query takes no SELECT after it",
            sql: "FROM t SELECT a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a pipe operator takes no ORDER BY after it",
            sql: "SELECT 1 |> SELECT 2 ORDER BY 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "pipe operators follow a query's ORDER BY",
            sql: "SELECT 1 ORDER BY 1 |> WHERE TRUE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a trailing comma may end a select list before ORDER BY",
            sql: "SELECT 1, ORDER BY 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a set operation chain mixing operations is refused",
            sql: "SELECT 1 UNION ALL SELECT 2 INTERSECT DISTINCT SELECT 3",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a CROSS JOIN condition counts in consecutive conditions",
            sql: "SELECT * FROM t1 JOIN t2 CROSS JOIN t3 ON a ON b",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TABLESAMPLE follows a subquery's alias",
            sql: "SELECT * FROM (SELECT 1) AS s TABLESAMPLE SYSTEM (10 PERCENT)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "TABLESAMPLE does not precede a table's alias",
            sql: "SELECT * FROM t TABLESAMPLE SYSTEM (1 PERCENT) AS x",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an UNPIVOT label is a string or an integer, never a name",
            sql: "SELECT * FROM t UNPIVOT(a FOR b IN (c AS x))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "digits after a call's dot are a field name",
            sql: "SELECT f(x).1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a dashed path is a table name only, never an expression",
            sql: "SELECT 1 FROM t WHERE my-proj-2.d.t > 0",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a dash joins words only in a path's first part",
            sql: "SELECT * FROM myproject.mydataset.my-table",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a backquoted part takes no dash after it",
            sql: "SELECT * FROM `a`-b",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "GRANT on a kinded object to string grantees is structured",
            sql: "GRANT `roles/bigquery.dataViewer` ON VIEW d.t TO 'user:a@example.com'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "REVOKE takes a parameter grantee",
            sql: "REVOKE x ON EXTERNAL TABLE d.t FROM @p",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a grantee is no bytes literal",
            sql: "GRANT x ON d.t TO b'a'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "GRANT has no WITH GRANT OPTION",
            sql: "GRANT x ON d.t TO 'a' WITH GRANT OPTION",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an object takes at most two kind words",
            sql: "GRANT x ON a b c d.t TO 'a'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "digits after a dot in a table path are a name part, letters following",
            sql: "SELECT * FROM foo.bar.25x a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a dashed path part may end in digits before the dot",
            sql: "SELECT * FROM my-proj-2.d.t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a table alias takes no column list",
            sql: "SELECT * FROM t AS t(c1, c2)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a named query takes no column list",
            sql: "WITH cte(c) AS (SELECT 1) SELECT * FROM cte",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a colon path is Snowflake's, not GoogleSQL's",
            sql: "SELECT payload:order_id FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a chained call on a parameter is a structured call",
            sql: "SELECT @sql.month_start(o.created_at) FROM orders AS o",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "HAVING MAX in an aggregate is structured",
            sql: "SELECT ANY_VALUE(fruit HAVING MAX sold) FROM fruits",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "HAVING in an aggregate needs MAX or MIN",
            sql: "SELECT ANY_VALUE(fruit HAVING sold) FROM fruits",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "FOR SYSTEM_TIME AS OF follows the table alias",
            sql: "SELECT * FROM t AS t0 FOR SYSTEM_TIME AS OF '2020-01-01'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "FOR SYSTEM_TIME AS OF does not precede the alias",
            sql: "SELECT * FROM t FOR SYSTEM_TIME AS OF '2020-01-01' AS t0",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SYSTEM_TIME AS OF needs FOR",
            sql: "SELECT * FROM t SYSTEM_TIME AS OF '2020-01-01'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "VERSION AS OF is no BigQuery time travel",
            sql: "SELECT * FROM t VERSION AS OF 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a parenthesised INSERT query continues with set operations",
            sql: "INSERT t (a) (SELECT 1) UNION ALL (SELECT 2) ORDER BY 1 LIMIT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a parenthesised INSERT query takes an alias only after AS",
            sql: "INSERT t (SELECT 1) x",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            verdict(test_case.sql),
            test_case.expected_verdict,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_parsing_then_the_tree_keeps_what_lineage_reads() {
    let test_cases = [
        ValueTestCase {
            description: "a backquoted project.dataset.table path splits into its parts",
            sql: "SELECT o.id FROM `p.d.orders` AS o",
            pointer: "/0/select/from/expressions/0/table/catalog/name",
            expected_json: "\"p\"",
        },
        ValueTestCase {
            description: "the path's last part is the table name",
            sql: "SELECT o.id FROM `p.d.orders` AS o",
            pointer: "/0/select/from/expressions/0/table/name/name",
            expected_json: "\"orders\"",
        },
        ValueTestCase {
            description: "a column keeps its qualifier",
            sql: "SELECT o.id FROM `p.d.orders` AS o",
            pointer: "/0/select/expressions/0/column/table/name",
            expected_json: "\"o\"",
        },
        ValueTestCase {
            description: "an UNNEST source keeps its alias",
            sql: "SELECT x FROM UNNEST([1, 2]) AS x WITH OFFSET AS off",
            pointer: "/0/select/from/expressions/0/function/args/0/alias/alias/name",
            expected_json: "\"x\"",
        },
        ValueTestCase {
            description: "SAFE_CAST is a try cast",
            sql: "SELECT SAFE_CAST(a AS INT64) FROM t",
            pointer: "/0/select/expressions/0/try_cast/this/column/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "BY NAME is kept on the set operation",
            sql: "SELECT a FROM t UNION ALL BY NAME SELECT a FROM u",
            pointer: "/0/union/by_name",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a CTE is on the select's WITH",
            sql: "WITH c AS (SELECT 1 AS a) SELECT a FROM c",
            pointer: "/0/select/with/ctes/0/alias/name",
            expected_json: "\"c\"",
        },
        ValueTestCase {
            description: "MERGE keeps its target",
            sql: "MERGE INTO t USING s ON t.id = s.id WHEN MATCHED THEN DELETE",
            pointer: "/0/merge/this/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "CREATE TABLE keeps its column names",
            sql: "CREATE TABLE t (a INT64) PARTITION BY a",
            pointer: "/0/create_table/columns/0/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "ALTER TABLE keeps each action as a clause",
            sql: "ALTER TABLE t RENAME COLUMN b TO c",
            pointer: "/0/alter_object/operands/0/tuple/expressions/0/var/this",
            expected_json: "\"RENAME COLUMN\"",
        },
        ValueTestCase {
            description: "DROP SCHEMA keeps CASCADE",
            sql: "DROP SCHEMA IF EXISTS p.s CASCADE",
            pointer: "/0/drop_object/cascade",
            expected_json: "true",
        },
        ValueTestCase {
            description: "DROP keeps the kind",
            sql: "DROP SNAPSHOT TABLE d.t",
            pointer: "/0/drop_object/kind",
            expected_json: "\"SNAPSHOT TABLE\"",
        },
        ValueTestCase {
            description: "TRUNCATE keeps its table",
            sql: "TRUNCATE TABLE d.t",
            pointer: "/0/truncate/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "a bytes literal is no string",
            sql: "SELECT b'abc'",
            pointer: "/0/select/expressions/0/literal/literal_type",
            expected_json: "\"byte_string\"",
        },
        ValueTestCase {
            description: "SELECT AS STRUCT keeps its kind",
            sql: "SELECT AS STRUCT 1 a, 2 b",
            pointer: "/0/select/kind",
            expected_json: "\"STRUCT\"",
        },
        ValueTestCase {
            description: "|| binds tighter than +",
            sql: "SELECT a + b || c",
            pointer: "/0/select/expressions/0/add/right/concat/left/column/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "IS UNKNOWN is no null test",
            sql: "SELECT x IS UNKNOWN",
            pointer: "/0/select/expressions/0/is/right/var/this",
            expected_json: "\"UNKNOWN\"",
        },
        ValueTestCase {
            description: "a typed literal is a cast typed by its syntax",
            sql: "SELECT BIGNUMERIC '1.5'",
            pointer: "/0/select/expressions/0/cast/inferred_type/name",
            expected_json: "\"BIGNUMERIC\"",
        },
        ValueTestCase {
            description: "BIGNUMERIC keeps its precision and scale",
            sql: "SELECT CAST(NULL AS BIGNUMERIC(10, 2))",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"BIGNUMERIC(10, 2)\"",
        },
        ValueTestCase {
            description: "RANGE keeps its element type",
            sql: "SELECT CAST(NULL AS RANGE<INT64>)",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"RANGE<INT64>\"",
        },
        ValueTestCase {
            description: "a typed array keeps its element type",
            sql: "SELECT ARRAY<FLOAT64>[1, 2]",
            pointer: "/0/select/expressions/0/cast/to/element_type/data_type",
            expected_json: "\"double\"",
        },
        ValueTestCase {
            description: "CAST FORMAT keeps its time zone",
            sql: "SELECT CAST(x AS STRING FORMAT 'TZH' AT TIME ZONE 'Asia/Kolkata')",
            pointer: "/0/select/expressions/0/cast/format/at_time_zone/zone/literal/value",
            expected_json: "\"Asia/Kolkata\"",
        },
        ValueTestCase {
            description: "a backquoted dotted SET target stays one name",
            sql: "UPDATE t SET `a.b` = 1 WHERE TRUE",
            pointer: "/0/update/set/0/0/quoted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "CREATE FUNCTION keeps its body as a clause",
            sql: "CREATE TEMP FUNCTION f(x INT64) RETURNS INT64 AS (x + 1)",
            pointer: "/0/create_object/modifier",
            expected_json: "\"TEMP\"",
        },
        ValueTestCase {
            description: "CREATE TABLE CLONE keeps its source",
            sql: "CREATE TABLE d.t CLONE d.s",
            pointer: "/0/create_table/clone_source/name/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "CREATE TABLE COPY is marked a copy",
            sql: "CREATE TABLE d.t COPY d.s",
            pointer: "/0/create_table/is_copy",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a materialized view replica keeps its source",
            sql: "CREATE MATERIALIZED VIEW v1 AS REPLICA OF d.t1",
            pointer: "/0/create_object/source",
            expected_json: "\"REPLICA\"",
        },
        ValueTestCase {
            description: "CREATE VIEW keeps its query's source",
            sql: "CREATE VIEW v AS SELECT a FROM d.t",
            pointer: "/0/create_view/query/select/from/expressions/0/table/schema/name",
            expected_json: "\"d\"",
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
fn given_sql_when_parsing_strictly_then_any_error_refuses_it() {
    let test_cases = [
        StrictTestCase {
            description: "two statements split on semicolons",
            sql: "SELECT 1; SELECT 2;",
            expected_outcome: Ok((2, 0)),
        },
        StrictTestCase {
            description: "a gap is kept, not refused",
            sql: "CALL p(1)",
            expected_outcome: Ok((1, 1)),
        },
        StrictTestCase {
            description: "a syntax error refuses the whole input",
            sql: "SELECT 1; SELECT (",
            expected_outcome: Err(Some(true)),
        },
        StrictTestCase {
            description: "an unclosed string is a syntax error",
            sql: "SELECT 'a",
            expected_outcome: Err(Some(true)),
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
        description: "a CALL after a statement is a call gap to the end of the input",
        sql: "SELECT 1; CALL p(1)",
        expected_gaps: vec![Gap {
            construct: "call",
            start: 10,
            end: 19,
        }],
    }];
    for test_case in test_cases {
        let parsed: Parsed = parse(test_case.sql).expect(test_case.description);
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
            description: "a refused parse carries its syntax error",
            failure: parse_strict("SELECT (").expect_err("refused"),
            expected_syntax: Some(true),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            test_case.failure.parse_error().map(|error| error.syntax),
            test_case.expected_syntax,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_bytes_literals_when_parsing_then_each_byte_is_one_unit() {
    let tree = |sql: &str| parse(sql).map(|parsed| parsed.statements);
    let test_cases = [
        TreePairTestCase {
            description: "a non-ASCII character in a bytes literal is its UTF-8 bytes",
            sql: "SELECT b'\u{ff}'",
            other_sql: "SELECT b'\\xc3\\xbf'",
            expected_same: true,
        },
        TreePairTestCase {
            description: "a hex escape and an octal escape of one byte are the same",
            sql: "SELECT b'\\xff'",
            other_sql: "SELECT b'\\377'",
            expected_same: true,
        },
        TreePairTestCase {
            description: "a non-ASCII character is not the one byte of its code point",
            sql: "SELECT b'\u{ff}'",
            other_sql: "SELECT b'\\xff'",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a raw bytes literal also reads a character as its UTF-8 bytes",
            sql: "SELECT rb'\u{ff}'",
            other_sql: "SELECT b'\\xc3\\xbf'",
            expected_same: true,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            tree(test_case.sql) == tree(test_case.other_sql),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_digits_after_a_dash_when_not_in_a_table_path_then_they_stay_a_number() {
    let tree = |sql: &str| parse(sql).map(|parsed| parsed.statements);
    let test_cases = [
        TreePairTestCase {
            description: "a-1.e5 is a minus and a number",
            sql: "SELECT a-1.e5",
            other_sql: "SELECT a - 1.e5",
            expected_same: true,
        },
        TreePairTestCase {
            description: "x-0.y is a minus, a number and an alias",
            sql: "SELECT x-0.y",
            other_sql: "SELECT x - 0. y",
            expected_same: true,
        },
        TreePairTestCase {
            description: "in a table path, my-proj-2 is not `my-proj` minus 2",
            sql: "SELECT * FROM my-proj-2.d.t",
            other_sql: "SELECT * FROM `my-proj`-2.d.t",
            expected_same: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            tree(test_case.sql) == tree(test_case.other_sql),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_pipe_operators_when_parsing_then_each_step_keeps_its_own_tree() {
    let tree = |sql: &str| parse(sql).map(|parsed| parsed.statements);
    let test_cases = [
        TreePairTestCase {
            description: "two pipe WHERE steps are not one WHERE with AND",
            sql: "FROM t |> WHERE a |> WHERE b",
            other_sql: "FROM t |> WHERE a AND b",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a pipe DISTINCT is not a pipe SELECT DISTINCT *",
            sql: "FROM t |> DISTINCT",
            other_sql: "FROM t |> SELECT DISTINCT *",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a pipe WHERE after SELECT AS STRUCT is not a WHERE over a subquery",
            sql: "SELECT AS STRUCT 1 a |> WHERE a = 1",
            other_sql: "SELECT * FROM (SELECT AS STRUCT 1 a) WHERE a = 1",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a piped FROM query is not a piped SELECT * FROM query",
            sql: "FROM t |> WHERE t.a = 1",
            other_sql: "SELECT * FROM t |> WHERE t.a = 1",
            expected_same: false,
        },
        TreePairTestCase {
            description: "an alias after a parenthesised query is a pipe AS step",
            sql: "(SELECT 1 a) AS s",
            other_sql: "(SELECT 1 a) |> AS s",
            expected_same: true,
        },
        TreePairTestCase {
            description: "ORDER BY after a parenthesised query is not the inner query's ORDER BY",
            sql: "(SELECT -x AS y FROM UNNEST([3, 1, 2]) x) ORDER BY x",
            other_sql: "SELECT -x AS y FROM UNNEST([3, 1, 2]) x ORDER BY x",
            expected_same: false,
        },
        TreePairTestCase {
            description: "ORDER BY after a parenthesised query is a pipe ORDER BY step",
            sql: "(SELECT -x AS y FROM UNNEST([3, 1, 2]) x) ORDER BY x",
            other_sql: "(SELECT -x AS y FROM UNNEST([3, 1, 2]) x) |> ORDER BY x",
            expected_same: true,
        },
        TreePairTestCase {
            description: "LIMIT after a parenthesised query is not a LIMIT over a subquery",
            sql: "(SELECT x FROM t ORDER BY x LIMIT 2) LIMIT 1",
            other_sql: "SELECT * FROM (SELECT x FROM t ORDER BY x LIMIT 2) LIMIT 1",
            expected_same: false,
        },
        TreePairTestCase {
            description: "LIMIT after a parenthesised query in ARRAY is not a LIMIT over a subquery",
            sql: "SELECT ARRAY((SELECT AS STRUCT 1 a LIMIT 5) LIMIT 1)",
            other_sql: "SELECT ARRAY(SELECT * FROM (SELECT AS STRUCT 1 a LIMIT 5) LIMIT 1)",
            expected_same: false,
        },
        TreePairTestCase {
            description: "GROUP AND ORDER BY is not GROUP BY",
            sql: "FROM t |> AGGREGATE COUNT(*) c GROUP AND ORDER BY x",
            other_sql: "FROM t |> AGGREGATE COUNT(*) c GROUP BY x",
            expected_same: false,
        },
        TreePairTestCase {
            description: "an aggregate's NULLS LAST is kept",
            sql: "FROM t |> AGGREGATE COUNT(*) c DESC NULLS LAST GROUP BY x",
            other_sql: "FROM t |> AGGREGATE COUNT(*) c DESC GROUP BY x",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a grouping item's NULLS FIRST is kept",
            sql: "FROM t |> AGGREGATE COUNT(*) c GROUP BY x NULLS FIRST",
            other_sql: "FROM t |> AGGREGATE COUNT(*) c GROUP BY x",
            expected_same: false,
        },
        TreePairTestCase {
            description: "pipe steps after a parenthesised WITH query are not inside it",
            sql: "(WITH c AS (SELECT 1 x) SELECT * FROM c |> AS s) |> WHERE x IN (FROM c)",
            other_sql: "WITH c AS (SELECT 1 x) SELECT * FROM c |> AS s |> WHERE x IN (FROM c)",
            expected_same: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            tree(test_case.sql) == tree(test_case.other_sql),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_joins_and_commas_when_parsing_then_their_order_and_grouping_are_kept() {
    let tree = |sql: &str| parse(sql).map(|parsed| parsed.statements);
    let test_cases = [
        TreePairTestCase {
            description: "a comma after a LEFT JOIN keeps its place",
            sql: "SELECT * FROM t1 LEFT JOIN t2 ON a, t3",
            other_sql: "SELECT * FROM t1, t3 LEFT JOIN t2 ON a",
            expected_same: false,
        },
        TreePairTestCase {
            description: "consecutive ON conditions keep their joins' grouping",
            sql: "SELECT * FROM t1 LEFT JOIN t2 JOIN t3 ON a JOIN t4 ON b ON c",
            other_sql: "SELECT * FROM t1 LEFT JOIN t2 JOIN t3 ON a ON c JOIN t4 ON b",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a chain of set operations is not its parenthesised first pair",
            sql: "SELECT 1 UNION ALL SELECT 2 UNION DISTINCT SELECT 3",
            other_sql: "(SELECT 1 UNION ALL SELECT 2) UNION DISTINCT SELECT 3",
            expected_same: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            tree(test_case.sql) == tree(test_case.other_sql),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_match_recognize_when_parsing_then_its_pattern_and_options_are_kept() {
    let tree = |(pattern, options): (&str, &str)| {
        parse(&format!(
            "SELECT * FROM t MATCH_RECOGNIZE (ORDER BY b MEASURES COUNT(*) AS n PATTERN ({pattern}) DEFINE a AS TRUE{options})"
        ))
        .map(|parsed| parsed.statements)
    };
    let test_cases = [
        MatchRecognizeTestCase {
            description: "a reluctant quantifier may have a blank before its ?",
            pattern: ("a+ ?", ""),
            other_pattern: ("a+?", ""),
            expected_same: true,
        },
        MatchRecognizeTestCase {
            description: "a reluctant quantifier is not a greedy one",
            pattern: ("a+?", ""),
            other_pattern: ("a+", ""),
            expected_same: false,
        },
        MatchRecognizeTestCase {
            description: "|| is two alternations",
            pattern: ("a||b", ""),
            other_pattern: ("a | | b", ""),
            expected_same: true,
        },
        MatchRecognizeTestCase {
            description: "a parenthesised pattern keeps its group",
            pattern: ("(a)", ""),
            other_pattern: ("a", ""),
            expected_same: false,
        },
        MatchRecognizeTestCase {
            description: "alternation is not grouped like concatenation",
            pattern: ("a b | c", ""),
            other_pattern: ("a (b | c)", ""),
            expected_same: false,
        },
        MatchRecognizeTestCase {
            description: "a backquoted pattern variable is one name",
            pattern: ("`a b`", ""),
            other_pattern: ("a b", ""),
            expected_same: false,
        },
        MatchRecognizeTestCase {
            description: "use_longest_match TRUE is not FALSE",
            pattern: ("a", " OPTIONS (use_longest_match = TRUE)"),
            other_pattern: ("a", " OPTIONS (use_longest_match = FALSE)"),
            expected_same: false,
        },
        MatchRecognizeTestCase {
            description: "an explicit use_longest_match = FALSE is kept",
            pattern: ("a", " OPTIONS (use_longest_match = FALSE)"),
            other_pattern: ("a", ""),
            expected_same: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            tree(test_case.pattern) == tree(test_case.other_pattern),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unpivot_labels_and_ddl_options_when_parsing_then_they_are_kept() {
    let tree = |sql: &str| parse(sql).map(|parsed| parsed.statements);
    let test_cases = [
        TreePairTestCase {
            description: "an UNPIVOT label keeps whether it is a string or a number",
            sql: "SELECT * FROM t UNPIVOT(a FOR b IN (c AS '1'))",
            other_sql: "SELECT * FROM t UNPIVOT(a FOR b IN (c AS 1))",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a view column's OPTIONS are kept",
            sql: "CREATE VIEW d.v (a OPTIONS(description='x'), b) AS SELECT 1, 2",
            other_sql: "CREATE VIEW d.v (a, b) AS SELECT 1, 2",
            expected_same: false,
        },
        TreePairTestCase {
            description: "OPTIONS += is not =",
            sql: "CREATE VIEW d.v OPTIONS(labels += [('a', 'b')]) AS SELECT 1 x",
            other_sql: "CREATE VIEW d.v OPTIONS(labels = [('a', 'b')]) AS SELECT 1 x",
            expected_same: false,
        },
        TreePairTestCase {
            description: "every CLUSTER BY expression of a materialized view is kept",
            sql: "CREATE MATERIALIZED VIEW d.v CLUSTER BY a, LOWER(b) AS SELECT 1 a",
            other_sql: "CREATE MATERIALIZED VIEW d.v CLUSTER BY a AS SELECT 1 a",
            expected_same: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            tree(test_case.sql) == tree(test_case.other_sql),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_whole_backquoted_path_when_parsing_then_it_stays_split_and_marked() {
    let tree = |sql: &str| parse(sql).map(|parsed| parsed.statements);
    let test_cases = [
        TreePairTestCase {
            description: "a whole backquoted path in FROM is not its backquoted parts",
            sql: "SELECT * FROM T1 t1, `t1.arr`",
            other_sql: "SELECT * FROM T1 t1, `t1`.`arr`",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a backquoted dotted alias then a part is not one whole path",
            sql: "SELECT * FROM T1 AS `t1.x`, `t1.x`.`arr`",
            other_sql: "SELECT * FROM T1 AS `t1.x`, `t1.x.arr`",
            expected_same: false,
        },
        TreePairTestCase {
            description: "parts holding escaped backquotes are not one whole backquoted part",
            sql: "UPDATE t SET a.`\\`b`.`c\\`` = 1 WHERE TRUE",
            other_sql: "UPDATE t SET a.`b.c` = 1 WHERE TRUE",
            expected_same: false,
        },
        TreePairTestCase {
            description: "where a whole backquoted path splits is kept",
            sql: "UPDATE t SET `a.b`.c = 1 WHERE TRUE",
            other_sql: "UPDATE t SET a.`b.c` = 1 WHERE TRUE",
            expected_same: false,
        },
        TreePairTestCase {
            description: "a whole backquoted path is marked apart from a plain path",
            sql: "UPDATE t SET `a.b`.c = 1 WHERE TRUE",
            other_sql: "UPDATE t SET a.b.c = 1 WHERE TRUE",
            expected_same: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            tree(test_case.sql) == tree(test_case.other_sql),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}
