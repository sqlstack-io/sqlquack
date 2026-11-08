//! Verdicts and trees of the DuckDB parser, as DuckDB 2.0.0's `extract_statements` gives them.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::parse::errors::StrictFailure;
use crate::parse::main::parse::parse;
use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::helpers::{
    agrees_with_duckdb, nested_struct_lists, nested_unions, reaches_gap,
};
use crate::parse::tests::test_types::{
    CorpusTestCase, ExpectedVerdict, FailureTestCase, GapTestCase, NestingTestCase,
    RecordedCorpusTestCase, StrictTestCase, ValueTestCase, VerdictTestCase,
};
use sqlquack_syntax::{Gap, Parsed};

#[test]
fn given_sql_when_parsing_then_the_verdict_is_duckdb() {
    let test_cases = [
        VerdictTestCase {
            description: "an empty input is no statements, not an error",
            sql: " ;; ",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SHOW TABLES FROM takes a string part after a dotted name",
            sql: "SHOW TABLES FROM memory.'main'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a CTE body may be a TRUNCATE",
            sql: "WITH c AS (TRUNCATE t1) SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a DML CTE body may start with its own WITH",
            sql: "WITH c AS (WITH d AS (SELECT 1) DELETE FROM t1) SELECT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a MERGE CTE body is refused even after its own WITH",
            sql: "WITH c AS (WITH d AS (SELECT 1) MERGE INTO t USING s ON true WHEN MATCHED THEN DELETE) SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a typed literal's type may be qualified",
            sql: "SELECT s1.mood 'a'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a macro's overloads parse",
            sql: "CREATE MACRO f(a) AS a, (a, b) AS a + b",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "table and scalar macro overloads are refused together",
            sql: "CREATE MACRO f(a) AS a, (a, b) AS TABLE SELECT a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a SET column list takes a row of as many values",
            sql: "UPDATE t1 SET (c2, c3) = (1, 2)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a SET column list refuses a row of another length",
            sql: "UPDATE t1 SET (c2, c3) = (1, 2, 3)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "OVERRIDING USER VALUE and DROP VIEW CASCADE and DELETE USING a subquery are trees",
            sql: "INSERT INTO t1 OVERRIDING USER VALUE VALUES (1); DROP VIEW v1 CASCADE; DELETE FROM t1 USING (SELECT 1 AS a) s WHERE t1.c1 = s.a",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a parenthesized row of another length is refused",
            sql: "UPDATE t1 SET (c2, c3) = (('a', 'b', 'c'))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a qualified row call of another length is refused",
            sql: "UPDATE t1 SET (c2, c3) = system.main.row(1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a quoted ROW call of another length is refused",
            sql: "UPDATE t1 SET (c2, c3) = \"ROW\"(1, 2, 3)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "FROM may come first, with or without a SELECT",
            sql: "FROM t SELECT a; FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "-> is a lambda inside a list function and JSON extraction elsewhere",
            sql: "SELECT list_transform([1, 2], x -> x + 1), j -> '$.a', j ->> 'b' FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a prefix alias names its expression",
            sql: "SELECT x: 1, y: 2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "strings separated by a newline concatenate",
            sql: "SELECT 'a'\n'b'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "strings on one line do not",
            sql: "SELECT 'a' 'b'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a star takes EXCLUDE and REPLACE, and COLUMNS takes a pattern or lambda",
            sql: "SELECT * EXCLUDE (a) REPLACE (b + 1 AS b), COLUMNS('a.*'), \
                  COLUMNS(c -> c LIKE 'x%') FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "list comprehensions, struct and map literals, and slices",
            sql: "SELECT [x * 2 FOR x IN [1, 2] IF x > 1], {'a': 1}.a, MAP {'k': 1}['k'], \
                  [1, 2, 3][2:3]",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a select list cannot be empty, which the transformer reports",
            sql: "SELECT DISTINCT FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ORDER BY ALL is refused inside a window",
            sql: "SELECT sum(a) OVER (ORDER BY ALL) FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "but GROUP BY ALL and ORDER BY ALL are clauses of a query",
            sql: "SELECT a, count(*) FROM t GROUP BY ALL ORDER BY ALL DESC",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "NULLIF takes exactly two arguments in the grammar",
            sql: "SELECT NULLIF(1, 2, )",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "PERCENTILE_CONT WITHIN GROUP takes one argument",
            sql: "SELECT percentile_cont(0.5, 0.6) WITHIN GROUP (ORDER BY a) FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "MODE takes none and PERCENTILE_DISC one",
            sql: "SELECT mode() WITHIN GROUP (ORDER BY a), \
                  percentile_disc(0.5) WITHIN GROUP (ORDER BY a) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "TRY takes one argument",
            sql: "SELECT TRY(1, 2)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "IFNULL takes two",
            sql: "SELECT IFNULL(1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a number has at most one point",
            sql: "SELECT 1.2.3",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "hex, binary, separated and point-first numbers, and a hex string",
            sql: "SELECT 0x1F, 0b101, 1_000, .5e3, x'1F'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ASOF and POSITIONAL joins",
            sql: "SELECT * FROM t ASOF LEFT JOIN u USING (k, ts) POSITIONAL JOIN v",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a plain JOIN needs ON or USING",
            sql: "SELECT * FROM t JOIN u",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a NATURAL JOIN takes neither",
            sql: "SELECT * FROM t NATURAL JOIN u ON t.a = u.a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CUBE and ROLLUP may not be empty",
            sql: "SELECT a FROM t GROUP BY CUBE ()",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a reserved word is a column label only after AS",
            sql: "SELECT 1 select",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "with AS it is a label, and an unreserved word needs no AS",
            sql: "SELECT 1 AS select, 2 date",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a type modifier must be a constant",
            sql: "SELECT a::DECIMAL(b, 2) FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "nested and parameterised types",
            sql: "SELECT CAST(a AS DECIMAL(10, 2)), a::INTEGER[], \
                  TRY_CAST(a AS STRUCT(x INT, y VARCHAR)) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "GEOMETRY is a function, but GEOMETRY() is not a type",
            sql: "SELECT CAST(a AS GEOMETRY())",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a statement may not mix named and positional parameters",
            sql: "SELECT (SELECT $x), ?",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "but separate statements may, and ? and $n are both positional",
            sql: "SELECT ?, $1; SELECT $name",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "recursive CTEs take USING KEY, and set operations BY NAME",
            sql: "WITH RECURSIVE r(n) USING KEY (n) AS (SELECT 1 UNION SELECT n + 1 FROM r) \
                  SELECT * FROM r UNION ALL BY NAME SELECT 2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a macro is a function, with a default parameter",
            sql: "CREATE OR REPLACE MACRO m(a, b := 2) AS a + b",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DML after WITH is a tree",
            sql: "WITH c AS (SELECT 1 AS x) INSERT INTO t SELECT * FROM c",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MERGE is a tree",
            sql: "MERGE INTO t USING s ON t.a = s.a WHEN MATCHED THEN DELETE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a SET column is an identifier, so a reserved word is refused",
            sql: "UPDATE t1 SET left = 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "the transformer refuses a qualified SET column",
            sql: "UPDATE t1 SET a.b = 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "UPDATE EXTENSIONS is a statement of its own, kept as a gap",
            sql: "UPDATE EXTENSIONS (json)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a table named extensions is still updated",
            sql: "UPDATE extensions SET a = 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a duplicate column is the catalog's refusal, not the parser's",
            sql: "CREATE TABLE t (a INT, a INT)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a column needs a type unless it is generated",
            sql: "CREATE TABLE t (a)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a generated column needs no type",
            sql: "CREATE TABLE t (a GENERATED ALWAYS AS (1) VIRTUAL, b INT AS (a + 1))",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "OR REPLACE and IF NOT EXISTS together are refused",
            sql: "CREATE OR REPLACE TABLE IF NOT EXISTS t AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a foreign key may not cascade",
            sql: "CREATE TABLE t (a INT REFERENCES u ON DELETE CASCADE)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE AS takes names and WITH NO DATA",
            sql: "CREATE TABLE t (x, y) AS SELECT 1, 2 WITH NO DATA",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a reserved word is no unqualified table name",
            sql: "CREATE TABLE select (a INT)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DROP takes one object at a time",
            sql: "DROP TABLE IF EXISTS s.t, u CASCADE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a recursive view needs a column list",
            sql: "CREATE RECURSIVE VIEW v AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a descending sequence starts at most at -1 unless MAXVALUE says otherwise",
            sql: "CREATE SEQUENCE s INCREMENT -1 START 5",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a sequence option folds parentheses and minus signs",
            sql: "CREATE SEQUENCE s START (-5) MINVALUE - -10 INCREMENT BY 5 - 3",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a negative start needs a lower MINVALUE",
            sql: "CREATE SEQUENCE s START WITH -5 MINVALUE -10",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a sequence option must be an integer",
            sql: "CREATE SEQUENCE s START 1e3",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a sequence option outside BIGINT is refused",
            sql: "CREATE SEQUENCE s MINVALUE -9223372036854775809",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "the lowest BIGINT is a sequence bound",
            sql: "CREATE SEQUENCE s INCREMENT -1 MINVALUE -9223372036854775808",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "OWNED BY is parsed but refused",
            sql: "CREATE SEQUENCE s OWNED BY t.a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a sequence name has at most three parts when dropped",
            sql: "DROP SEQUENCE a.b.c.d",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a reserved word may end a dropped type's qualified name",
            sql: "DROP TYPE s1.SELECT",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a reserved word may not end a dropped sequence's name",
            sql: "DROP SEQUENCE s1.SELECT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "PERSISTENT is only for secrets",
            sql: "CREATE PERSISTENT SEQUENCE s",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an index needs a name",
            sql: "CREATE INDEX ON t (a)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an index cannot be temporary",
            sql: "CREATE TEMP INDEX i ON t (a)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an index option must be a constant",
            sql: "CREATE INDEX i ON t (a) WITH (x = y)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a CHECK holds no subquery, not even under IN",
            sql: "CREATE TABLE t (a INT CHECK (1 IN (FROM u)))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a generated column refuses a qualified column",
            sql: "CREATE TABLE t (a INT AS (t.b))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a generated column accepts a method call",
            sql: "CREATE TABLE t (a INT AS (b.lower()))",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an unknown compression is refused",
            sql: "CREATE TABLE t (a INT USING COMPRESSION bogus)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a trigger's REFERENCING aliases differ without case",
            sql: "CREATE TRIGGER tr AFTER INSERT ON t REFERENCING NEW TABLE AS a OLD TABLE AS A DELETE FROM u",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a trigger's body is a DML statement",
            sql: "CREATE TRIGGER tr AFTER INSERT ON t SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unnamed secret needs a constant type",
            sql: "CREATE SECRET (TYPE f(1))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a comparison in a secret option is refused only after parsing",
            sql: "CREATE SECRET s (TYPE s3, k a > ALL (SELECT 1))",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a secret option names each key once without case",
            sql: "CREATE SECRET s (TYPE s3, k 1, K 2)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "OR in a secret option is refused only after parsing",
            sql: "CREATE SECRET s (TYPE s3, k a IS NOT DISTINCT FROM b OR TRUE)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a named secret is accepted as a gap",
            sql: "CREATE SECRET s (TYPE s3, KEY_ID 'x', SCOPE ['a', 'b'])",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a view takes only DEFER_BINDING, as a boolean",
            sql: "CREATE VIEW v WITH (DEFER_BINDING = 'x') AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a recursive view refuses LIMIT on its union",
            sql: "CREATE RECURSIVE VIEW v (a) AS SELECT 1 UNION SELECT 2 OFFSET 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "DROP COLLATION is not implemented",
            sql: "DROP COLLATION c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a temporary secret has no storage to drop from",
            sql: "DROP TEMPORARY SECRET s FROM x",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "DROP SCHEMA takes a qualified name",
            sql: "DROP SCHEMA IF EXISTS a.b CASCADE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an enum may be built from a query",
            sql: "CREATE TYPE t AS ENUM (SELECT 1)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "ALTER TABLE takes one action",
            sql: "ALTER TABLE t RENAME TO u, RENAME TO v",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ALTER TABLE allows a trailing comma after its action",
            sql: "ALTER TABLE t RENAME TO u,",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an added column cannot be a primary key",
            sql: "ALTER TABLE t ADD a INT PRIMARY KEY",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an added column needs a type",
            sql: "ALTER TABLE t ADD COLUMN a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an added column with IF NOT EXISTS and NOT NULL is left to a later stage",
            sql: "ALTER TABLE t ADD COLUMN IF NOT EXISTS a INT NOT NULL",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a nested field cannot be added NOT NULL",
            sql: "ALTER TABLE t ADD a.b INT NOT NULL",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an added column takes repeated collations",
            sql: "ALTER TABLE t ADD a VARCHAR COLLATE nocase COLLATE de",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "only a VARCHAR column takes a collation",
            sql: "CREATE TABLE t (a INT COLLATE nocase)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a qualified VARCHAR type takes a collation",
            sql: "CREATE TABLE t (a main.varchar COLLATE nocase)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a new type may be left out only with USING",
            sql: "ALTER TABLE t ALTER a TYPE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a collation needs the VARCHAR type written",
            sql: "ALTER TABLE t ALTER a TYPE COLLATE nocase USING 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "DROP CONSTRAINT is not supported",
            sql: "ALTER TABLE t DROP CONSTRAINT k",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a reset option holds no value but NULL",
            sql: "ALTER TABLE t RESET (a = TRUE)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a reset option may be NONE",
            sql: "ALTER TABLE t RESET (a = NONE)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "SORTED BY takes a parenthesised list",
            sql: "ALTER TABLE t SET SORTED BY ALL",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a renamed table takes one name",
            sql: "ALTER TABLE IF EXISTS t RENAME TO s.u",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "renaming a sequence is not supported",
            sql: "ALTER SEQUENCE s RENAME TO r",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ALTER SEQUENCE needs OWNED BY",
            sql: "ALTER SEQUENCE s INCREMENT 2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ALTER SEQUENCE leaves NULL values to the statement it does not build",
            sql: "ALTER SEQUENCE s OWNED BY t START NULL",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a sequence option is still an integer in ALTER SEQUENCE",
            sql: "ALTER SEQUENCE s START 1.5 OWNED BY t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "renaming a schema is not supported",
            sql: "ALTER SCHEMA s RENAME TO r",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ALTER DATABASE takes a plain alias",
            sql: "ALTER DATABASE d SET ALIAS TO 'e'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a NULL sequence start is refused by CREATE SEQUENCE",
            sql: "CREATE SEQUENCE s START NULL",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a PIVOT statement needs one of its clauses",
            sql: "PIVOT t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a PIVOT statement is a query a set operation takes",
            sql: "SELECT 1 UNION ALL PIVOT t ON a IN (SELECT 1) USING sum(b) GROUP BY (c, d)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an UNPIVOT statement needs its ON list",
            sql: "UNPIVOT t INTO NAME k VALUE v",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "every list takes a trailing comma",
            sql: "SELECT * FROM t, u, GROUP BY a, ORDER BY a,",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a column list takes a trailing comma",
            sql: "CREATE TABLE t (a INT, PRIMARY KEY (a,),)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a list takes one trailing comma, not two",
            sql: "SELECT * FROM t ORDER BY a,,",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ORDER BY ALL stands alone",
            sql: "SELECT 1 ORDER BY ALL, a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a lone DESCRIBE of a table takes no ORDER BY, even in parentheses",
            sql: "(DESCRIBE t) ORDER BY 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a DESCRIBE inside a set operation takes the chain's ORDER BY",
            sql: "DESCRIBE t UNION ALL SELECT 1 ORDER BY 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a lone SUMMARIZE takes no CTE",
            sql: "WITH q AS (SELECT 1) SUMMARIZE t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a syntax error after a bare SUMMARIZE comes before its refusal",
            sql: "SUMMARIZE ALL TABLES",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "DESCRIBE of a listing name is the listing",
            sql: "DESCRIBE TABLES",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a MERGE takes every action in every clause kind",
            sql: "WITH s AS (SELECT 1 AS k) MERGE INTO t USING s ON t.k = s.k WHEN MATCHED AND t.a THEN \
                  UPDATE SET * WHEN MATCHED THEN ERROR 'x' WHEN NOT MATCHED THEN INSERT BY NAME * \
                  WHEN NOT MATCHED BY SOURCE THEN DELETE RETURNING merge_action",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a MERGE clause after an unconditional one of its kind is unreachable",
            sql: "MERGE INTO t USING s ON 1 WHEN NOT MATCHED THEN INSERT * WHEN NOT MATCHED BY TARGET THEN INSERT *",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a syntax error after an unreachable MERGE clause comes before its refusal",
            sql: "MERGE INTO t USING s ON 1 WHEN MATCHED THEN DELETE WHEN MATCHED THEN DELETE garbage",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "MERGE INSERT takes its columns only with VALUES",
            sql: "MERGE INTO t USING s ON 1 WHEN NOT MATCHED THEN INSERT (a) DEFAULT VALUES",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a CTE body is a query or DML, not a MERGE",
            sql: "WITH s AS (MERGE INTO t USING u ON 1 WHEN MATCHED THEN DELETE) SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a CTE body is not DDL",
            sql: "WITH s AS (DROP TABLE t) SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ATTACH, SET, a transaction and COMMENT ON are trees",
            sql: "BEGIN TRANSACTION READ ONLY; ATTACH 'f.duckdb' AS shop (TYPE sqlite, READ_ONLY); \
                  SET GLOBAL threads = 1; SET TIME ZONE 'UTC'; COMMENT ON COLUMN s.t.c IS NULL; \
                  DETACH DATABASE IF EXISTS shop; ROLLBACK",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a name before ATTACH options that are no call's arguments is the path",
            sql: "ATTACH f (TYPE sqlite)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a syntax error comes before a duplicate ATTACH option's refusal",
            sql: "ATTACH 'f' AS x (TYPE sqlite, type duckdb) garbage",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET takes one value",
            sql: "SET VARIABLE x = 1, 2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a scope word alone is no setting name",
            sql: "SET local = 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a column comment names its table",
            sql: "COMMENT ON COLUMN c IS 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a comment is a string or NULL",
            sql: "COMMENT ON TABLE t IS 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a column both excluded and replaced is refused, qualified or not",
            sql: "SELECT * EXCLUDE (t.a) REPLACE (1 AS A) FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a star takes one qualifier",
            sql: "SELECT a.b.* FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "IGNORE NULLS needs a window",
            sql: "SELECT first(x IGNORE NULLS) FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a qualified OPERATOR is infix too, and BETWEEN without a range an alias",
            sql: "SELECT 1 OPERATOR(pg_catalog.+) 2, x BETWEEN FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a tokenizer fault comes before any statement's refusal",
            sql: "COMMENT ON COLUMN c IS 'x'; SELECT \"open",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COPY takes a table or a query, with generic or specialized options",
            sql: "COPY t (a, b) FROM 'f' USING DELIMITERS ',' WITH CSV HEADER; \
                  COPY (SELECT 1) TO 'f' (FORMAT parquet, PARTITION_BY (a), ORDER BY (x DESC)); \
                  COPY FROM DATABASE a TO b (SCHEMA)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a COPY option is set once, whichever way it is written",
            sql: "COPY t TO 'f' CSV BINARY",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "COPY FROM is no CTE body",
            sql: "WITH a AS (COPY t FROM 'f') SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "PRAGMA, TRUNCATE and USE are trees",
            sql: "PRAGMA table_info('t'); PRAGMA threads = 4; PRAGMA f(a.b = 1, 2); \
                  TRUNCATE TABLE s.t AS x RESTRICT; USE shop.main",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a pragma's named parameter has a column on its left",
            sql: "PRAGMA f((1 = 1))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "USE names a database and at most a schema",
            sql: "USE a.b.c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a scope word before RESET's name is always a scope",
            sql: "RESET local",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "two comparisons in a row are refused, after an IS test too",
            sql: "SELECT a IS NULL = b < c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an IS test separates two comparisons",
            sql: "SELECT a = b IS NULL = c",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "table options are trees, with a keyword, NONE or an expression as a value",
            sql: "CREATE TABLE t (a INT) WITH (x = select, y = NONE, 'z' = lower('a')); \
                  ALTER TABLE t SET (a.b = -1); ALTER TABLE t RESET (x = NULL)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a string or number option value stands alone",
            sql: "CREATE TABLE t (a INT) WITH (x = 'a'::INT)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a reset option takes no value",
            sql: "ALTER TABLE t RESET (x = 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an option name is set once, in any case",
            sql: "CREATE TABLE t (a INT) WITH (x = 1, X = 2)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "INSTALL is a tree, from a repository name or URL",
            sql: "FORCE INSTALL httpfs FROM community; INSTALL 'spatial' FROM 'https://x'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "INSTALL AND LOAD and VERSION are gaps the tree cannot hold",
            sql: "INSTALL AND LOAD httpfs VERSION 'v1'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a LOAD alias follows its source",
            sql: "LOAD httpfs AS h FROM community",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "FORCE starts only INSTALL and CHECKPOINT",
            sql: "FORCE LOAD httpfs",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an extension name is no function-name keyword",
            sql: "UPDATE EXTENSIONS (left)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a string continued on the next line names a LOAD or USE target",
            sql: "USE 'a'\n'b'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DELETE takes FROM",
            sql: "DELETE t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a PIVOT entry's IN filter ends the entry, so no operator may follow it",
            sql: "PIVOT t1 ON c1 IN (1) IS NULL",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "LIMIT and FETCH never count rows together",
            sql: "SELECT 1 UNION SELECT 2 LIMIT 1 FETCH FIRST 1 ROW ONLY",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "PERCENT follows only a number",
            sql: "SELECT 1 LIMIT (10) PERCENT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "EXCEPT takes no BY NAME",
            sql: "SELECT 1 EXCEPT ALL BY NAME SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a frame starting after the current row cannot end before it",
            sql: "SELECT sum(1) OVER (ROWS BETWEEN 1 FOLLOWING AND 1 PRECEDING)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a sample percentage is at most 100",
            sql: "FROM t1 USING SAMPLE 100.5%",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a decimal row count rounds half to even before its bound",
            sql: "FROM t1 USING SAMPLE 1000000000.5 ROWS",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an int64 row count past the bound is compared exactly",
            sql: "FROM t1 USING SAMPLE 9223372036854775296 ROWS",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a window definition holds no window function",
            sql: "SELECT sum(x) OVER (ORDER BY rank() OVER ()) FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a window function's arguments may hold one",
            sql: "SELECT sum(rank() OVER ()) OVER () FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an interval range is refused after parsing",
            sql: "SELECT INTERVAL '1-2' YEAR TO MONTH",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ANY over a subquery needs a comparison",
            sql: "SELECT 1 + ANY (SELECT 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a recursive CTE takes no DML body",
            sql: "WITH RECURSIVE t1 AS (DELETE FROM t2 RETURNING *) SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a named column constraint needs its constraint",
            sql: "CREATE TABLE t1 (c1 INTEGER CONSTRAINT k1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COLLATE is not a constraint a name can precede",
            sql: "CREATE TABLE t1 (c1 VARCHAR CONSTRAINT k1 COLLATE nocase)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "START WITH takes a constant, not a subtraction",
            sql: "CREATE SEQUENCE s1 START WITH 5 - 3",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "WITHIN GROUP takes one ordering",
            sql: "SELECT mode() WITHIN GROUP (ORDER BY x, y) FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a call the transformer rewrites takes no named argument",
            sql: "SELECT ifnull(a := 1, b := 2)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "NOT cannot precede an already negated regexp match",
            sql: "SELECT 'a' NOT !~ 'b'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a case-insensitive regexp match takes no ESCAPE",
            sql: "SELECT 'a' ~* 'b' ESCAPE 'c'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a named argument is never a reserved word",
            sql: "SELECT f(select := 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a sample size is a constant",
            sql: "FROM t1 TABLESAMPLE ?",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a LATERAL reference takes no sample",
            sql: "FROM t1, LATERAL (SELECT 1) s TABLESAMPLE 10%",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "PREPARE takes only a query, INSERT, UPDATE, DELETE or COPY",
            sql: "PREPARE s1 AS CREATE TABLE t1 (c1 INTEGER)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a PREPARE type list is left to the stage that refuses it",
            sql: "PREPARE s1 (INTEGER) AS CREATE TABLE t1 (c1 INTEGER)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DEALLOCATE PREPARE needs a name after PREPARE",
            sql: "DEALLOCATE PREPARE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CALL needs an argument list",
            sql: "CALL range",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "EXPLAIN takes no ATTACH",
            sql: "EXPLAIN ATTACH 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "EXPLAIN ANALYZE reads ANALYZE as its own, not as a statement",
            sql: "EXPLAIN ANALYZE t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "EXPLAIN (SQL) refuses a PIVOT whose values come from the data",
            sql: "EXPLAIN (SQL) SELECT * FROM (PIVOT t1 ON c1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a view refuses a PIVOT whose values come from the data",
            sql: "CREATE VIEW v1 AS PIVOT t1 ON c1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a view takes a PIVOT whose values are listed",
            sql: "CREATE VIEW v1 AS PIVOT t1 ON c1 IN (1, 2)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "VACUUM refuses an option it does not know",
            sql: "VACUUM (analyze, foo) t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "VACUUM's legacy options come in their order",
            sql: "VACUUM ANALYZE FULL",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an EXPORT DATABASE format is a name or a string",
            sql: "EXPORT DATABASE 'd1' (FORMAT 1 + 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "IMPORT DATABASE takes a string",
            sql: "IMPORT DATABASE d1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "the scanner refuses an unterminated comment",
            sql: "SELECT 1 /* open",
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
            description: "SQLBuild's type marker asserts its type around its expression",
            sql: "SELECT __polyglot_typed(t.x, 'varchar(5)') AS v FROM t",
            pointer: "/0/select/expressions/0/alias/this/annotated/type_assertion/data_type",
            expected_json: r#""var_char""#,
        },
        ValueTestCase {
            description: "SQLBuild's marker name is compared without case",
            sql: "SELECT __POLYGLOT_TYPED(x, 'BIGINT') FROM t",
            pointer: "/0/select/expressions/0/annotated/type_assertion/data_type",
            expected_json: r#""big_int""#,
        },
        ValueTestCase {
            description: "SQLBuild's typed-like marker keeps the expression it is typed like",
            sql: "SELECT __polyglot_typed_like(x, y + 1) FROM t",
            pointer: "/0/select/expressions/0/annotated/type_like/add/right/literal/value",
            expected_json: r#""1""#,
        },
        ValueTestCase {
            description: "a type marker whose type text does not parse stays a call",
            sql: "SELECT __polyglot_typed(x, 'int garbage') FROM t",
            pointer: "/0/select/expressions/0/function/name",
            expected_json: r#""__polyglot_typed""#,
        },
        ValueTestCase {
            description: "a quoted marker name is a call, not a marker",
            sql: "SELECT \"__polyglot_typed\"(x, 'BIGINT') FROM t",
            pointer: "/0/select/expressions/0/function/name",
            expected_json: r#""__polyglot_typed""#,
        },
        ValueTestCase {
            description: "FORCE CHECKPOINT is the force_checkpoint call",
            sql: "FORCE CHECKPOINT \"Db1\"",
            pointer: "/0/call/name/2/name",
            expected_json: r#""force_checkpoint""#,
        },
        ValueTestCase {
            description: "CHECKPOINT calls system.main.checkpoint, not a macro named checkpoint",
            sql: "CHECKPOINT",
            pointer: "/0/call/name/0/name",
            expected_json: r#""system""#,
        },
        ValueTestCase {
            description: "a checkpoint's catalog is its argument, as written",
            sql: "FORCE CHECKPOINT \"Db1\"",
            pointer: "/0/call/arguments/0/literal/value",
            expected_json: r#""Db1""#,
        },
        ValueTestCase {
            description: "a column keeps its qualifier",
            sql: "SELECT t.a FROM s.t AS t",
            pointer: "/0/select/expressions/0/column/table/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "a table keeps its schema",
            sql: "SELECT t.a FROM s.t AS t",
            pointer: "/0/select/from/expressions/0/table/schema/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "a PIVOT entry's IN filter stays a filter before ORDER BY",
            sql: "PIVOT t1 ON c1 IN (1, 2) ORDER BY c2",
            pointer: "/0/select/from/expressions/0/subquery/this/pivot/expressions/0/in/expressions/1/literal/value",
            expected_json: "\"2\"",
        },
        ValueTestCase {
            description: "a qualified table's quoted last part keeps its quotes",
            sql: "SELECT * FROM s1.'t1'",
            pointer: "/0/select/from/expressions/0/table/name/name",
            expected_json: "\"'t1'\"",
        },
        ValueTestCase {
            description: "~ is a partial regexp match, not SIMILAR TO",
            sql: "SELECT 'abc' ~ 'b'",
            pointer: "/0/select/expressions/0/function/name",
            expected_json: "\"regexp_matches\"",
        },
        ValueTestCase {
            description: "POSITION(a IN b) searches b for a",
            sql: "SELECT POSITION('b' IN 'abc')",
            pointer: "/0/select/expressions/0/function/args/0/literal/value",
            expected_json: "\"abc\"",
        },
        ValueTestCase {
            description: "OVERLAY takes a plain argument list",
            sql: "SELECT overlay('Txxxxas', 'hom', 2, 4)",
            pointer: "/0/select/expressions/0/function/args/3/literal/value",
            expected_json: "\"4\"",
        },
        ValueTestCase {
            description: "a dotted collation keeps every part",
            sql: "SELECT 'hello' COLLATE NOCASE.NOACCENT",
            pointer: "/0/select/expressions/0/collation/collation",
            expected_json: "\"NOCASE.NOACCENT\"",
        },
        ValueTestCase {
            description: "a named argument may be named by a type keyword",
            sql: "SELECT * FROM read_csv('f.csv', columns => {'a': 'INTEGER'})",
            pointer: "/0/select/from/expressions/0/function/args/1/named_argument/name/name",
            expected_json: "\"columns\"",
        },
        ValueTestCase {
            description: "ON CONFLICT keeps its conflict columns",
            sql: "INSERT INTO t2 VALUES (1) ON CONFLICT (c1) DO NOTHING",
            pointer: "/0/insert/on_conflict/on_conflict/conflict_keys/tuple/expressions/0/identifier/name",
            expected_json: "\"c1\"",
        },
        ValueTestCase {
            description: "a column DEFAULT stops before NOT NULL",
            sql: "CREATE TABLE t1 (c1 INTEGER DEFAULT 1 NOT NULL)",
            pointer: "/0/create_table/columns/0/constraints/0",
            expected_json: "\"NotNull\"",
        },
        ValueTestCase {
            description: "a PARTITION_PATH value keeps any expression",
            sql: "COPY t1 TO 'p' (FORMAT parquet, PARTITION_BY (c1), PARTITION_PATH c1 = 1)",
            pointer: "/0/copy/params/2/value/eq/right/literal/value",
            expected_json: "\"1\"",
        },
        ValueTestCase {
            description: "a qualified aggregate keeps its name's parts",
            sql: "SELECT main.sum(DISTINCT x) FROM t",
            pointer: "/0/select/expressions/0/aggregate_function/qualified_name/0/name",
            expected_json: "\"main\"",
        },
        ValueTestCase {
            description: "a method call keeps DISTINCT",
            sql: "SELECT (k).variant_group_object(DISTINCT v) FROM t",
            pointer: "/0/select/expressions/0/method_call/distinct",
            expected_json: "true",
        },
        ValueTestCase {
            description: "EXPORT_STATE is kept on the call",
            sql: "SELECT bit_and(x) EXPORT_STATE FROM t",
            pointer: "/0/select/expressions/0/function/export_state",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a quoted dotted type name keeps its quotes",
            sql: "SELECT NULL::\"s1.mood\"",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"\\\"s1.mood\\\"\"",
        },
        ValueTestCase {
            description: "a custom type keeps its modifiers",
            sql: "SELECT NULL::GEOMETRY('OGC:CRS84')",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"GEOMETRY('OGC:CRS84')\"",
        },
        ValueTestCase {
            description: "CREATE TEMP TYPE keeps TEMP",
            sql: "CREATE TEMP TYPE t1 AS INTEGER",
            pointer: "/0/create_type/temporary",
            expected_json: "true",
        },
        ValueTestCase {
            description: "DROP INDEX keeps CASCADE",
            sql: "DROP INDEX n1 CASCADE",
            pointer: "/0/drop_object/cascade",
            expected_json: "true",
        },
        ValueTestCase {
            description: "SHOW of a string looks it up as a setting, not as a listing",
            sql: "SHOW 'tables'",
            pointer: "/0/show/this",
            expected_json: "\"SETTING\"",
        },
        ValueTestCase {
            description: "SHOW TABLES FROM keeps the database it lists",
            sql: "SHOW TABLES FROM m1.s1",
            pointer: "/0/show/from/table/schema/name",
            expected_json: "\"m1\"",
        },
        ValueTestCase {
            description: "PREPARE keeps the statement it prepares",
            sql: "PREPARE s1 AS SELECT 1",
            pointer: "/0/prepare/statement/select/expressions/0/literal/value",
            expected_json: "\"1\"",
        },
        ValueTestCase {
            description: "EXECUTE keeps its named arguments",
            sql: "EXECUTE s1(a := 2)",
            pointer: "/0/execute/arguments/0/named_argument/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "DEALLOCATE drops a prepared statement",
            sql: "DEALLOCATE PREPARE s1",
            pointer: "/0/drop_object/kind",
            expected_json: "\"PREPARED STATEMENT\"",
        },
        ValueTestCase {
            description: "CALL keeps the function's qualified name",
            sql: "CALL main.range(3)",
            pointer: "/0/call/name/0/name",
            expected_json: "\"main\"",
        },
        ValueTestCase {
            description: "EXPLAIN is a describe styled EXPLAIN, with its options",
            sql: "EXPLAIN ANALYZE (FORMAT json) SELECT 1",
            pointer: "/0/describe/style",
            expected_json: "\"EXPLAIN ANALYZE\"",
        },
        ValueTestCase {
            description: "VACUUM keeps its options and columns",
            sql: "VACUUM ANALYZE t1 (c1)",
            pointer: "/0/analyze/options/0/var/this",
            expected_json: "\"analyze\"",
        },
        ValueTestCase {
            description: "EXPORT DATABASE keeps the database it exports",
            sql: "EXPORT DATABASE m1 TO 'd1'",
            pointer: "/0/export/connection/identifier/name",
            expected_json: "\"m1\"",
        },
        ValueTestCase {
            description: "IMPORT DATABASE is the import_database pragma",
            sql: "IMPORT DATABASE 'd1'",
            pointer: "/0/pragma/args/0/literal/value",
            expected_json: "\"d1\"",
        },
        ValueTestCase {
            description: "a table keeps its alias",
            sql: "SELECT t.a FROM s.t AS t",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "a FROM-first query keeps its source",
            sql: "FROM t SELECT a",
            pointer: "/0/select/from/expressions/0/table/name/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "a CTE keeps its name",
            sql: "WITH c AS (SELECT 1 AS x) SELECT x FROM c",
            pointer: "/0/select/with/ctes/0/alias/name",
            expected_json: "\"c\"",
        },
        ValueTestCase {
            description: "a prefix alias is an alias",
            sql: "SELECT x: 1",
            pointer: "/0/select/expressions/0/alias/alias/name",
            expected_json: "\"x\"",
        },
        ValueTestCase {
            description: "a star keeps its EXCLUDE list",
            sql: "SELECT * EXCLUDE (b) FROM t",
            pointer: "/0/select/expressions/0/star/except/0/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "a star keeps its RENAME pairs",
            sql: "SELECT * RENAME (a AS b) FROM t",
            pointer: "/0/select/expressions/0/star/rename/0/1/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "an ASOF LEFT JOIN keeps its kind",
            sql: "SELECT a FROM t ASOF LEFT JOIN u ON t.x >= u.x",
            pointer: "/0/select/joins/0/kind",
            expected_json: "\"AsOfLeft\"",
        },
        ValueTestCase {
            description: "a joined table keeps its name",
            sql: "SELECT a FROM t ASOF LEFT JOIN u ON t.x >= u.x",
            pointer: "/0/select/joins/0/this/table/name/name",
            expected_json: "\"u\"",
        },
        ValueTestCase {
            description: "a table function keeps its name and alias",
            sql: "SELECT x FROM read_parquet('f.parquet') r",
            pointer: "/0/select/from/expressions/0/alias/this/function/name",
            expected_json: "\"read_parquet\"",
        },
        ValueTestCase {
            description: "a lambda is a lambda, not JSON extraction",
            sql: "SELECT list_transform(l, x -> x + 1) FROM t",
            pointer: "/0/select/expressions/0/function/args/1/lambda/parameters/0/name",
            expected_json: "\"x\"",
        },
        ValueTestCase {
            description: "a macro keeps its name and parameters",
            sql: "CREATE MACRO m(a) AS a + 1",
            pointer: "/0/create_function/parameters/0/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "an INSERT keeps its target",
            sql: "INSERT INTO s.t (a, b) SELECT x, y FROM u",
            pointer: "/0/insert/table/schema/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "an INSERT keeps its column list",
            sql: "INSERT INTO s.t (a, b) SELECT x, y FROM u",
            pointer: "/0/insert/columns/1/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "an INSERT keeps the source of its query",
            sql: "INSERT INTO s.t (a, b) SELECT x, y FROM u",
            pointer: "/0/insert/query/select/from/expressions/0/table/name/name",
            expected_json: "\"u\"",
        },
        ValueTestCase {
            description: "an UPDATE keeps its SET column",
            sql: "UPDATE t AS x SET a = u.b FROM u WHERE x.k = u.k",
            pointer: "/0/update/set/0/0/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "an UPDATE keeps its target alias",
            sql: "UPDATE t AS x SET a = u.b FROM u WHERE x.k = u.k",
            pointer: "/0/update/table/alias/name",
            expected_json: "\"x\"",
        },
        ValueTestCase {
            description: "an UPDATE keeps its FROM source",
            sql: "UPDATE t AS x SET a = u.b FROM u WHERE x.k = u.k",
            pointer: "/0/update/from_clause/expressions/0/table/name/name",
            expected_json: "\"u\"",
        },
        ValueTestCase {
            description: "a CREATE TABLE keeps its columns",
            sql: "CREATE TABLE s.t (a INT NOT NULL, b VARCHAR, PRIMARY KEY (a))",
            pointer: "/0/create_table/columns/1/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "a CREATE TABLE AS keeps the source of its query",
            sql: "CREATE OR REPLACE TABLE t AS SELECT x FROM u",
            pointer: "/0/create_table/as_select/select/from/expressions/0/table/name/name",
            expected_json: "\"u\"",
        },
        ValueTestCase {
            description: "a CREATE VIEW keeps its column names",
            sql: "CREATE VIEW s.v (a) AS SELECT x FROM u",
            pointer: "/0/create_view/columns/0/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "a CREATE VIEW keeps the source of its query",
            sql: "CREATE VIEW s.v (a) AS SELECT x FROM u",
            pointer: "/0/create_view/query/select/from/expressions/0/table/name/name",
            expected_json: "\"u\"",
        },
        ValueTestCase {
            description: "a DROP TABLE keeps its schema",
            sql: "DROP TABLE IF EXISTS s.t",
            pointer: "/0/drop_table/names/0/schema/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "a CREATE INDEX keeps its table",
            sql: "CREATE UNIQUE INDEX i ON s.t (a DESC)",
            pointer: "/0/create_index/table/schema/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "a CREATE SEQUENCE keeps its folded start",
            sql: "CREATE SEQUENCE s START WITH (-5) MINVALUE -10",
            pointer: "/0/create_sequence/start",
            expected_json: "-5",
        },
        ValueTestCase {
            description: "a DROP FUNCTION keeps its schema",
            sql: "DROP MACRO IF EXISTS s.f",
            pointer: "/0/drop_function/name/schema/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "an ALTER TABLE keeps the column it adds",
            sql: "ALTER TABLE s.t ADD COLUMN c VARCHAR",
            pointer: "/0/alter_table/actions/0/AddColumn/column/name/name",
            expected_json: "\"c\"",
        },
        ValueTestCase {
            description: "an ALTER VIEW keeps its new name",
            sql: "ALTER VIEW s.v RENAME TO w",
            pointer: "/0/alter_view/actions/0/Rename/name/name",
            expected_json: "\"w\"",
        },
        ValueTestCase {
            description: "a PIVOT statement keeps the values it pivots on",
            sql: "PIVOT t ON status IN ('placed') USING MAX(amount) GROUP BY id",
            pointer: "/0/pivot/expressions/0/in/expressions/0/literal/value",
            expected_json: "\"placed\"",
        },
        ValueTestCase {
            description: "a PIVOT statement keeps its GROUP BY columns",
            sql: "PIVOT t ON status USING MAX(amount) GROUP BY id",
            pointer: "/0/pivot/group/tuple/expressions/0/column/name/name",
            expected_json: "\"id\"",
        },
        ValueTestCase {
            description: "an UNPIVOT statement keeps the column its names go into",
            sql: "UNPIVOT t ON a, b INTO NAME k VALUE v",
            pointer: "/0/pivot/into/unpivot_columns/this/column/name/name",
            expected_json: "\"k\"",
        },
        ValueTestCase {
            description: "a trailing comma ends the FROM list",
            sql: "SELECT * FROM a, b,",
            pointer: "/0/select/from/expressions/1/table/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "a DESCRIBE keeps the table it describes",
            sql: "DESCRIBE s.t",
            pointer: "/0/describe/target/table/schema/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "a SUMMARIZE keeps the query it summarizes and its style",
            sql: "SUMMARIZE SELECT 1 AS a",
            pointer: "/0/describe/style",
            expected_json: "\"SUMMARIZE\"",
        },
        ValueTestCase {
            description: "a MERGE keeps its target's alias",
            sql: "MERGE INTO t AS x USING s ON x.k = s.k WHEN MATCHED THEN DELETE",
            pointer: "/0/merge/this/table/alias/name",
            expected_json: "\"x\"",
        },
        ValueTestCase {
            description: "a MERGE keeps a NOT MATCHED BY SOURCE clause's kind",
            sql: "MERGE INTO t USING s ON 1 WHEN NOT MATCHED BY SOURCE THEN DELETE",
            pointer: "/0/merge/whens/whens/expressions/0/when/source/boolean/value",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a MERGE keeps the columns of its USING qualifier",
            sql: "MERGE INTO t USING s USING (k) WHEN MATCHED THEN DELETE",
            pointer: "/0/merge/using_cond/tuple/expressions/0/column/name/name",
            expected_json: "\"k\"",
        },
        ValueTestCase {
            description: "an ATTACH keeps its alias",
            sql: "ATTACH 'f.duckdb' AS shop",
            pointer: "/0/attach/this/alias/alias/name",
            expected_json: "\"shop\"",
        },
        ValueTestCase {
            description: "an ATTACH keeps its options' values",
            sql: "ATTACH 'f.duckdb' AS shop (TYPE sqlite, READ_ONLY)",
            pointer: "/0/attach/expressions/0/attach_option/expression/column/name/name",
            expected_json: "\"sqlite\"",
        },
        ValueTestCase {
            description: "a SET keeps its scope",
            sql: "SET GLOBAL threads = 1",
            pointer: "/0/set_statement/items/0/kind",
            expected_json: "\"GLOBAL\"",
        },
        ValueTestCase {
            description: "a COMMENT ON keeps its kind",
            sql: "COMMENT ON MACRO TABLE m IS 'x'",
            pointer: "/0/comment/kind",
            expected_json: "\"MACRO TABLE\"",
        },
        ValueTestCase {
            description: "a DELETE keeps its USING table",
            sql: "DELETE FROM t USING u WHERE t.k = u.k",
            pointer: "/0/delete/using/0/name/name",
            expected_json: "\"u\"",
        },
        ValueTestCase {
            description: "a table option keeps its value as written",
            sql: "CREATE TABLE z WITH (FORMAT = 'parquet') AS SELECT 1",
            pointer: "/0/create_table/with_properties/0/1",
            expected_json: "\"'parquet'\"",
        },
        ValueTestCase {
            description: "ALTER TABLE RESET keeps the option it resets",
            sql: "ALTER TABLE t RESET (a.b)",
            pointer: "/0/alter_table/actions/0/UnsetProperty/properties/0",
            expected_json: "\"a.b\"",
        },
        ValueTestCase {
            description: "a nested schema path keeps each catalog part with its quoting",
            sql: "SELECT * FROM m.\"a.b\".c.t",
            pointer: "/0/select/from/expressions/0/table/catalog_parts/1/quoted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a table function keeps WITH ORDINALITY",
            sql: "SELECT * FROM range(3) WITH ORDINALITY",
            pointer: "/0/select/from/expressions/0/lateral/ordinality/boolean/value",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a set operation keeps a percentage limit",
            sql: "SELECT 1 UNION SELECT 2 LIMIT 10%",
            pointer: "/0/union/limit/limit/percent",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a set operation's FETCH without a count limits it to one row",
            sql: "SELECT 1 UNION SELECT 2 FETCH FIRST ROW ONLY",
            pointer: "/0/union/limit/literal/value",
            expected_json: "\"1\"",
        },
        ValueTestCase {
            description: "a pivot on an enum's values marks the name as a field",
            sql: "PIVOT t1 ON c1 IN e1",
            pointer: "/0/pivot/expressions/0/in/is_field",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a pivot ON entry with an operator before IN is one boolean expression",
            sql: "PIVOT t1 ON c1 + 1 IN (2)",
            pointer: "/0/pivot/expressions/0/paren/this/in/expressions/0/literal/value",
            expected_json: "\"2\"",
        },
        ValueTestCase {
            description: "a sample after a subquery samples that subquery",
            sql: "FROM (SELECT 1) TABLESAMPLE 10%",
            pointer: "/0/select/from/expressions/0/table_sample/sample/is_percent",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a NATURAL SEMI join keeps its kind",
            sql: "SELECT * FROM t1 NATURAL SEMI JOIN t2",
            pointer: "/0/select/joins/0/kind",
            expected_json: "\"NaturalSemi\"",
        },
        ValueTestCase {
            description: "an ASOF FULL join keeps its kind",
            sql: "SELECT * FROM t1 ASOF FULL JOIN t2 ON t1.a >= t2.a",
            pointer: "/0/select/joins/0/kind",
            expected_json: "\"AsOfFull\"",
        },
        ValueTestCase {
            description: "a qualified EXCLUDE name keeps its parts and their quoting",
            sql: "SELECT * EXCLUDE (\"x.y\".a) FROM t1",
            pointer: "/0/select/expressions/0/star/qualified/except/0/0/quoted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a qualified RENAME source keeps its parts and their quoting",
            sql: "SELECT * RENAME (x.\"y.a\" AS b) FROM t1",
            pointer: "/0/select/expressions/0/star/qualified/rename/0/1/name",
            expected_json: "\"y.a\"",
        },
        ValueTestCase {
            description: "a USING KEY entry that is no bare column is kept as an expression",
            sql: "WITH RECURSIVE r(a, b) USING KEY (a, avg(b)) AS (SELECT 1, 5) FROM r",
            pointer: "/0/select/with/ctes/0/key_items/1/function/name",
            expected_json: "\"avg\"",
        },
        ValueTestCase {
            description: "dropping a struct field keeps its path",
            sql: "ALTER TABLE t DROP COLUMN s.i",
            pointer: "/0/alter_table/actions/0/DropField/names/1/name",
            expected_json: "\"i\"",
        },
        ValueTestCase {
            description: "renaming a struct field keeps its path",
            sql: "ALTER TABLE t RENAME s.j TO v1",
            pointer: "/0/alter_table/actions/0/RenameField/names/1/name",
            expected_json: "\"j\"",
        },
        ValueTestCase {
            description: "RESET VARIABLE keeps VARIABLE as its scope",
            sql: "RESET VARIABLE x",
            pointer: "/0/reset_statement/scope",
            expected_json: "\"VARIABLE\"",
        },
        ValueTestCase {
            description: "RESET keeps the setting it resets",
            sql: "RESET SESSION threads",
            pointer: "/0/reset_statement/name/identifier/name",
            expected_json: "\"threads\"",
        },
        ValueTestCase {
            description: "INSTALL keeps a repository URL as a string",
            sql: "INSTALL x FROM 'https://x'",
            pointer: "/0/install/from_/literal/value",
            expected_json: "\"https://x\"",
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
            sql: "SELECT 1; CONNECT c1",
            expected_outcome: Ok((2, 1)),
        },
        StrictTestCase {
            description: "a syntax error refuses the whole input",
            sql: "SELECT 1; SELECT (",
            expected_outcome: Err(Some(true)),
        },
        StrictTestCase {
            description: "a transformer check refuses it too",
            sql: "SELECT 1; SELECT IFNULL(1)",
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
        description: "a CONNECT is a gap from its first byte to the end of the input",
        sql: "SELECT 1; CONNECT c1",
        expected_gaps: &[Gap {
            construct: "connect",
            start: 10,
            end: 20,
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
fn given_deep_nesting_when_parsing_on_a_small_stack_then_it_is_refused_cleanly_and_quickly() {
    let too_deep: Option<&str> = Some("the statement nests too deeply");
    let chain: Option<&str> = Some("Max expression depth limit of 1000 exceeded");
    let test_cases = [
        NestingTestCase {
            description: "1,500 parentheses around an expression",
            sql: || format!("SELECT {}1{}", "(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 parentheses around a table in FROM",
            sql: || format!("SELECT * FROM {}t1{}", "(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 subqueries nested in FROM",
            sql: || {
                let open: String = "(SELECT * FROM ".repeat(1500);
                format!("SELECT * FROM {open}t1{}", ")".repeat(1500))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 scalar subqueries",
            sql: || format!("SELECT {}1{}", "(SELECT ".repeat(1500), ")".repeat(1500)),
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
            description: "1,500 nested STRUCT types",
            sql: || {
                let open: String = "STRUCT(a ".repeat(1500);
                format!("SELECT CAST(NULL AS {open}INTEGER{})", ")".repeat(1500))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 nested MAP types",
            sql: || {
                let open: String = "MAP(INTEGER, ".repeat(1500);
                format!("SELECT CAST(NULL AS {open}INTEGER{})", ")".repeat(1500))
            },
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "1,500 nested calls",
            sql: || format!("SELECT {}1{}", "abs(".repeat(1500), ")".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "a list type with 1,500 bounds",
            sql: || format!("SELECT CAST(NULL AS INTEGER{})", "[]".repeat(1500)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "3 STRUCT levels each wrapping 60 bounds are 184 type levels",
            sql: || nested_struct_lists(3, 60),
            expected_refusal: None,
        },
        NestingTestCase {
            description: "4 such levels are 245, so constructors and bounds count together",
            sql: || nested_struct_lists(4, 60),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "999 additions are DuckDB's deepest chain",
            sql: || format!("SELECT 1{}", " + 1".repeat(999)),
            expected_refusal: None,
        },
        NestingTestCase {
            description: "1,000 additions pass it",
            sql: || format!("SELECT 1{}", " + 1".repeat(1000)),
            expected_refusal: chain,
        },
        NestingTestCase {
            description: "1,000 concatenations pass it",
            sql: || format!("SELECT 'a'{}", " || 'a'".repeat(1000)),
            expected_refusal: chain,
        },
        NestingTestCase {
            description: "AND is one conjunction to DuckDB with no limit of its own, so 5,000 ANDs parse",
            sql: || format!("SELECT true{}", " AND true".repeat(5000)),
            expected_refusal: None,
        },
        NestingTestCase {
            description: "10,000 ORs nest deeper than the tree may",
            sql: || format!("SELECT true{}", " OR true".repeat(10_000)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "5,000 UNION ALL operands parse",
            sql: || format!("SELECT 1{}", " UNION ALL SELECT 1".repeat(5000)),
            expected_refusal: None,
        },
        NestingTestCase {
            description: "10,000 INTERSECT operands nest deeper than the tree may",
            sql: || format!("SELECT 1{}", " INTERSECT SELECT 1".repeat(10_000)),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "60 nested queries of 200 UNION ALL links each nest too deeply as a tree",
            sql: || nested_unions(60, 200),
            expected_refusal: too_deep,
        },
        NestingTestCase {
            description: "60 nested TRIM calls parse without retrying each inner call",
            sql: || format!("SELECT {}'a'{}", "trim(".repeat(60), ")".repeat(60)),
            expected_refusal: None,
        },
        NestingTestCase {
            description: "60 nested calls parse without trying each as a typed literal",
            sql: || format!("SELECT {}1{}", "abs(".repeat(60), ")".repeat(60)),
            expected_refusal: None,
        },
        NestingTestCase {
            description: "past the limit, every other reading fails at once instead of retrying",
            sql: || {
                let open: String = "(SELECT 1 IN ".repeat(150);
                format!("SELECT 1 IN {open}(1){}", ")".repeat(150))
            },
            expected_refusal: too_deep,
        },
    ];
    for test_case in test_cases {
        let sql: String = (test_case.sql)();
        let refusal: Option<String> = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || {
                parse(&sql)
                    .map(|parsed| {
                        serde_json::to_string(&parsed.statements).expect("the tree serialises")
                    })
                    .err()
                    .map(|error| error.message)
            })
            .expect("the parse thread starts")
            .join()
            .expect("the parse thread finishes");
        assert_eq!(
            refusal.as_deref().map(|message| test_case
                .expected_refusal
                .is_some_and(|prefix| message.starts_with(prefix))),
            test_case.expected_refusal.map(|_| true),
            "{}: {refusal:?}",
            test_case.description
        );
    }
}

#[test]
fn given_strict_failures_when_matched_then_only_parse_errors_carry_one() {
    let test_cases = [
        FailureTestCase {
            description: "an uncovered byte carries no parse error",
            failure: StrictFailure::Uncovered { position: 3 },
            expected_syntax: None,
        },
        FailureTestCase {
            description: "a refused input carries its syntax error",
            failure: parse_strict("SELECT (").expect_err("refused"),
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
fn given_the_duckdb_core_corpus_when_parsing_then_only_the_connect_by_pair_fails() {
    let test_cases = [CorpusTestCase {
        description: "every DuckShop statement parses strictly but the CONNECT BY pair",
        corpus: "../../corpora/duckdb-core/duckshop.jsonl",
        expected_rows: 877,
        expected_refused: &[
            "duckshop.models/intermediate/edge_cases/int_edge_category_walk",
            "duckshop.tests/unit/intermediate/edge_cases/test_int_edge_category_walk",
        ],
    }];
    for test_case in test_cases {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(test_case.corpus);
        let text: String = std::fs::read_to_string(path).expect("the corpus is committed");
        let rows: Vec<Value> = text
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("one JSON object per line"))
            .collect();
        let refused: BTreeSet<&str> = rows
            .iter()
            .filter(|row| parse_strict(row["sql"].as_str().expect("a sql field")).is_err())
            .map(|row| row["id"].as_str().expect("an id field"))
            .collect();
        let expected_refused: BTreeSet<&str> = test_case.expected_refused.iter().copied().collect();
        assert_eq!(
            rows.len(),
            test_case.expected_rows,
            "{}",
            test_case.description
        );
        assert_eq!(refused, expected_refused, "{}", test_case.description);
    }
}

#[test]
fn given_the_recorded_extras_when_parsing_then_every_verdict_outside_a_gap_is_duckdb() {
    let test_cases = [RecordedCorpusTestCase {
        description: "the DuckDB inputs of sqlglot's and polyglot's tests, as DuckDB 2.0.0 judged them",
        corpus: "../../corpora/duckdb-core/extras.jsonl",
        expected_rows: 4133,
        expected_gapped: 0,
    }];
    for test_case in test_cases {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(test_case.corpus);
        let text: String = std::fs::read_to_string(path).expect("the corpus is committed");
        let rows: Vec<Value> = text
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("one JSON object per line"))
            .collect();
        let gapped: usize = rows.iter().filter(|row| reaches_gap(row)).count();
        let disagreements: Vec<&str> = rows
            .iter()
            .filter(|row| !reaches_gap(row) && !agrees_with_duckdb(row))
            .map(|row| row["id"].as_str().expect("an id field"))
            .collect();
        assert_eq!(
            rows.len(),
            test_case.expected_rows,
            "{}",
            test_case.description
        );
        assert_eq!(
            disagreements,
            Vec::<String>::new(),
            "{}",
            test_case.description
        );
        assert_eq!(
            gapped, test_case.expected_gapped,
            "{}",
            test_case.description
        );
    }
}
