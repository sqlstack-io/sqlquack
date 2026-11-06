//! Verdicts and trees of the PostgreSQL parser, each confirmed against PostgreSQL 18.6.

use serde_json::Value;

use crate::parse::main::format_type::format_type;
use crate::parse::main::parse::parse;
use crate::parse::main::parse_strict::parse_strict;
use crate::parse::main::parse_type::parse_type;
use crate::parse::tests::test_types::{
    ExpectedVerdict, ShapeTestCase, SpanTestCase, StrictTestCase, TypeTextTestCase, ValueTestCase,
    VerdictTestCase,
};

#[test]
fn given_sql_when_parsing_then_the_verdict_is_postgresqls() {
    let test_cases = [
        VerdictTestCase {
            description: "ALTER TYPE's attribute commands take at most three name parts",
            sql: "ALTER TYPE a.b.c.d ADD ATTRIBUTE x int",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER TYPE ADD VALUE keeps any_name, so four parts are accepted",
            sql: "ALTER TYPE a.b.c.d ADD VALUE 'x'",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER TEXT SEARCH needs its object kind",
            sql: "ALTER TEXT SEARCH t1 RENAME TO v1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER TYPE DROP VALUE is refused, as gram.y refuses it",
            sql: "ALTER TYPE t DROP VALUE 'a'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ALTER DOMAIN ADD NOT NULL takes no NOT VALID",
            sql: "ALTER DOMAIN d ADD NOT NULL NOT VALID",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a publication list cannot start with a bare name",
            sql: "ALTER PUBLICATION p ADD a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a schema in a publication list takes no row filter",
            sql: "ALTER PUBLICATION p ADD TABLES IN SCHEMA s, t WHERE (x)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER FOREIGN DATA WRAPPER needs an option",
            sql: "ALTER FOREIGN DATA WRAPPER w",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER OPERATOR FAMILY ADD is structured, not opaque",
            sql: "ALTER OPERATOR FAMILY f USING btree ADD OPERATOR 1 < (int, int), FUNCTION 1 f(int, int)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER FUNCTION takes common_func_opt_item, not LANGUAGE",
            sql: "ALTER FUNCTION f LANGUAGE sql",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER FUNCTION needs at least one option",
            sql: "ALTER FUNCTION f RESTRICT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET CATALOG is refused, as gram.y refuses it",
            sql: "ALTER FUNCTION f SET CATALOG 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a time zone interval may name only HOUR and MINUTE",
            sql: "ALTER FUNCTION f SET TIME ZONE INTERVAL '1' DAY",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER AGGREGATE takes no function options",
            sql: "ALTER AGGREGATE a(*) IMMUTABLE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SET TIME ZONE in a routine is structured, not opaque",
            sql: "CREATE FUNCTION f() RETURNS int SET TIME ZONE 'UTC' RETURN 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER with a word no statement kind allows is refused",
            sql: "ALTER FOO t RENAME TO u",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER TABLE takes at most three name parts, as qualified_name does",
            sql: "ALTER TABLE a.b.c.d ADD COLUMN x int",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER SEQUENCE NO CYCLE is a SeqOptList",
            sql: "ALTER SEQUENCE s NO CYCLE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ordered-set VARIADIC types compare after folding, as equal() does",
            sql: "CREATE AGGREGATE a (VARIADIC \"foo\" ORDER BY VARIADIC foo) (sfunc = f, stype = int)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ordered-set VARIADIC types compare their array bounds",
            sql: "CREATE AGGREGATE a (VARIADIC int4[] ORDER BY VARIADIC int4[3]) (sfunc = f, stype = int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an old-style aggregate element name is an IDENT, not a keyword",
            sql: "CREATE AGGREGATE a (sfunc = f, parallel = safe)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a database option names a keyword PostgreSQL lists, or an identifier",
            sql: "CREATE DATABASE d LIMIT 5",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a database option takes a value",
            sql: "CREATE DATABASE d OWNER",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an aggregate argument cannot be OUT (0A000)",
            sql: "CREATE AGGREGATE a (OUT int) (sfunc = f, stype = int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a VARIADIC direct argument needs one VARIADIC aggregated argument of the same raw type (0A000)",
            sql: "CREATE AGGREGATE a (VARIADIC int ORDER BY VARIADIC int4) (sfunc = f, stype = int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "INT and INTEGER are one raw type name, as equal() reads them",
            sql: "CREATE AGGREGATE a (VARIADIC int ORDER BY VARIADIC integer) (sfunc = f, stype = int)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "INT and pg_catalog.int4 are one raw type name",
            sql: "CREATE AGGREGATE a (VARIADIC int ORDER BY VARIADIC pg_catalog.int4) (sfunc = f, stype = int)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "the old-style definition needs = after each name",
            sql: "CREATE AGGREGATE a (sfunc = f, stype)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "OptTemp is one word or pair, so UNLOGGED TEMP is refused",
            sql: "CREATE UNLOGGED TEMP TABLE t (a int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a TEMP sequence is parsed",
            sql: "CREATE TEMP SEQUENCE s",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a role option gram.y does not know is refused (42601)",
            sql: "CREATE ROLE r foo",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "UNENCRYPTED PASSWORD is refused in gram.y (0A000)",
            sql: "CREATE ROLE r UNENCRYPTED PASSWORD 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "CURRENT_USER cannot name a new role (42939)",
            sql: "CREATE ROLE current_user",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "RENAME takes a RoleId, so not CURRENT_USER (42939)",
            sql: "ALTER ROLE current_user RENAME TO x",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "ALTER SYSTEM takes generic_set, which has no FROM CURRENT",
            sql: "ALTER SYSTEM SET x FROM CURRENT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER GROUP names one role, not ALL",
            sql: "ALTER GROUP ALL ADD USER a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER POLICY IF EXISTS only renames",
            sql: "ALTER POLICY IF EXISTS p ON t TO a",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a role option needs OPTION, TRUE or FALSE",
            sql: "GRANT r TO u WITH ADMIN",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a GRANT inside CREATE SCHEMA grants on objects only",
            sql: "CREATE SCHEMA s GRANT r TO u",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ALTER DEFAULT PRIVILEGES takes no GRANTED BY",
            sql: "ALTER DEFAULT PRIVILEGES GRANT SELECT ON TABLES TO r GRANTED BY s",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a policy is PERMISSIVE or RESTRICTIVE, compared after folding (42601)",
            sql: "CREATE POLICY p ON t AS \"RESTRICTIVE\"",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CREATE EXTENSION ... FROM is refused in gram.y (0A000)",
            sql: "CREATE EXTENSION e FROM x",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "CREATE SCHEMA IF NOT EXISTS takes no schema elements (0A000)",
            sql: "CREATE SCHEMA IF NOT EXISTS s CREATE TABLE t (a int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a schema element cannot be CREATE TABLE AS",
            sql: "CREATE SCHEMA s CREATE TABLE t AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a rule action cannot be MERGE",
            sql: "CREATE RULE r AS ON INSERT TO t DO (SELECT 1; MERGE INTO t USING s ON true WHEN MATCHED THEN DELETE)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a transform names FROM SQL and TO SQL at most once each",
            sql: "CREATE TRANSFORM FOR int LANGUAGE l (FROM SQL WITH FUNCTION f, FROM SQL WITH FUNCTION g)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "OR REPLACE is refused where gram.y has none",
            sql: "CREATE OR REPLACE CAST (int AS text) WITH INOUT",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a composite type's name has at most three parts, as makeRangeVarFromAnyName reads it",
            sql: "CREATE TYPE a.b.c.d AS (x int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an enum type keeps any_name, so four parts are accepted",
            sql: "CREATE TYPE a.b.c.d AS ENUM ('x')",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a recursive view refuses WITH CHECK OPTION in gram.y (0A000)",
            sql: "CREATE RECURSIVE VIEW v (a) AS SELECT 1 WITH CHECK OPTION",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "CREATE ASSERTION parses, then is refused (0A000)",
            sql: "CREATE ASSERTION a CHECK (true)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a CREATE that stops after a word no statement allows is refused",
            sql: "CREATE UNIQUE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a trigger function argument cannot be an expression",
            sql: "CREATE TRIGGER g AFTER INSERT ON t EXECUTE FUNCTION f(1 + 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a foreign table needs a SERVER",
            sql: "CREATE FOREIGN TABLE f (a int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COLLATE twice on one column is refused (42601)",
            sql: "CREATE TABLE t (a int COLLATE \"C\" COLLATE \"C\")",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a generated column must be GENERATED ALWAYS (42601)",
            sql: "CREATE TABLE t (a int GENERATED BY DEFAULT AS (1 + 2))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a CHECK constraint cannot be DEFERRABLE (0A000)",
            sql: "CREATE TABLE t (CHECK (a > 0) DEFERRABLE)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a UNIQUE constraint cannot be NOT VALID (0A000)",
            sql: "CREATE TABLE t (UNIQUE (a) NOT VALID)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "NOT DEFERRABLE conflicts with INITIALLY DEFERRED (42601)",
            sql: "CREATE TABLE t (CONSTRAINT x PRIMARY KEY (a) NOT DEFERRABLE INITIALLY DEFERRED)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ON UPDATE takes no SET NULL column list (0A000)",
            sql: "CREATE TABLE t (a int REFERENCES p ON UPDATE SET NULL (a))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "MATCH PARTIAL is not implemented (0A000)",
            sql: "CREATE TABLE t (a int REFERENCES p MATCH PARTIAL)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "an unknown partitioning strategy (22023)",
            sql: "CREATE TABLE t (a int) PARTITION BY foo (a)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a partition key is a column, a call or a parenthesized expression",
            sql: "CREATE TABLE t (a int) PARTITION BY RANGE (1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a hash bound needs a remainder (42601)",
            sql: "CREATE TABLE t PARTITION OF p FOR VALUES WITH (MODULUS 4)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a hash bound names its modulus once (42710)",
            sql: "CREATE TABLE t PARTITION OF p FOR VALUES WITH (MODULUS 4, MODULUS 3, REMAINDER 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "EXCLUDE and PERIOD as column names",
            sql: "CREATE TABLE t (exclude int, period int)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "IF as a table name",
            sql: "CREATE TABLE if (a int)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE AS EXECUTE with parameters",
            sql: "CREATE TABLE t AS EXECUTE q (1, 2) WITH DATA",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CREATE TABLE AS takes column names, not definitions",
            sql: "CREATE TABLE t (a int) AS SELECT 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "LOCAL needs TEMP or TEMPORARY",
            sql: "CREATE LOCAL TABLE t (a int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a table name has at most three parts (42601)",
            sql: "CREATE TABLE a.b.c.d (a int)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SECURITY LABEL names no CAST, which only COMMENT names",
            sql: "SECURITY LABEL ON CAST (int AS text) IS 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SECURITY needs LABEL, its only statement",
            sql: "SECURITY ON TABLE t IS 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an operator with one argument type needs NONE (42601 missing argument)",
            sql: "COMMENT ON OPERATOR + (int) IS 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an operator has at most one NONE side",
            sql: "COMMENT ON OPERATOR + (NONE, NONE) IS 'x'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "CREATE needs an object kind",
            sql: "CREATE t1 ( ) LANGUAGE plpgsql",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unknown table, which only analysis rejects",
            sql: "SELECT a, b FROM missing_t WHERE a > 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a keyword usable as a bare label",
            sql: "SELECT 1 like FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a generic type name before a string, as a typed literal",
            sql: "SELECT a 'x' FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a column-name keyword as a column",
            sql: "SELECT time FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a reserved keyword after AS",
            sql: "SELECT 1 AS from",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "IS NULL repeated",
            sql: "SELECT a IS NULL IS NULL",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "array types after :: and ARRAY with a bound",
            sql: "SELECT x::int[], '{1}'::int ARRAY[2]",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a join nested by its qualifiers",
            sql: "SELECT * FROM a JOIN b JOIN c ON true ON true",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a value function called with parentheses",
            sql: "SELECT current_schema()",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "WITH before a set operation with ORDER BY and LIMIT",
            sql: "WITH r AS MATERIALIZED (SELECT 1) SELECT * FROM r UNION SELECT 2 ORDER BY 1 LIMIT 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "INSERT with an alias, ON CONFLICT and RETURNING",
            sql: "INSERT INTO t AS x (a) VALUES (1) ON CONFLICT (a) DO UPDATE SET a = excluded.a RETURNING *",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "UPDATE with FROM",
            sql: "UPDATE t SET a = 1 FROM s WHERE t.id = s.id",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "DELETE with an alias and USING",
            sql: "DELETE FROM t u USING s WHERE u.id = s.id",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a semicolon inside a dollar quote",
            sql: "SELECT $$a;b$$; SELECT 2",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "OFFSET and FETCH in their standard spellings",
            sql: "SELECT 1 OFFSET 2 ROWS FETCH FIRST ROW ONLY",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "named arguments in both spellings",
            sql: "SELECT f(a => 1, b := 2)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "an aggregate with FILTER and a framed window",
            sql: "SELECT count(*) FILTER (WHERE x) OVER (PARTITION BY a ROWS BETWEEN 1 PRECEDING AND CURRENT ROW EXCLUDE TIES) FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "five dotted names as a field selection",
            sql: "SELECT a.b.c.d.e FROM t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "ALTER TABLE ADD COLUMN, as a structured AlterObject",
            sql: "ALTER TABLE t ADD COLUMN a int",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "MERGE_ACTION(), which the parser keeps opaque",
            sql: "MERGE INTO t USING s ON true WHEN MATCHED THEN DELETE RETURNING merge_action()",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "a comparison chained",
            sql: "SELECT a = b = c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "LIKE chained",
            sql: "SELECT a like b like c",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a type-or-function keyword as a column",
            sql: "SELECT left FROM t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a keyword that is not a bare label",
            sql: "SELECT 1 year",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a field of a call's result without parentheses",
            sql: "SELECT f(1).x",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a subscript on a constant",
            sql: "SELECT 'x'[1]",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "ARRAY with empty brackets",
            sql: "SELECT CAST(NULL AS INT ARRAY [ ])",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "WITH TIES without ORDER BY",
            sql: "SELECT 1 FETCH FIRST ROWS WITH TIES",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "LIMIT with a comma",
            sql: "SELECT 1 LIMIT 1, 2",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a parenthesized table that is not a join",
            sql: "SELECT * FROM (t)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a statement starting with FROM",
            sql: "FROM ONLY t1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "AT TIME without ZONE",
            sql: "SELECT x at time",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a second ORDER BY around a parenthesized query",
            sql: "(SELECT 1 ORDER BY 1) ORDER BY 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a table name with four parts",
            sql: "SELECT 1 FROM a.b.c.d",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unterminated string",
            sql: "SELECT 'abc",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "the UNIQUE predicate, refused as unsupported",
            sql: "SELECT UNIQUE (SELECT 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a malformed Unicode escape in an escape string",
            sql: "SELECT E'\\u12'",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "COPY options in parentheses with a star argument",
            sql: "COPY t TO STDOUT WITH (FORMAT csv, FORCE_QUOTE *)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "old-style COPY options with a bare column list",
            sql: "COPY t (a, b) FROM STDIN WITH CSV HEADER FORCE NOT NULL a, b",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "COPY FROM PROGRAM with STDIN",
            sql: "COPY t FROM PROGRAM STDIN",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a number inside a COPY option's list",
            sql: "COPY t FROM 'f' (a (1))",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COPY of a query FROM a file",
            sql: "COPY (SELECT 1) FROM STDIN",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "COPY TO with a WHERE clause",
            sql: "COPY t TO 'f' WHERE a > 1",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "USING without DELIMITERS in COPY",
            sql: "COPY t FROM 'f' USING ','",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "TRUNCATE of several relations with identity and drop behaviour",
            sql: "TRUNCATE TABLE ONLY a.b, c * RESTART IDENTITY CASCADE",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "CONTINUE without IDENTITY in TRUNCATE",
            sql: "TRUNCATE t CONTINUE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a MERGE update when the source has no matching row",
            sql: "MERGE INTO t USING s ON true WHEN NOT MATCHED BY SOURCE THEN UPDATE SET a = 1",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a MERGE insert for a matched row",
            sql: "MERGE INTO t USING s ON true WHEN MATCHED THEN INSERT VALUES (1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "a MERGE delete when the target has no matching row",
            sql: "MERGE INTO t USING s ON true WHEN NOT MATCHED BY TARGET THEN DELETE",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "SQL/JSON query options, which the parser checks and keeps opaque",
            sql: "SELECT json_query(js, '$' RETURNING text WITH CONDITIONAL ARRAY WRAPPER KEEP QUOTES DEFAULT 1 ON EMPTY ERROR ON ERROR)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "ON ERROR before ON EMPTY in a SQL/JSON query",
            sql: "SELECT json_query(js, '$' ERROR ON ERROR EMPTY ON EMPTY)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an operator expression before VALUE in JSON_OBJECT",
            sql: "SELECT json_object(a || b VALUE 1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "an unknown JSON encoding, refused as an invalid parameter",
            sql: "SELECT json_object('a': 1 FORMAT JSON ENCODING latin1)",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a JSON_TABLE path that is not a string constant, refused as unsupported",
            sql: "SELECT * FROM json_table(js, 1 COLUMNS (a int)) jt",
            expected_verdict: ExpectedVerdict::Rejected { syntax: false },
        },
        VerdictTestCase {
            description: "a second DEFAULT on an XMLTABLE column",
            sql: "SELECT * FROM xmltable('/r' PASSING x COLUMNS a int DEFAULT 1 DEFAULT 2) t",
            expected_verdict: ExpectedVerdict::Rejected { syntax: true },
        },
        VerdictTestCase {
            description: "XMLTABLE after LATERAL",
            sql: "SELECT * FROM LATERAL xmltable('/r' PASSING x COLUMNS a int) t",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
        },
        VerdictTestCase {
            description: "GROUP BY DISTINCT, held as the group's quantifier",
            sql: "SELECT a FROM t GROUP BY DISTINCT ROLLUP (a, b)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a row assigned across several UPDATE targets, split as PostgreSQL does",
            sql: "UPDATE t SET (a, b) = ROW(1, DEFAULT), c = 3",
            expected_verdict: ExpectedVerdict::Accepted { opaque: false },
        },
        VerdictTestCase {
            description: "a sub-SELECT assigned across several UPDATE targets, kept opaque",
            sql: "UPDATE t SET (a, b) = (SELECT 1, 2)",
            expected_verdict: ExpectedVerdict::Accepted { opaque: true },
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
fn given_sql_when_parsing_then_the_tree_follows_postgresqls_grammar() {
    let test_cases = [
        ShapeTestCase {
            description: "AND binds tighter than OR",
            sql: "SELECT a OR b AND c",
            pointer: "/0/select/expressions/0/or/right",
            expected_node: "and",
        },
        ShapeTestCase {
            description: "NOT binds looser than a comparison",
            sql: "SELECT NOT a = b",
            pointer: "/0/select/expressions/0/not/this",
            expected_node: "eq",
        },
        ShapeTestCase {
            description: "a cast binds tighter than unary minus",
            sql: "SELECT -x::int",
            pointer: "/0/select/expressions/0/neg/this",
            expected_node: "cast",
        },
        ShapeTestCase {
            description: "IS NULL applies to a whole comparison",
            sql: "SELECT a = b IS NULL",
            pointer: "/0/select/expressions/0/is_null/this",
            expected_node: "eq",
        },
        ShapeTestCase {
            description: "a keyword after an expression is its bare label",
            sql: "SELECT 1 like FROM t",
            pointer: "/0/select/expressions/0",
            expected_node: "alias",
        },
        ShapeTestCase {
            description: "a type name before a string is a typed literal",
            sql: "SELECT int4 '1'",
            pointer: "/0/select/expressions/0",
            expected_node: "cast",
        },
        ShapeTestCase {
            description: "the qualifier after a nested join belongs to the inner join",
            sql: "SELECT * FROM a JOIN b JOIN c ON true ON true",
            pointer: "/0/select/joins/0/this",
            expected_node: "joined_table",
        },
        ShapeTestCase {
            description: "INTERSECT binds tighter than UNION",
            sql: "SELECT 1 UNION SELECT 2 INTERSECT SELECT 3",
            pointer: "/0/union/right",
            expected_node: "intersect",
        },
        ShapeTestCase {
            description: "ORDER BY after a set operation belongs to the whole operation",
            sql: "SELECT 1 UNION SELECT 2 ORDER BY 1",
            pointer: "/0/union/order_by",
            expected_node: "expressions",
        },
        ShapeTestCase {
            description: "ON CONFLICT is kept on the insert",
            sql: "INSERT INTO t VALUES (1) ON CONFLICT DO NOTHING",
            pointer: "/0/insert/on_conflict",
            expected_node: "on_conflict",
        },
        ShapeTestCase {
            description: "COPY with a column list names the table and its columns",
            sql: "COPY t (a, b) FROM STDIN",
            pointer: "/0/copy/this",
            expected_node: "schema",
        },
        ShapeTestCase {
            description: "a MERGE source may be a joined table",
            sql: "MERGE INTO t USING a JOIN b ON true ON true WHEN MATCHED THEN DELETE",
            pointer: "/0/merge/using",
            expected_node: "joined_table",
        },
        ShapeTestCase {
            description: "SQLBuild's type marker is an annotation around its expression",
            sql: "SELECT __polyglot_typed(t.x, 'varchar(5)') AS v FROM t",
            pointer: "/0/select/expressions/0/alias/this/annotated",
            expected_node: "type_assertion",
        },
        ShapeTestCase {
            description: "SQLBuild's typed-like marker keeps the expression it is typed like",
            sql: "SELECT __polyglot_typed_like(x, y + 1) FROM t",
            pointer: "/0/select/expressions/0/annotated",
            expected_node: "type_like",
        },
        ShapeTestCase {
            description: "a type marker whose type text does not parse stays a call",
            sql: "SELECT __polyglot_typed(x, 'int garbage') FROM t",
            pointer: "/0/select/expressions/0",
            expected_node: "function",
        },
        ShapeTestCase {
            description: "TRUNCATE keeps every relation after the first",
            sql: "TRUNCATE a, b",
            pointer: "/0/truncate/extra_tables/0",
            expected_node: "table",
        },
    ];
    for test_case in test_cases {
        let parsed = parse(test_case.sql).expect(test_case.description);
        let tree: Value = serde_json::to_value(&parsed.statements).expect(test_case.description);
        let node: Option<&Value> = tree.pointer(test_case.pointer);
        assert!(
            node.and_then(Value::as_object)
                .is_some_and(|object| object.contains_key(test_case.expected_node)),
            "{}: {node:?}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_parsing_then_nodes_record_their_source_range_in_characters() {
    let test_cases = [
        SpanTestCase {
            description: "a column after a multi-byte character counts characters, not bytes",
            sql: "SELECT 'é', bb FROM t",
            pointer: "/0/select/expressions/1/column/span",
            expected_span: [12, 14, 1, 15],
        },
        SpanTestCase {
            description: "a cast covers its qualified operand and type, on the second line",
            sql: "SELECT\n  t.\"Ab\"::int",
            pointer: "/0/select/expressions/0/cast/span",
            expected_span: [9, 20, 2, 14],
        },
        SpanTestCase {
            description: "a quoted name includes its quotes",
            sql: "SELECT\n  t.\"Ab\"::int",
            pointer: "/0/select/expressions/0/cast/this/column/name/span",
            expected_span: [11, 15, 2, 9],
        },
        SpanTestCase {
            description: "a query covers its WITH clause and ORDER BY",
            sql: "WITH c AS (SELECT 1) SELECT 2 ORDER BY 1",
            pointer: "/0/select/span",
            expected_span: [0, 40, 1, 41],
        },
        SpanTestCase {
            description: "a qualified star's qualifier covers every part of its name",
            sql: "SELECT s.\"T\".* FROM s.\"T\"",
            pointer: "/0/select/expressions/0/star/table/span",
            expected_span: [7, 12, 1, 13],
        },
    ];
    for test_case in test_cases {
        let parsed = parse(test_case.sql).expect(test_case.description);
        let tree: Value = serde_json::to_value(&parsed.statements).expect(test_case.description);
        let span: Option<[usize; 4]> = tree.pointer(test_case.pointer).map(|span| {
            ["start", "end", "line", "column"].map(|key| {
                span[key]
                    .as_u64()
                    .and_then(|value| usize::try_from(value).ok())
                    .unwrap_or(usize::MAX)
            })
        });
        assert_eq!(
            span,
            Some(test_case.expected_span),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_type_string_when_parsed_and_formatted_then_it_reads_as_postgresqls_format_type() {
    let test_cases = [
        TypeTextTestCase {
            description: "int",
            text: "int",
            expected_name: Ok("integer"),
        },
        TypeTextTestCase {
            description: "int8",
            text: "int8",
            expected_name: Ok("bigint"),
        },
        TypeTextTestCase {
            description: "float(24) is real",
            text: "float(24)",
            expected_name: Ok("real"),
        },
        TypeTextTestCase {
            description: "float(25) is double",
            text: "float(25)",
            expected_name: Ok("double precision"),
        },
        TypeTextTestCase {
            description: "float with no bits",
            text: "float(0)",
            expected_name: Err(false),
        },
        TypeTextTestCase {
            description: "float with too many bits",
            text: "float(54)",
            expected_name: Err(false),
        },
        TypeTextTestCase {
            description: "decimal with a precision only",
            text: "decimal(10)",
            expected_name: Ok("numeric(10,0)"),
        },
        TypeTextTestCase {
            description: "numeric with spaces",
            text: "numeric(18, 2)",
            expected_name: Ok("numeric(18,2)"),
        },
        TypeTextTestCase {
            description: "bare bit is one bit",
            text: "bit",
            expected_name: Ok("bit(1)"),
        },
        TypeTextTestCase {
            description: "varbit with a length",
            text: "varbit(5)",
            expected_name: Ok("bit varying(5)"),
        },
        TypeTextTestCase {
            description: "bare char is one character",
            text: "char",
            expected_name: Ok("character(1)"),
        },
        TypeTextTestCase {
            description: "bare bpchar has no length",
            text: "bpchar",
            expected_name: Ok("bpchar"),
        },
        TypeTextTestCase {
            description: "bpchar with a length",
            text: "bpchar(3)",
            expected_name: Ok("character(3)"),
        },
        TypeTextTestCase {
            description: "national char varying",
            text: "national char varying(4)",
            expected_name: Ok("character varying(4)"),
        },
        TypeTextTestCase {
            description: "upper-case varchar",
            text: "VARCHAR(19)",
            expected_name: Ok("character varying(19)"),
        },
        TypeTextTestCase {
            description: "time with precision",
            text: "time(3)",
            expected_name: Ok("time(3) without time zone"),
        },
        TypeTextTestCase {
            description: "timetz with precision",
            text: "timetz(3)",
            expected_name: Ok("time(3) with time zone"),
        },
        TypeTextTestCase {
            description: "timestamp precision is capped",
            text: "timestamp(8)",
            expected_name: Ok("timestamp(6) without time zone"),
        },
        TypeTextTestCase {
            description: "timestamptz",
            text: "timestamptz",
            expected_name: Ok("timestamp with time zone"),
        },
        TypeTextTestCase {
            description: "an interval range",
            text: "interval day to minute",
            expected_name: Ok("interval day to minute"),
        },
        TypeTextTestCase {
            description: "an interval range ending wrongly",
            text: "interval year to day",
            expected_name: Err(true),
        },
        TypeTextTestCase {
            description: "array bounds are dropped",
            text: "int[3][4]",
            expected_name: Ok("integer[]"),
        },
        TypeTextTestCase {
            description: "ARRAY spelling",
            text: "integer array[5]",
            expected_name: Ok("integer[]"),
        },
        TypeTextTestCase {
            description: "an array of sized varchar",
            text: "varchar(10)[]",
            expected_name: Ok("character varying(10)[]"),
        },
        TypeTextTestCase {
            description: "a catalogue-qualified sized name",
            text: "pg_catalog.varchar(5)",
            expected_name: Ok("character varying(5)"),
        },
        TypeTextTestCase {
            description: "a quoted catalogue name",
            text: "\"int4\"",
            expected_name: Ok("integer"),
        },
        TypeTextTestCase {
            description: "a quoted char is the one-byte type",
            text: "\"char\"",
            expected_name: Ok("\"char\""),
        },
        TypeTextTestCase {
            description: "a type the tree has no node for",
            text: "regclass",
            expected_name: Ok("regclass"),
        },
        TypeTextTestCase {
            description: "SETOF in a type string",
            text: "setof int",
            expected_name: Err(true),
        },
        TypeTextTestCase {
            description: "trailing words",
            text: "int garbage",
            expected_name: Err(true),
        },
        TypeTextTestCase {
            description: "an unclosed modifier list",
            text: "varchar(",
            expected_name: Err(true),
        },
    ];
    for test_case in test_cases {
        let name: Result<String, bool> = parse_type(test_case.text)
            .map(|data_type| format_type(&data_type).unwrap_or_default())
            .map_err(|error| error.syntax);
        assert_eq!(
            name.as_deref().map_err(|syntax| *syntax),
            test_case.expected_name,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_parsing_strictly_then_any_parser_error_refuses_it() {
    let test_cases = [
        StrictTestCase {
            description: "a statement with comments after it",
            sql: "SELECT 1 /* c */ -- d",
            expected_outcome: Ok(1),
        },
        StrictTestCase {
            description: "only comments and semicolons",
            sql: " ; /* c */ ;",
            expected_outcome: Ok(0),
        },
        StrictTestCase {
            description: "a 42601 syntax error",
            sql: "SELECT 1 FROM",
            expected_outcome: Err(Some(true)),
        },
        StrictTestCase {
            description: "a 22023 error the grammar raises",
            sql: "SELECT 1::float(0)",
            expected_outcome: Err(Some(false)),
        },
        StrictTestCase {
            description: "an unterminated comment",
            sql: "SELECT 1 /* c",
            expected_outcome: Err(Some(true)),
        },
    ];
    for test_case in test_cases {
        let outcome: Result<usize, Option<bool>> = parse_strict(test_case.sql)
            .map(|parsed| parsed.statements.len())
            .map_err(|failure| failure.parse_error().map(|error| error.syntax));
        assert_eq!(
            outcome, test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_sql_when_parsing_then_the_tree_keeps_what_postgresqls_raw_tree_keeps() {
    let test_cases = [
        ValueTestCase {
            description: "a role option is the DefElem gram.y names it",
            sql: "CREATE ROLE r NOLOGIN",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/identifier/name",
            expected_json: "\"canlogin\"",
        },
        ValueTestCase {
            description: "ALTER GROUP ADD USER is the AlterRoleStmt that ALTER ROLE ... USER makes",
            sql: "ALTER GROUP g ADD USER a",
            pointer: "/0/alter_object/operands/0/tuple/expressions/0/identifier/name",
            expected_json: "\"rolemembers\"",
        },
        ValueTestCase {
            description: "ALTER GROUP DROP USER removes the members",
            sql: "ALTER GROUP g DROP USER a",
            pointer: "/0/alter_object/action",
            expected_json: "\"DROP\"",
        },
        ValueTestCase {
            description: "a role that is not a name is the first operand",
            sql: "ALTER ROLE ALL IN DATABASE d RESET ALL",
            pointer: "/0/alter_object/operands/0/var/this",
            expected_json: "\"ALL\"",
        },
        ValueTestCase {
            description: "a user mapping FOR USER is for CURRENT_USER",
            sql: "ALTER USER MAPPING FOR USER SERVER s OPTIONS (DROP a)",
            pointer: "/0/alter_object/operands/0/tuple/expressions/1/var/this",
            expected_json: "\"CURRENT_USER\"",
        },
        ValueTestCase {
            description: "a GRANT without a kind word grants on tables",
            sql: "GRANT SELECT ON t, u TO r",
            pointer: "/0/grant/kind",
            expected_json: "\"TABLE\"",
        },
        ValueTestCase {
            description: "ALL PRIVILEGES is no privileges, as gram.y keeps it",
            sql: "GRANT ALL PRIVILEGES ON SCHEMA s TO r",
            pointer: "/0/grant/privileges",
            expected_json: "[]",
        },
        ValueTestCase {
            description: "a role option is kept by name, not folded into grant_option",
            sql: "GRANT r TO u WITH ADMIN OPTION GRANTED BY v",
            pointer: "/0/grant/options/0/tuple/expressions/0/identifier/name",
            expected_json: "\"admin\"",
        },
        ValueTestCase {
            description: "REVOKE ADMIN OPTION FOR revokes the option, as a false DefElem",
            sql: "REVOKE ADMIN OPTION FOR r FROM u",
            pointer: "/0/revoke/options/0/tuple/expressions/1/boolean/value",
            expected_json: "false",
        },
        ValueTestCase {
            description: "default privileges on ROUTINES are on functions",
            sql: "ALTER DEFAULT PRIVILEGES GRANT EXECUTE ON ROUTINES TO r",
            pointer: "/0/alter_default_privileges/action/grant/kind",
            expected_json: "\"FUNCTIONS\"",
        },
        ValueTestCase {
            description: "a policy without TO is for PUBLIC",
            sql: "CREATE POLICY p ON t",
            pointer: "/0/create_object/operands/3/tuple/expressions/1/tuple/expressions/0/var/this",
            expected_json: "\"PUBLIC\"",
        },
        ValueTestCase {
            description: "CREATE LANGUAGE without HANDLER is the CREATE EXTENSION gram.y makes of it",
            sql: "CREATE OR REPLACE LANGUAGE l",
            pointer: "/0/create_object/if_not_exists",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a transform keeps FROM SQL before TO SQL, whichever is written first",
            sql: "CREATE TRANSFORM FOR int LANGUAGE l (TO SQL WITH FUNCTION g, FROM SQL WITH FUNCTION f)",
            pointer: "/0/create_object/operands/2/tuple/expressions/0/var/this",
            expected_json: "\"FROM SQL\"",
        },
        ValueTestCase {
            description: "CREATE SERVER stores VERSION NULL as no version",
            sql: "CREATE SERVER s VERSION NULL FOREIGN DATA WRAPPER w",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/var/this",
            expected_json: "\"FOREIGN DATA WRAPPER\"",
        },
        ValueTestCase {
            description: "an index without USING records btree, as gram.y's DEFAULT_INDEX_TYPE does",
            sql: "CREATE INDEX i ON t (a)",
            pointer: "/0/create_object/operands/1/tuple/expressions/1/identifier/name",
            expected_json: "\"btree\"",
        },
        ValueTestCase {
            description: "trigger events are kept in TRIGGER_TYPE bit order, not as written",
            sql: "CREATE TRIGGER g AFTER DELETE OR INSERT ON t EXECUTE FUNCTION f()",
            pointer: "/0/create_object/operands/1/tuple/expressions/1/var/this",
            expected_json: "\"INSERT\"",
        },
        ValueTestCase {
            description: "a numeric trigger argument becomes the string gram.y makes of it",
            sql: "CREATE TRIGGER g AFTER INSERT ON t EXECUTE FUNCTION f(1)",
            pointer: "/0/create_object/operands/3/tuple/expressions/1/function/args/0/literal/literal_type",
            expected_json: "\"string\"",
        },
        ValueTestCase {
            description: "a foreign table keeps its server",
            sql: "CREATE FOREIGN TABLE f (a int) SERVER s",
            pointer: "/0/create_table/properties/0/tuple/expressions/1/identifier/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "a bare name after TABLE continues the TABLE list, as preprocess_pubobj_list reads it",
            sql: "ALTER PUBLICATION p ADD TABLE a, b",
            pointer: "/0/alter_object/operands/1/tuple/expressions/0/var/this",
            expected_json: "\"TABLE\"",
        },
        ValueTestCase {
            description: "operator_def_list's NONE has no argument, as a bare name has none",
            sql: "ALTER TYPE t SET (a = none)",
            pointer: "/0/alter_object/operands/0/tuple/expressions",
            expected_json: "[{\"identifier\":{\"name\":\"a\",\"quoted\":false,\"trailing_comments\":[]}}]",
        },
        ValueTestCase {
            description: "SET TRANSACTION in ALTER DATABASE names transaction_read_only",
            sql: "ALTER DATABASE d SET TRANSACTION READ ONLY",
            pointer: "/0/alter_object/operands/0/tuple/expressions/1/tuple/expressions/0/identifier/name",
            expected_json: "\"transaction_read_only\"",
        },
        ValueTestCase {
            description: "an ordered-set aggregate's ALTER keeps its name parts",
            sql: "ALTER AGGREGATE s.a(int ORDER BY int) RENAME TO b",
            pointer: "/0/alter_object/name/schema/name",
            expected_json: "\"s\"",
        },
        ValueTestCase {
            description: "ALTER OPERATOR CLASS keeps its access method",
            sql: "ALTER OPERATOR CLASS c USING btree RENAME TO d",
            pointer: "/0/alter_object/operands/0/tuple/expressions/1/identifier/name",
            expected_json: "\"btree\"",
        },
        ValueTestCase {
            description: "a four-part ALTER FUNCTION name keeps its first part",
            sql: "ALTER FUNCTION a.b.c.d (int) IMMUTABLE",
            pointer: "/0/alter_object/name/catalog/name",
            expected_json: "\"a.b\"",
        },
        ValueTestCase {
            description: "ALTER FUNCTION f with no argument list keeps no call",
            sql: "ALTER FUNCTION f IMMUTABLE",
            pointer: "/0/alter_object/operands/0/identifier/name",
            expected_json: "\"f\"",
        },
        ValueTestCase {
            description: "SET TIME ZONE is the timezone setting, as PostgreSQL's raw tree names it",
            sql: "ALTER FUNCTION f SET TIME ZONE 'UTC'",
            pointer: "/0/alter_object/operands/1/tuple/expressions/1/identifier/name",
            expected_json: "\"timezone\"",
        },
        ValueTestCase {
            description: "SET TIME ZONE LOCAL sets the default",
            sql: "ALTER FUNCTION f SET TIME ZONE LOCAL",
            pointer: "/0/alter_object/operands/1/tuple/expressions/0/var/this",
            expected_json: "\"SET DEFAULT\"",
        },
        ValueTestCase {
            description: "RESET TRANSACTION ISOLATION LEVEL resets transaction_isolation",
            sql: "ALTER FUNCTION f RESET TRANSACTION ISOLATION LEVEL",
            pointer: "/0/alter_object/operands/1/tuple/expressions/1/identifier/name",
            expected_json: "\"transaction_isolation\"",
        },
        ValueTestCase {
            description: "ALTER ... NO DEPENDS ON EXTENSION names its action",
            sql: "ALTER PROCEDURE p NO DEPENDS ON EXTENSION e",
            pointer: "/0/alter_object/action",
            expected_json: "\"NO DEPENDS ON EXTENSION\"",
        },
        ValueTestCase {
            description: "SET STATISTICS reads a hexadecimal ICONST as its value",
            sql: "ALTER TABLE ONLY t ALTER c SET STATISTICS 0x10",
            pointer: "/0/alter_object/operands/0/tuple/expressions/2/tuple/expressions/1/literal/value",
            expected_json: "\"16\"",
        },
        ValueTestCase {
            description: "ALTER TABLE ONLY keeps ONLY on the relation",
            sql: "ALTER TABLE ONLY t ALTER c SET STATISTICS 5",
            pointer: "/0/alter_object/name/only",
            expected_json: "true",
        },
        ValueTestCase {
            description: "ALTER INDEX ALL IN TABLESPACE names the move as its action",
            sql: "ALTER INDEX ALL IN TABLESPACE a SET TABLESPACE b NOWAIT",
            pointer: "/0/alter_object/action",
            expected_json: "\"ALL IN TABLESPACE\"",
        },
        ValueTestCase {
            description: "ORDER BY ... USING keeps its operator, even < which ASC also means",
            sql: "VALUES (1) ORDER BY 1 USING <",
            pointer: "/0/select/order_by/expressions/0/this/with_operator/op",
            expected_json: "\"<\"",
        },
        ValueTestCase {
            description: "ORDER BY ... USING an operator of its own keeps it",
            sql: "SELECT a FROM t ORDER BY a USING ~<~",
            pointer: "/0/select/order_by/expressions/0/this/with_operator/op",
            expected_json: "\"~<~\"",
        },
        ValueTestCase {
            description: "ORDER BY ... USING is neither ASC nor DESC",
            sql: "SELECT a FROM t ORDER BY a USING >",
            pointer: "/0/select/order_by/expressions/0/desc",
            expected_json: "false",
        },
        ValueTestCase {
            description: "COPY TO PROGRAM names the command as a program target",
            sql: "COPY t1 TO PROGRAM 'm' WITH ( x )",
            pointer: "/0/copy/files/0/function/name",
            expected_json: "\"program\"",
        },
        ValueTestCase {
            description: "COPY TO a file whose options name program keeps the file",
            sql: "COPY t1 TO 'm' WITH ( program , x )",
            pointer: "/0/copy/files/0/literal/value",
            expected_json: "\"m\"",
        },
        ValueTestCase {
            description: "COPY FROM PROGRAM names the command as a program target",
            sql: "COPY t1 FROM PROGRAM 'm'",
            pointer: "/0/copy/files/0/function/name",
            expected_json: "\"program\"",
        },
        ValueTestCase {
            description: "TRUNCATE a * is one target, a, as in PostgreSQL's raw tree",
            sql: "TRUNCATE a *",
            pointer: "/0/truncate/extra_tables",
            expected_json: "[]",
        },
        ValueTestCase {
            description: "TRUNCATE a *, b * is two targets, the second b",
            sql: "TRUNCATE a *, b *",
            pointer: "/0/truncate/extra_tables/0/table/name/name",
            expected_json: "\"b\"",
        },
        ValueTestCase {
            description: "COPY (FORCE_QUOTE *) keeps the star the option takes",
            sql: "COPY t TO 'm' (FORCE_QUOTE *)",
            pointer: "/0/copy/params/0/value/star/table",
            expected_json: "null",
        },
        ValueTestCase {
            description: "COPY (force_quote) has no argument",
            sql: "COPY t TO 'm' (force_quote)",
            pointer: "/0/copy/params/0/value",
            expected_json: "null",
        },
        ValueTestCase {
            description: "COPY FROM (FORCE_NULL *) keeps the star",
            sql: "COPY t FROM 'm' (FORCE_NULL *)",
            pointer: "/0/copy/params/0/value/star/table",
            expected_json: "null",
        },
        ValueTestCase {
            description: "OPERATOR(\"s.x\".<) quotes its dotted schema",
            sql: "SELECT 1 OPERATOR(\"s.x\".<) 2",
            pointer: "/0/select/expressions/0/operator/operator/identifier/name",
            expected_json: "\"\\\"s.x\\\".<\"",
        },
        ValueTestCase {
            description: "OPERATOR(s.x.<) is a three-part name",
            sql: "SELECT 1 OPERATOR(s.x.<) 2",
            pointer: "/0/select/expressions/0/operator/operator/identifier/name",
            expected_json: "\"s.x.<\"",
        },
        ValueTestCase {
            description: "ORDER BY ... USING OPERATOR(\"s.x\".<) quotes its dotted schema",
            sql: "VALUES (1) ORDER BY 1 USING OPERATOR(\"s.x\".<)",
            pointer: "/0/select/order_by/expressions/0/this/with_operator/op",
            expected_json: "\"\\\"s.x\\\".<\"",
        },
        ValueTestCase {
            description: "a prefix OPERATOR(schema.op) is an unquoted call of the operator",
            sql: "SELECT OPERATOR(pg_catalog.-) 1",
            pointer: "/0/select/expressions/0/function/name",
            expected_json: "\"pg_catalog.-\"",
        },
        ValueTestCase {
            description: "IS UNKNOWN is a boolean test, not IS NULL",
            sql: "SELECT x IS UNKNOWN FROM t",
            pointer: "/0/select/expressions/0/is/right/var/this",
            expected_json: "\"UNKNOWN\"",
        },
        ValueTestCase {
            description: "IS NOT UNKNOWN is a boolean test, not IS NOT NULL",
            sql: "SELECT x IS NOT UNKNOWN FROM t",
            pointer: "/0/select/expressions/0/is/right/var/this",
            expected_json: "\"NOT UNKNOWN\"",
        },
        ValueTestCase {
            description: "op ANY with an operator of its own keeps it",
            sql: "SELECT 1 @> ANY (SELECT 1)",
            pointer: "/0/select/expressions/0/any/this/with_operator/op",
            expected_json: "\"@>\"",
        },
        ValueTestCase {
            description: "OPERATOR(schema.op) ALL keeps the qualified operator",
            sql: "SELECT 1 OPERATOR(pg_catalog.<) ALL (ARRAY[1])",
            pointer: "/0/select/expressions/0/all/this/with_operator/op",
            expected_json: "\"pg_catalog.<\"",
        },
        ValueTestCase {
            description: "an aggregate keeps a quoted dotted name quoted",
            sql: "SELECT \"s.f\"(1) FILTER (WHERE true)",
            pointer: "/0/select/expressions/0/aggregate_function/name",
            expected_json: "\"\\\"s.f\\\"\"",
        },
        ValueTestCase {
            description: "an aggregate keeps a qualified name's parts",
            sql: "SELECT s.f(*) FILTER (WHERE true)",
            pointer: "/0/select/expressions/0/aggregate_function/name",
            expected_json: "\"s.f\"",
        },
        ValueTestCase {
            description: "COLLATE \"s.c\" is one quoted name",
            sql: "SELECT 'a' COLLATE \"s.c\"",
            pointer: "/0/select/expressions/0/collation/collation",
            expected_json: "\"\\\"s.c\\\"\"",
        },
        ValueTestCase {
            description: "COLLATE \"s\".\"c\" is a qualified name",
            sql: "SELECT 'a' COLLATE \"s\".\"c\"",
            pointer: "/0/select/expressions/0/collation/collation",
            expected_json: "\"s.c\"",
        },
        ValueTestCase {
            description: "OVER w1 names the window itself",
            sql: "SELECT count(*) OVER w1 FROM t WINDOW w1 AS ()",
            pointer: "/0/select/expressions/0/window_function/over/alias/name",
            expected_json: "\"w1\"",
        },
        ValueTestCase {
            description: "OVER (w1) refers to w1 without naming the window",
            sql: "SELECT count(*) OVER (w1) FROM t WINDOW w1 AS ()",
            pointer: "/0/select/expressions/0/window_function/over/alias",
            expected_json: "null",
        },
        ValueTestCase {
            description: "BIT('10') keeps a modifier that is not an integer",
            sql: "SELECT CAST(x AS BIT('10')) FROM t",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"bit('10')\"",
        },
        ValueTestCase {
            description: "BIT(10,2) keeps its second modifier",
            sql: "SELECT CAST(x AS BIT(10,2)) FROM t",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"bit(10,2)\"",
        },
        ValueTestCase {
            description: "DECIMAL(3,v1) keeps an expression modifier, named as numeric",
            sql: "SELECT CAST(x AS DECIMAL(3,v1)) FROM t",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"numeric(3,v1)\"",
        },
        ValueTestCase {
            description: "INTERVAL(3) keeps its precision",
            sql: "SELECT CAST(x AS INTERVAL(3)) FROM t",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"interval(3)\"",
        },
        ValueTestCase {
            description: "INTERVAL(3) '1' keeps its precision",
            sql: "SELECT INTERVAL(3) '1'",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"interval(3)\"",
        },
        ValueTestCase {
            description: "INTERVAL SECOND(3) keeps its precision",
            sql: "SELECT INTERVAL '1' SECOND(3)",
            pointer: "/0/select/expressions/0/cast/to/unit",
            expected_json: "\"SECOND(3)\"",
        },
        ValueTestCase {
            description: "INTERVAL DAY TO SECOND(3) keeps its precision",
            sql: "SELECT CAST(x AS INTERVAL DAY TO SECOND(3)) FROM t",
            pointer: "/0/select/expressions/0/cast/to/to",
            expected_json: "\"SECOND(3)\"",
        },
        ValueTestCase {
            description: "AT LOCAL is timezone(expr), as in PostgreSQL's raw tree",
            sql: "SELECT ts AT LOCAL FROM t",
            pointer: "/0/select/expressions/0/function/name",
            expected_json: "\"pg_catalog.timezone\"",
        },
        ValueTestCase {
            description: "COMMENT ON names the object's kind",
            sql: "COMMENT ON MATERIALIZED VIEW s.v IS 'x'",
            pointer: "/0/comment/kind",
            expected_json: "\"MATERIALIZED VIEW\"",
        },
        ValueTestCase {
            description: "COMMENT ON keeps a qualified object name",
            sql: "COMMENT ON TABLE s.t IS 'x'",
            pointer: "/0/comment/this/dot/field/name",
            expected_json: "\"t\"",
        },
        ValueTestCase {
            description: "COMMENT ... IS NULL removes the comment",
            sql: "COMMENT ON COLUMN t.c IS NULL",
            pointer: "/0/comment/expression",
            expected_json: "{\"null\":null}",
        },
        ValueTestCase {
            description: "COMMENT ON FUNCTION keeps a VARIADIC argument type",
            sql: "COMMENT ON FUNCTION f(int, VARIADIC text[]) IS 'x'",
            pointer: "/0/comment/this/function/args/1/variadic/this/data_type/data_type",
            expected_json: "\"array\"",
        },
        ValueTestCase {
            description: "COMMENT ON OPERATOR keeps NONE for a missing operand",
            sql: "COMMENT ON OPERATOR - (NONE, int) IS 'x'",
            pointer: "/0/comment/this/operator/this",
            expected_json: "{\"null\":null}",
        },
        ValueTestCase {
            description: "COMMENT ON CONSTRAINT ON DOMAIN names the domain as a type",
            sql: "COMMENT ON CONSTRAINT k ON DOMAIN d IS 'x'",
            pointer: "/0/comment/this/tuple/expressions/1/data_type/name",
            expected_json: "\"d\"",
        },
        ValueTestCase {
            description: "a quoted type name doubles its embedded quotes in CAST",
            sql: "SELECT CAST(x AS \"a\"\".\"\"b\")",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"\\\"a\\\"\\\".\\\"\\\"b\\\"\"",
        },
        ValueTestCase {
            description: "a qualified quoted type name stays two parts",
            sql: "SELECT CAST(x AS \"a\".\"b\")",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"\\\"a\\\".\\\"b\\\"\"",
        },
        ValueTestCase {
            description: "x::type doubles embedded quotes in an array element type",
            sql: "SELECT x::\"a\"\".\"\"b\"[]",
            pointer: "/0/select/expressions/0/cast/to/element_type/name",
            expected_json: "\"\\\"a\\\"\\\".\\\"\\\"b\\\"\"",
        },
        ValueTestCase {
            description: "a typed literal doubles embedded quotes",
            sql: "SELECT \"a\"\".\"\"b\" 'v'",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"\\\"a\\\"\\\".\\\"\\\"b\\\"\"",
        },
        ValueTestCase {
            description: "a modified type name doubles embedded quotes",
            sql: "SELECT CAST(x AS \"a\"\".\"\"b\"(3))",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"\\\"a\\\"\\\".\\\"\\\"b\\\"(3)\"",
        },
        ValueTestCase {
            description: "a routine argument type doubles embedded quotes",
            sql: "CREATE FUNCTION f(x \"a\"\".\"\"b\") LANGUAGE sql",
            pointer: "/0/create_function/parameters/0/data_type/name",
            expected_json: "\"\\\"a\\\"\\\".\\\"\\\"b\\\"\"",
        },
        ValueTestCase {
            description: "a routine return type doubles embedded quotes",
            sql: "CREATE FUNCTION f() RETURNS \"a\"\".\"\"b\" LANGUAGE sql",
            pointer: "/0/create_function/return_type/name",
            expected_json: "\"\\\"a\\\"\\\".\\\"\\\"b\\\"\"",
        },
        ValueTestCase {
            description: "COMMENT ON TYPE doubles embedded quotes",
            sql: "COMMENT ON TYPE \"a\"\".\"\"b\" IS NULL",
            pointer: "/0/comment/this/data_type/name",
            expected_json: "\"\\\"a\\\"\\\".\\\"\\\"b\\\"\"",
        },
        ValueTestCase {
            description: "SECURITY LABEL ON TYPE doubles embedded quotes",
            sql: "SECURITY LABEL ON TYPE \"a\"\".\"\"b\" IS NULL",
            pointer: "/0/comment/this/tuple/expressions/1/data_type/name",
            expected_json: "\"\\\"a\\\"\\\".\\\"\\\"b\\\"\"",
        },
        ValueTestCase {
            description: "a %TYPE reference quotes a part with a space",
            sql: "CREATE FUNCTION f(x \"SETOF t\".c%TYPE) LANGUAGE sql",
            pointer: "/0/create_function/parameters/0/data_type/name",
            expected_json: "\"\\\"SETOF t\\\".c%TYPE\"",
        },
        ValueTestCase {
            description: "an unquoted type name folds to lower case and stays bare",
            sql: "SELECT CAST(x AS myType)",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_json: "\"mytype\"",
        },
        ValueTestCase {
            description: "COMMENT ON OPERATOR quotes a dotted schema",
            sql: "COMMENT ON OPERATOR \"s.t\".+ (int, int) IS 'x'",
            pointer: "/0/comment/this/operator/operator/identifier/name",
            expected_json: "\"\\\"s.t\\\".+\"",
        },
        ValueTestCase {
            description: "COMMENT ON OPERATOR keeps a qualified operator's parts",
            sql: "COMMENT ON OPERATOR s.t.+ (int, int) IS 'x'",
            pointer: "/0/comment/this/operator/operator/identifier/name",
            expected_json: "\"s.t.+\"",
        },
        ValueTestCase {
            description: "VARCHAR 'ab' is a typed literal",
            sql: "SELECT VARCHAR 'ab'",
            pointer: "/0/select/expressions/0/cast/to/data_type",
            expected_json: "\"var_char\"",
        },
        ValueTestCase {
            description: "a SETOF argument type keeps SETOF",
            sql: "CREATE FUNCTION f(a SETOF int) LANGUAGE sql",
            pointer: "/0/create_function/parameters/0/data_type/name",
            expected_json: "\"SETOF integer\"",
        },
        ValueTestCase {
            description: "a %TYPE argument type keeps its reference",
            sql: "CREATE FUNCTION f(b t.c%TYPE) LANGUAGE sql",
            pointer: "/0/create_function/parameters/0/data_type/name",
            expected_json: "\"t.c%TYPE\"",
        },
        ValueTestCase {
            description: "COMMENT ON TYPE SETOF keeps SETOF",
            sql: "COMMENT ON TYPE SETOF int IS NULL",
            pointer: "/0/comment/this/data_type/name",
            expected_json: "\"SETOF integer\"",
        },
        ValueTestCase {
            description: "RETURNS NULL ON NULL INPUT is STRICT, not a return type",
            sql: "CREATE FUNCTION f() RETURNS NULL ON NULL INPUT",
            pointer: "/0/create_function/returns_null_on_null_input",
            expected_json: "true",
        },
        ValueTestCase {
            description: "CREATE FUNCTION keeps an argument's mode and default",
            sql: "CREATE FUNCTION f(IN a int DEFAULT 1) RETURN a",
            pointer: "/0/create_function/parameters/0/mode",
            expected_json: "\"In\"",
        },
        ValueTestCase {
            description: "CREATE FUNCTION ... RETURN keeps the body expression",
            sql: "CREATE FUNCTION f(a int) RETURN a",
            pointer: "/0/create_function/body/Return/column/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "RETURNS TABLE is a marker option with the columns",
            sql: "CREATE FUNCTION f() RETURNS TABLE (c int) LANGUAGE sql",
            pointer: "/0/create_function/options/0/tuple/expressions/0/var/this",
            expected_json: "\"RETURNS TABLE\"",
        },
        ValueTestCase {
            description: "CREATE PROCEDURE keeps a BEGIN ATOMIC body's statements",
            sql: "CREATE PROCEDURE p() BEGIN ATOMIC SELECT 1; END",
            pointer: "/0/create_procedure/body/Statements/0/select/expressions/0/literal/value",
            expected_json: "\"1\"",
        },
        ValueTestCase {
            description: "BIT without a length is BIT(1), which casts '101' to B'1'",
            sql: "SELECT CAST('101' AS BIT)",
            pointer: "/0/select/expressions/0/cast/to",
            expected_json: "{\"data_type\":\"bit\",\"length\":1}",
        },
        ValueTestCase {
            description: "the type name \"bit\" takes no length, unlike the keyword BIT",
            sql: "SELECT CAST('101' AS \"bit\")",
            pointer: "/0/select/expressions/0/cast/to",
            expected_json: "{\"data_type\":\"bit\",\"length\":null}",
        },
        ValueTestCase {
            description: "the literal BIT '101' takes no length, as ConstBit clears it",
            sql: "SELECT BIT '101'",
            pointer: "/0/select/expressions/0/cast/to",
            expected_json: "{\"data_type\":\"bit\",\"length\":null}",
        },
        ValueTestCase {
            description: "CHAR without a length is CHAR(1), which casts 'ab' to 'a'",
            sql: "SELECT CAST('ab' AS CHAR)",
            pointer: "/0/select/expressions/0/cast/to",
            expected_json: "{\"data_type\":\"char\",\"length\":1}",
        },
        ValueTestCase {
            description: "NCHAR without a length is CHAR(1) too",
            sql: "SELECT CAST('ab' AS NCHAR)",
            pointer: "/0/select/expressions/0/cast/to",
            expected_json: "{\"data_type\":\"char\",\"length\":1}",
        },
        ValueTestCase {
            description: "the literal CHAR 'ab' takes no length, as ConstCharacter clears it",
            sql: "SELECT CHAR 'ab'",
            pointer: "/0/select/expressions/0/cast/to",
            expected_json: "{\"data_type\":\"char\",\"length\":null}",
        },
        ValueTestCase {
            description: "SECURITY LABEL is kept apart from COMMENT by its kind",
            sql: "SECURITY LABEL ON TABLE s.t IS 'x'",
            pointer: "/0/comment/kind",
            expected_json: "\"SECURITY LABEL TABLE\"",
        },
        ValueTestCase {
            description: "SECURITY LABEL without a provider pairs NULL with the object",
            sql: "SECURITY LABEL ON TABLE s.t IS 'x'",
            pointer: "/0/comment/this/tuple/expressions/0",
            expected_json: "{\"null\":null}",
        },
        ValueTestCase {
            description: "SECURITY LABEL FOR keeps the provider's name, as a word or a string",
            sql: "SECURITY LABEL FOR \"Sel\" ON ROLE r IS NULL",
            pointer: "/0/comment/this/tuple/expressions/0/literal/value",
            expected_json: "\"Sel\"",
        },
        ValueTestCase {
            description: "COMMENT ON FUNCTION keeps an IN mode apart from no mode",
            sql: "COMMENT ON FUNCTION f(IN a int) IS 'x'",
            pointer: "/0/comment/this/function/args/0/tuple/expressions/0/var/this",
            expected_json: "\"IN\"",
        },
        ValueTestCase {
            description: "COMMENT ON FUNCTION reads IN OUT as INOUT, as PostgreSQL does",
            sql: "COMMENT ON FUNCTION f(a IN OUT int) IS 'x'",
            pointer: "/0/comment/this/function/args/0/tuple/expressions/0/var/this",
            expected_json: "\"INOUT\"",
        },
        ValueTestCase {
            description: "COMMENT ON FUNCTION without a mode keeps the bare argument",
            sql: "COMMENT ON FUNCTION f(a int) IS 'x'",
            pointer: "/0/comment/this/function/args/0/column_def/name/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "a four-part function name keeps its first part, as func_name does",
            sql: "CREATE FUNCTION a.b.c.d () RETURNS int RETURN 1",
            pointer: "/0/create_function/name/catalog/name",
            expected_json: "\"a.b\"",
        },
        ValueTestCase {
            description: "a four-part old-style aggregate name keeps its first part",
            sql: "CREATE AGGREGATE a.b.c.d (basetype = int, sfunc = f, stype = int)",
            pointer: "/0/create_object/name/catalog/name",
            expected_json: "\"a.b\"",
        },
        ValueTestCase {
            description: "CONNECTION LIMIT is named connection_limit, as PostgreSQL names it",
            sql: "CREATE DATABASE d WITH CONNECTION LIMIT = -1",
            pointer: "/0/create_database/properties/0/tuple/expressions/0/identifier/name",
            expected_json: "\"connection_limit\"",
        },
        ValueTestCase {
            description: "a database option written DEFAULT keeps DEFAULT apart from a value",
            sql: "CREATE DATABASE d TEMPLATE DEFAULT",
            pointer: "/0/create_database/properties/0/tuple/expressions/1/var/this",
            expected_json: "\"DEFAULT\"",
        },
        ValueTestCase {
            description: "the pre-8.2 CREATE AGGREGATE form is kept apart by its source",
            sql: "CREATE AGGREGATE a (sfunc = f, stype = int)",
            pointer: "/0/create_object/source",
            expected_json: "\"OLD STYLE\"",
        },
        ValueTestCase {
            description: "a bare definition element keeps no value, apart from = true",
            sql: "CREATE AGGREGATE a (int) (sfunc = f, finalfunc_extra)",
            pointer: "/0/create_object/properties/1/value",
            expected_json: "null",
        },
        ValueTestCase {
            description: "CREATE AGGREGATE keeps its argument list as a call",
            sql: "CREATE AGGREGATE a (int) (sfunc = f, finalfunc_extra)",
            pointer: "/0/create_object/operands/0/function/args/0/data_type/data_type",
            expected_json: "\"int\"",
        },
        ValueTestCase {
            description: "a table constraint keeps how many columns precede it",
            sql: "CREATE TABLE t (a int, CHECK (a > 0))",
            pointer: "/0/create_table/properties/0/tuple/expressions/1/literal/value",
            expected_json: "\"1\"",
        },
        ValueTestCase {
            description: "a hash bound keeps its modulus first, as the raw tree's fields do",
            sql: "CREATE TABLE t PARTITION OF p FOR VALUES WITH (REMAINDER 1, MODULUS 4)",
            pointer: "/0/create_table/properties/0/tuple/expressions/1/literal/value",
            expected_json: "\"4\"",
        },
        ValueTestCase {
            description: "a column collation keeps a dotted name's quoting",
            sql: "CREATE TABLE t (a int COLLATE \"s.c\")",
            pointer: "/0/create_table/columns/0/options/0/tuple/expressions/1/identifier/name",
            expected_json: "\"\\\"s.c\\\"\"",
        },
        ValueTestCase {
            description: "ON DELETE SET NULL keeps its column list",
            sql: "CREATE TABLE t (a int REFERENCES p ON DELETE SET NULL (a))",
            pointer: "/0/create_table/columns/0/options/0/tuple/expressions/2/tuple/expressions/2/tuple/expressions/0/identifier/name",
            expected_json: "\"a\"",
        },
        ValueTestCase {
            description: "CREATE TABLE AS keeps WITH NO DATA",
            sql: "CREATE UNLOGGED TABLE t (a, b) AS SELECT 1, 2 WITH NO DATA",
            pointer: "/0/create_table/with_data",
            expected_json: "false",
        },
        ValueTestCase {
            description: "UNLOGGED is kept as the table's modifier",
            sql: "CREATE UNLOGGED TABLE t (a, b) AS SELECT 1, 2 WITH NO DATA",
            pointer: "/0/create_table/table_modifier",
            expected_json: "\"UNLOGGED\"",
        },
        ValueTestCase {
            description: "a partitioning strategy is matched without regard to case",
            sql: "CREATE TABLE t (a int) PARTITION BY \"Range\" (a)",
            pointer: "/0/create_table/properties/0/tuple/expressions/1/var/this",
            expected_json: "\"RANGE\"",
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
