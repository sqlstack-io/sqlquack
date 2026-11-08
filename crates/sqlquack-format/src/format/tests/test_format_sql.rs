use crate::format::models::FormatDialect;

use crate::format::constants::{
    BATCH_SEPARATOR_FAILURE, BIGQUERY_OPAQUE_STATEMENT_FAILURE, DEPTH_FAILURE, JOIN_FAILURE,
    PARSE_FAILURE, SCANNER_MISMATCH_FAILURE, SEPARATION_FAILURE,
    SQLSERVER_OPAQUE_STATEMENT_FAILURE, TYPE_DEPTH_FAILURE, UNSUPPORTED_DIALECT_FAILURE,
};
use crate::format::main::format_sql::format_sql;
use crate::format::models::FormatOptions;
use crate::format::tests::helpers::{format_time, refusal_start};
use crate::format::tests::test_types;

#[test]
fn given_sql_whose_spacing_carries_meaning_when_formatting_then_it_is_kept_or_refused() {
    let test_cases = [
        test_types::FormatSqlTestCase {
            description: "struct field names spelled like keywords keep their case in a column type",
            sql: "create table t (x object(type varchar, first int, key int))",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("CREATE TABLE t(x OBJECT(type varchar, first int, key int))"),
        },
        test_types::FormatSqlTestCase {
            description: "DuckDB struct field names spelled like keywords keep their case in a column type",
            sql: "create table t (x struct(type varchar, first int, key int, date date))",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Ok(
                "CREATE TABLE t(x STRUCT(type varchar, first int, key int, date DATE))",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a DuckDB macro name keeps its case, since DuckDB stores it as written",
            sql: "create macro add_one(x) as x + 1",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Ok("CREATE macro add_one(x) AS x + 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a temporary replacing DuckDB macro name keeps its case",
            sql: "create or replace temp macro add_one(x) as x + 1",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Ok("CREATE OR REPLACE TEMP macro add_one(x) AS x + 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a DuckDB table macro name keeps its case",
            sql: "create macro first_rows(n) as table select * from t limit n",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Ok(
                "CREATE macro first_rows(n) AS table SELECT\n  *\nFROM t\nLIMIT n",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "an unquoted PUT file URI is not split at its slashes",
            sql: "PUT file:///data/x.csv @s1/p1/",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "an unquoted GET file URI is not split at its slashes",
            sql: "GET @s1/p1/ file:///tmp/",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a hex-looking number is not split into a number and an alias",
            sql: "SELECT 0x1F",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a MATCH_RECOGNIZE pattern keeps its reluctant quantifier `*?`",
            sql: "SELECT * FROM t MATCH_RECOGNIZE (ORDER BY b PATTERN (A{2,} B*? C+) DEFINE A AS a > 0)",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "SELECT *\nFROM t MATCH_RECOGNIZE(ORDER BY b PATTERN(A{2,} B*? C+) DEFINE A AS a > 0)",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a comparison written without spaces is spaced",
            sql: "select a from t where x=1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT a\nFROM t\nWHERE x = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a join condition written without spaces is spaced",
            sql: "select a from t join u on t.id=u.id",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT a\nFROM t\nJOIN u\n  ON t.id = u.id"),
        },
        test_types::FormatSqlTestCase {
            description: "two-character comparisons stay whole",
            sql: "select a from t where x<=1 and y!=2 and z<>3",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT a\nFROM t\nWHERE\n  x <= 1\n  AND y != 2\n  AND z <> 3"),
        },
        test_types::FormatSqlTestCase {
            description: "an operator chain and a concatenation are spaced",
            sql: "select a x from t where a||b = 'x' and c+d*e-f = 1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "SELECT a x\nFROM t\nWHERE\n  a || b = 'x'\n  AND c + d * e - f = 1",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "exponent signs stay inside their numbers",
            sql: "select a y from t where y=1e-3 and z=1E+5",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT a y\nFROM t\nWHERE\n  y = 1e-3\n  AND z = 1E+5"),
        },
        test_types::FormatSqlTestCase {
            description: "stars that are not multiplication stay touching",
            sql: "select t.*, count(*) c from t where x=1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT\n  t.*,\n  COUNT(*) c\nFROM t\nWHERE x = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "path and cast accessors stay touching",
            sql: "select x:y z, x::int w from t where x::int=1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT\n  x:y z,\n  x::int w\nFROM t\nWHERE x::int = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a named argument arrow is not split",
            sql: "select f(a=>1) y from t",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a doubled minus keeps its space",
            sql: "select a y from t where x = - -1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT a y\nFROM t\nWHERE x = - -1"),
        },
        test_types::FormatSqlTestCase {
            description: "a multiplication beside a comment keeps the comment apart",
            sql: "select a y from t where x=a/*c*/*b",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT a y\nFROM t\nWHERE\n  x = a /*c*/ * b"),
        },
        test_types::FormatSqlTestCase {
            description: "a star touching a comment opener is refused",
            sql: "select a y from t where x*/*c*/b=1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "an operator beside a bind or session variable is refused",
            sql: "select a y from t where x=$1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a number touching a keyword is refused",
            sql: "select a y from t where x=1and y=2",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "operators in a literal-only chain without parse spans are refused",
            sql: "select a y from t where 1+2+3=6",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a long array breaks just inside its brackets",
            sql: "select ['aaaaaaaaaa','bbbbbbbbbb','cccccccccc','dddddddddd'] y from t",
            line_width: Some(40),
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "SELECT\n  [\n    'aaaaaaaaaa',\n    'bbbbbbbbbb',\n    'cccccccccc',\n    'dddddddddd'\n  ] y\nFROM t",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "PIVOT IN values keep their spacing, which names the pivot columns",
            sql: "SELECT * FROM t PIVOT (SUM(v) FOR k IN (1+1, 2-1)) ORDER BY 1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "SELECT *\nFROM t PIVOT(SUM(v) FOR k IN (1+1, 2-1))\nORDER BY\n  1",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "PIVOT IN concatenations keep their spacing",
            sql: "SELECT * FROM t PIVOT (SUM(v) FOR k IN ('a'||'b', 'c')) ORDER BY 1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "SELECT *\nFROM t PIVOT(SUM(v) FOR k IN ('a'||'b', 'c'))\nORDER BY\n  1",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "PIVOT IN values keep a spaced unary minus",
            sql: "SELECT * FROM t PIVOT (SUM(v) FOR k IN (- 1, 2)) ORDER BY 1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT *\nFROM t PIVOT(SUM(v) FOR k IN (- 1, 2))\nORDER BY\n  1"),
        },
        test_types::FormatSqlTestCase {
            description: "select items without an alias keep their text, which names their column",
            sql: "SELECT x FROM (SELECT a+b, - 1, t . c AS d FROM t) WHERE \"A+B\" > 1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "SELECT x\nFROM (\n  SELECT\n    a+b,\n    - 1,\n    t.c AS d\n  FROM t\n)\nWHERE \"A+B\" > 1",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a CTAS select item without an alias keeps its keyword case",
            sql: "CREATE OR REPLACE TABLE t2 AS SELECT count( distinct g ), g FROM t",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "CREATE OR REPLACE TABLE t2 AS SELECT\n  count( distinct g ),\n  g\nFROM t",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a second trailing line comment keeps its indentation",
            sql: "SELECT a FROM t -- one\n-- two",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT a\nFROM t -- one\n-- two"),
        },
        test_types::FormatSqlTestCase {
            description: "a minus is not joined to a DuckDB `@`, which would read as the operator `-@`",
            sql: "select - @a",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Err(JOIN_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a plus is not joined to a DuckDB `@`",
            sql: "select + @a",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Err(JOIN_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a DuckDB `@` is not joined to a plus sign",
            sql: "select @ +1",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Err(JOIN_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a DuckDB `@` is not joined to a minus sign, which would read as the operator `@-`",
            sql: "select @ -1",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Err(JOIN_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a DuckDB `@` is not joined to a minus before a column",
            sql: "select @ - a",
            line_width: None,
            dialect: FormatDialect::DuckDB,
            expected_outcome: Err(JOIN_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "PostgreSQL's own scanner decides which words are keywords, so CURRENT_DATE and COALESCE are recased, and a called name is recased as in every dialect",
            sql: "select date_part('dow', current_date), coalesce(a, 0) from t",
            line_width: None,
            dialect: FormatDialect::PostgreSQL,
            expected_outcome: Ok(
                "SELECT\n  DATE_PART('dow', CURRENT_DATE),\n  COALESCE(a, 0)\nFROM t",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "keyword-spelled column names keep their case",
            sql: "select name, value, type from t",
            line_width: None,
            dialect: FormatDialect::PostgreSQL,
            expected_outcome: Ok("SELECT\n  name,\n  value,\n  type\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "a word matching an alias the parse tree records without a span keeps its case",
            sql: "select * from t, lateral f(x) with ordinality as elem(value, ordinality)",
            line_width: None,
            dialect: FormatDialect::PostgreSQL,
            expected_outcome: Ok(
                "SELECT *\nFROM t, LATERAL F(x) WITH ordinality AS elem(value, ordinality)",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a PostgreSQL TABLESAMPLE stays on its table's line",
            sql: "select * from t tablesample system (50)",
            line_width: None,
            dialect: FormatDialect::PostgreSQL,
            expected_outcome: Ok("SELECT *\nFROM t TABLESAMPLE SYSTEM (50)"),
        },
        test_types::FormatSqlTestCase {
            description: "a token the PostgreSQL scanner reads differently from the shared tokenizer is refused",
            sql: "select a <<->> b",
            line_width: None,
            dialect: FormatDialect::PostgreSQL,
            expected_outcome: Err(SCANNER_MISMATCH_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a quantified LIKE puts its operand on its own lines",
            sql: "select x from t where x like any (array['a'])",
            line_width: None,
            dialect: FormatDialect::PostgreSQL,
            expected_outcome: Ok("SELECT x\nFROM t\nWHERE\n  x LIKE ANY(\n    ARRAY['a']\n  )"),
        },
        test_types::FormatSqlTestCase {
            description: "SELECT INTO starts its own line",
            sql: "select a into temp b from c",
            line_width: None,
            dialect: FormatDialect::PostgreSQL,
            expected_outcome: Ok("SELECT a\nINTO TEMP b\nFROM c"),
        },
        test_types::FormatSqlTestCase {
            description: "SQL Server bracketed names stay as written, a keyword inside brackets included",
            sql: "select [Order Id], [select] from [dbo].[Orders] o",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT\n  [Order Id],\n  [select]\nFROM [dbo].[Orders] o"),
        },
        test_types::FormatSqlTestCase {
            description: "SQL Server temporary table names stay one touching token each",
            sql: "SELECT * FROM #t JOIN ##g ON #t.id = ##g.id",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT *\nFROM #t\nJOIN ##g\n  ON #t.id = ##g.id"),
        },
        test_types::FormatSqlTestCase {
            description: "a SQL Server N prefix stays on its string, and a spaced N stays a column with an alias",
            sql: "select N'abc' as n, n'x' + N 'y'",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT\n  N'abc' AS n,\n  n'x' + N 'y'"),
        },
        test_types::FormatSqlTestCase {
            description: "SQL Server variables and system functions stay as written",
            sql: "SELECT @@ROWCOUNT, @v, @@version",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT\n  @@ROWCOUNT,\n  @v,\n  @@version"),
        },
        test_types::FormatSqlTestCase {
            description: "an empty SQL Server schema part stays touching",
            sql: "SELECT * FROM db..t",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT *\nFROM db..t"),
        },
        test_types::FormatSqlTestCase {
            description: "a SQL Server variable assignment is spaced like a comparison",
            sql: "SELECT @x   =   1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT @x = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a SQL Server variable is a whole token, so an operator it touches is spaced from it",
            sql: "SELECT * FROM t WHERE a=@x AND @y<>b",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT *\nFROM t\nWHERE\n  a = @x AND @y <> b"),
        },
        test_types::FormatSqlTestCase {
            description: "an operator touching a SQL Server variable and a literal is refused, since neither operand has a source position",
            sql: "SELECT @v+1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "SQL Server TOP and a WITH table hint keep their space before `(`",
            sql: "select top (5) a from t with(nowait)",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT TOP (5) a\nFROM t WITH (nowait)"),
        },
        test_types::FormatSqlTestCase {
            description: "SQL Server block comments nest",
            sql: "SELECT 1 /* a /* nested */ b */ FROM t",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT 1 /* a /* nested */ b */\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "a GO alone on its line is a client batch separator and is refused",
            sql: "SELECT 1\nGO\nSELECT 2",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(BATCH_SEPARATOR_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a GO followed by a repeat count is a client batch separator and is refused",
            sql: "SELECT 1\nGO 2\nSELECT 2",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(BATCH_SEPARATOR_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a zero-width space the SQL Server scanner skips between tokens is refused, not dropped",
            sql: "SELECT a\u{200B} FROM t",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err("formatter tokenization failed: no token covers byte 8"),
        },
        test_types::FormatSqlTestCase {
            description: "the same SQL with a plain space between the tokens is laid out",
            sql: "SELECT a  FROM t",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT a\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "a leading byte order mark the SQL Server scanner skips is refused, not dropped",
            sql: "\u{FEFF}SELECT 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err("formatter tokenization failed: no token covers byte 0"),
        },
        test_types::FormatSqlTestCase {
            description: "the same SQL with a leading plain space is laid out",
            sql: " SELECT 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT 1"),
        },
        test_types::FormatSqlTestCase {
            description: "SQL Server CROSS APPLY and OUTER APPLY start a line like a join, and a table-valued function keeps its case",
            sql: "SELECT a FROM t CROSS APPLY f(t.x) AS g OUTER APPLY h(t.y) AS k WHERE a = 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok(
                "SELECT a\nFROM t\nCROSS APPLY f(t.x) AS g\nOUTER APPLY h(t.y) AS k\nWHERE a = 1",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a SQL Server table-valued function after FROM, a FROM-list comma or JOIN keeps its case, as a case-sensitive collation resolves it by case, while a built-in call recases",
            sql: "select upper(a) from f(1) as x, g(2) as y join h(3) as z on 1 = 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok(
                "SELECT UPPER(a)\nFROM f(1) AS x, g(2) AS y\nJOIN h(3) AS z\n  ON 1 = 1",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a SQL Server TOP count that only starts with a subquery stays on the SELECT line",
            sql: "SELECT TOP ((SELECT 1) + 1) * FROM t",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT TOP ((SELECT 1) + 1) *\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "an INSERT target without INTO keeps its case before its column list",
            sql: "insert t (a) values (1)",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("INSERT t(a)\nVALUES (1)"),
        },
        test_types::FormatSqlTestCase {
            description: "a CREATE INDEX table keeps its case before its column list",
            sql: "create index ix on t (c)",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("CREATE INDEX ix ON t(c)"),
        },
        test_types::FormatSqlTestCase {
            description: "CREATE STATISTICS is kept opaque by the parser and refused",
            sql: "create statistics s on t (c)",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a FOREIGN KEY ... REFERENCES table is kept opaque by the parser and refused",
            sql: "create table c (a int, constraint fk foreign key (a) references t (id))",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a view name keeps its case before its column list",
            sql: "CREATE VIEW v (a) AS SELECT 1 AS a",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("CREATE VIEW v(a) AS SELECT\n  1 AS a"),
        },
        test_types::FormatSqlTestCase {
            description: "a function name keeps its case before its parameter list",
            sql: "CREATE FUNCTION f (@a int) RETURNS int AS BEGIN RETURN @a END",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("CREATE FUNCTION f(@a int) RETURNS int AS BEGIN RETURN @a END"),
        },
        test_types::FormatSqlTestCase {
            description: "CREATE PROCEDURE is kept opaque by the parser and refused",
            sql: "create procedure p (@a int) as select @a",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "UPDATE STATISTICS is kept opaque by the parser and refused",
            sql: "update statistics t (ix)",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a DELETE's joined table keeps its case before an old-style hint, which the parser reads as a call",
            sql: "delete t from t join s (nolock) on t.a = s.a",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("DELETE t FROM t JOIN s(nolock) ON t.a = s.a"),
        },
        test_types::FormatSqlTestCase {
            description: "an UPDATE's joined table keeps its case before an old-style hint",
            sql: "update t set a = 1 from t join s (nolock) on 1 = 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("UPDATE t SET a = 1 FROM t JOIN s(nolock) ON 1 = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "lower-case keywords after a SQL Server object keyword still recase, since only a name before `(` keeps its case",
            sql: "delete from t where a = 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("DELETE FROM t WHERE a = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a lower-case INSERT INTO recases INTO while the target before its column list keeps its case",
            sql: "insert into t (a) values (1)",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("INSERT INTO t(a)\nVALUES (1)"),
        },
        test_types::FormatSqlTestCase {
            description: "a lower-case MERGE INTO recases INTO",
            sql: "merge into t using s on t.a = s.a when matched then delete;",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("MERGE INTO t USING s ON t.a = s.a WHEN matched THEN DELETE;"),
        },
        test_types::FormatSqlTestCase {
            description: "a lower-case DROP TABLE IF EXISTS recases IF",
            sql: "drop table if exists t",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("DROP TABLE IF EXISTS t"),
        },
        test_types::FormatSqlTestCase {
            description: "a call in a derived table's select list, after a FROM-list comma, still recases",
            sql: "select * from t, (select a, upper(a) as b from s) as q, k(2) as w",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok(
                "SELECT *\nFROM t, (\n  SELECT\n    a,\n    UPPER(a) AS b\n  FROM s\n) AS q, k(2) AS w",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a GO that is not alone on its line is a name and is laid out",
            sql: "SELECT go FROM t",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT go\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "a SQL Server statement the parser keeps as opaque text is refused",
            sql: "SELECT * FROM t TABLESAMPLE (10 PERCENT)",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery dashed table path stays touching",
            sql: "select * from my-project.ds.t as x",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT *\nFROM my-project.ds.t AS x"),
        },
        test_types::FormatSqlTestCase {
            description: "BigQuery struct field names keep their case, so the columns of .* keep theirs",
            sql: "select struct<type string, first int64>('a', 1).* from t",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT STRUCT<type string, first int64> ('a', 1).*\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "BigQuery struct field names keep their case in a column type",
            sql: "create table t (x struct<type string, first int64>, y array<struct<key string, date date>>)",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok(
                "CREATE TABLE t(x STRUCT<type string, first int64>, y ARRAY<STRUCT<key string, date DATE>>)",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "BigQuery type parameters stay inside their angle brackets",
            sql: "select array<int64>[1, 2], struct<a int64>(1) from t",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT\n  ARRAY<int64>[1, 2],\n  STRUCT<a int64> (1)\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery system variable and query parameter keep their @",
            sql: "select @@time_zone, @p from t",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT\n  @@time_zone,\n  @p\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "BigQuery's * EXCEPT is a column list, not a set operation",
            sql: "select * except (b), b+1 from t",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT\n  * EXCEPT (b),\n  b + 1\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "SELECT AS STRUCT stays on the SELECT line",
            sql: "select as struct 1 as a, 2 as b",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT AS struct\n  1 AS a,\n  2 AS b"),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery set operation's prefix and modifiers share its line",
            sql: "select 1 as x left union all corresponding select 2 as x",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT 1 AS x\nLEFT UNION ALL corresponding\nSELECT 2 AS x"),
        },
        test_types::FormatSqlTestCase {
            description: "UNNEST ... WITH OFFSET does not start an OFFSET clause",
            sql: "select a from unnest(x) as a with offset as o",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT a\nFROM UNNEST(x) AS a WITH OFFSET AS o"),
        },
        test_types::FormatSqlTestCase {
            description: "IS DISTINCT FROM does not start a FROM clause",
            sql: "select * from t qualify a is distinct from b",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT *\nFROM t\nQUALIFY a IS DISTINCT FROM b"),
        },
        test_types::FormatSqlTestCase {
            description: "BigQuery names an unaliased item f0_, so its spacing is laid out",
            sql: "select a+b from t",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT a + b\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery # comment is kept",
            sql: "select a # note\nfrom t",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT a # note\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery operator whose operands the tree records without a span stays touching",
            sql: "select 2*-3",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Err(SEPARATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery ARRAY, a reserved word the shared tokenizer reads as a name, stays touching its brackets",
            sql: "select array[1, 2] as a, array[1][offset(0)] as b",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT\n  ARRAY[1, 2] AS a,\n  ARRAY[1][OFFSET(0)] AS b"),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery script statement is refused",
            sql: "declare x int64",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Err(BIGQUERY_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery foreign key's table keeps its case, since BigQuery table names are case-sensitive",
            sql: "create table t (a int64, foreign key (a) references p (id) not enforced)",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok(
                "CREATE TABLE t(a int64, FOREIGN KEY (a) REFERENCES p(id) NOT enforced)",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery search index's table keeps its case",
            sql: "create search index ix on t (c)",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("CREATE search INDEX ix ON t(c)"),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery INSERT's table keeps its case before its column list",
            sql: "insert t (a) values (1)",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("INSERT t(a)\nVALUES (1)"),
        },
        test_types::FormatSqlTestCase {
            description: "a BigQuery table function keeps its case when qualified, and an unqualified one, always a built-in, recases",
            sql: "select * from ds.tf(1), external_query('c', 'select 1')",
            line_width: None,
            dialect: FormatDialect::BigQuery,
            expected_outcome: Ok("SELECT *\nFROM ds.tf(1), EXTERNAL_QUERY('c', 'select 1')"),
        },
        test_types::FormatSqlTestCase {
            description: "Databricks is refused until it has a format gate",
            sql: "SELECT a FROM t",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(UNSUPPORTED_DIALECT_FAILURE),
        },
    ];
    for test_case in test_cases {
        let options = FormatOptions {
            line_width: test_case.line_width,
            ..FormatOptions::new(test_case.dialect)
        };
        let actual = format_sql(test_case.sql, &options);
        assert_eq!(
            actual
                .map(|formatted| formatted.sql)
                .map_err(|refusal| refusal.reason),
            test_case
                .expected_outcome
                .map(str::to_string)
                .map_err(str::to_string),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_deeply_nested_sql_when_formatting_then_it_is_refused_or_laid_out_without_exhausting_the_stack()
 {
    let test_cases = [
        test_types::DeepSqlTestCase {
            description: "1,500 SQL Server parentheses around a FROM source are kept opaque by the parser's depth guard and refused",
            dialects: &[FormatDialect::SqlServer],
            sql: || format!("SELECT * FROM {}t{}", "(".repeat(1_500), ")".repeat(1_500)),
            expected_refusal: Some(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 nested SQL Server subqueries are kept opaque by the parser's depth guard and refused",
            dialects: &[FormatDialect::SqlServer],
            sql: || {
                format!(
                    "{}SELECT 1 AS c{}",
                    "SELECT * FROM (".repeat(1_500),
                    ") AS q".repeat(1_500)
                )
            },
            expected_refusal: Some(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 nested SQL Server NOTs are kept opaque by the parser's depth guard and refused",
            dialects: &[FormatDialect::SqlServer],
            sql: || format!("SELECT * FROM t WHERE {}1 = 1", "NOT ".repeat(1_500)),
            expected_refusal: Some(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a SQL Server chain of 5,000 added terms is kept opaque by the parser's depth guard and refused",
            dialects: &[FormatDialect::SqlServer],
            sql: || format!("SELECT {}", vec!["1"; 5_000].join(" + ")),
            expected_refusal: Some(SQLSERVER_OPAQUE_STATEMENT_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "254 nested SQL Server subqueries, under the parser's depth guard, are laid out",
            dialects: &[FormatDialect::SqlServer],
            sql: || {
                format!(
                    "{}SELECT 1 AS c{}",
                    "SELECT * FROM (".repeat(254),
                    ") AS q".repeat(254)
                )
            },
            expected_refusal: None,
        },
        test_types::DeepSqlTestCase {
            description: "1,500 parentheses around a FROM source are refused by the parser's nesting guard",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT * FROM {}t{}", "(".repeat(1_500), ")".repeat(1_500)),
            expected_refusal: Some(PARSE_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 parentheses around an expression are refused by the parser's nesting guard",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT {}1{}", "(".repeat(1_500), ")".repeat(1_500)),
            expected_refusal: Some(PARSE_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a chain of 5,000 added terms is refused by the PostgreSQL parser's depth limit",
            dialects: &[FormatDialect::PostgreSQL],
            sql: || format!("SELECT {}", vec!["1"; 5_000].join(" + ")),
            expected_refusal: Some(PARSE_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 nested subqueries are refused by the parser's depth guard",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || {
                format!(
                    "{}SELECT 1{}",
                    "SELECT * FROM (".repeat(1_500),
                    ") AS q".repeat(1_500)
                )
            },
            expected_refusal: Some(PARSE_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 nested NOTs are refused by the parser's depth guard",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT {}TRUE", "NOT ".repeat(1_500)),
            expected_refusal: Some(PARSE_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a chain of 5,000 added terms is refused before its tree is walked",
            dialects: &[FormatDialect::Snowflake, FormatDialect::DuckDB],
            sql: || format!("SELECT {}", vec!["1"; 5_000].join(" + ")),
            expected_refusal: Some(DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a chain of 513 added terms, one level past the limit, is refused",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT {}", vec!["1"; 513].join(" + ")),
            expected_refusal: Some(DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to a type with 300 `[]` suffixes is refused before parsing",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT CAST(NULL AS INT{})", "[]".repeat(300)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to a type with 300 `[3]` suffixes is refused before parsing",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT CAST(NULL AS INT{})", "[3]".repeat(300)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a column of a type with 65 `[]` suffixes, one past the limit, is refused",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("CREATE TABLE t (c INT{})", "[]".repeat(65)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a column of a type with 64 `[]` suffixes, at the limit, is laid out",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("CREATE TABLE t (c INT{})", "[]".repeat(64)),
            expected_refusal: None,
        },
        test_types::DeepSqlTestCase {
            description: "a DuckDB `::` cast to a type with 300 `[]` suffixes is refused before parsing",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT NULL::INT{}", "[]".repeat(300)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a type alternating angle brackets with suffix runs, depth 182, is refused",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || {
                format!(
                    "SELECT CAST(NULL AS ARRAY<ARRAY<INT{}{})",
                    "[]".repeat(60) + ">",
                    "[]".repeat(60) + ">" + &"[]".repeat(60)
                )
            },
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to 2 angle brackets around 63 suffixes, depth 65, is refused",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT CAST(NULL AS ARRAY<ARRAY<INT{}>>)", "[]".repeat(63)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to 2 angle brackets around 62 suffixes, depth 64, is laid out",
            dialects: &[FormatDialect::Snowflake],
            sql: || format!("SELECT CAST(NULL AS ARRAY<ARRAY<INT{}>>)", "[]".repeat(62)),
            expected_refusal: None,
        },
        test_types::DeepSqlTestCase {
            description: "a struct of 100 one-suffix fields stays shallow and is laid out",
            dialects: &[FormatDialect::Snowflake],
            sql: || {
                format!(
                    "SELECT CAST(NULL AS STRUCT<{}>)",
                    (0..100)
                        .map(|field| format!("f{field} INT[]"))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            },
            expected_refusal: None,
        },
        test_types::DeepSqlTestCase {
            description: "a type alternating parenthesised constructors with suffix runs, depth 182, is refused",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || {
                format!(
                    "SELECT CAST(NULL AS ARRAY(ARRAY(INT{}{})",
                    "[]".repeat(60) + ")",
                    "[]".repeat(60) + ")" + &"[]".repeat(60)
                )
            },
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a type alternating quoted parenthesised constructors with suffix runs, depth 182, is refused",
            dialects: &[FormatDialect::Snowflake, FormatDialect::DuckDB],
            sql: || {
                format!(
                    "SELECT CAST(NULL AS \"ARRAY\"(\"ARRAY\"(INT{}{})",
                    "[]".repeat(60) + ")",
                    "[]".repeat(60) + ")" + &"[]".repeat(60)
                )
            },
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a column alternating quoted STRUCT<...> and ARRAY<...> with suffix runs is refused",
            dialects: &[FormatDialect::Snowflake, FormatDialect::DuckDB],
            sql: || {
                let run = "[]".repeat(40);
                format!("CREATE TABLE t (c \"STRUCT\"<a \"ARRAY\"<INT{run}>{run}>{run})")
            },
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a column alternating STRUCT(...), MAP(...) and ARRAY<...> with suffix runs is refused",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || {
                let run = "[]".repeat(30);
                format!(
                    "CREATE TABLE t (c STRUCT(a MAP(INT, ARRAY<OBJECT(b INT{run}){run}>{run}){run}){run})"
                )
            },
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to 2 parenthesised constructors around 63 suffixes, depth 65, is refused",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT CAST(NULL AS ARRAY(ARRAY(INT{})))", "[]".repeat(63)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to 2 parenthesised constructors around 62 suffixes, depth 64, is laid out",
            dialects: &[FormatDialect::Snowflake, FormatDialect::DuckDB],
            sql: || format!("SELECT CAST(NULL AS ARRAY(ARRAY(INT{})))", "[]".repeat(62)),
            expected_refusal: None,
        },
        test_types::DeepSqlTestCase {
            description: "a cast to 300 nested ARRAY<...> types is refused before parsing",
            dialects: &[FormatDialect::Snowflake, FormatDialect::DuckDB],
            sql: || {
                format!(
                    "SELECT CAST(NULL AS {}INT{})",
                    "ARRAY<".repeat(300),
                    ">".repeat(300)
                )
            },
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a chain of 512 added terms, at the limit, is laid out",
            dialects: &[
                FormatDialect::Snowflake,
                FormatDialect::DuckDB,
                FormatDialect::PostgreSQL,
            ],
            sql: || format!("SELECT {}", vec!["1"; 512].join(" + ")),
            expected_refusal: None,
        },
    ];
    for test_case in test_cases {
        for &dialect in test_case.dialects {
            let outcome = format_sql(&(test_case.sql)(), &FormatOptions::new(dialect));
            assert_eq!(
                outcome
                    .err()
                    .map(|refusal| refusal_start(&refusal.reason).to_string()),
                test_case.expected_refusal.map(str::to_string),
                "{dialect:?}: {}",
                test_case.description
            );
        }
    }
}

#[test]
#[ignore = "timing"]
fn given_wide_sql_when_formatting_as_sql_server_then_it_takes_about_as_long_as_snowflake() {
    let test_cases = [
        test_types::TimedSqlTestCase {
            description: "a select list of 10,000 columns",
            sql: || format!("SELECT {} FROM t", vec!["a"; 10_000].join(", ")),
            expected_max_ratio: 3,
        },
        test_types::TimedSqlTestCase {
            description: "a FROM list of 5,000 tables, whose commas the guide classifies",
            sql: || format!("SELECT 1 AS c FROM {}", vec!["t"; 5_000].join(", ")),
            expected_max_ratio: 3,
        },
        test_types::TimedSqlTestCase {
            description: "a select list of 10,000 columns, one per line, where each line is checked for GO",
            sql: || format!("SELECT a\n{} FROM t", vec![", a"; 10_000].join("\n")),
            expected_max_ratio: 3,
        },
    ];
    for test_case in test_cases {
        let sql = (test_case.sql)();
        let baseline = format_time(&sql, FormatDialect::Snowflake);
        let sqlserver = format_time(&sql, FormatDialect::SqlServer);
        assert!(
            sqlserver <= baseline * test_case.expected_max_ratio,
            "{}: SQL Server took {sqlserver:?}, Snowflake {baseline:?}",
            test_case.description
        );
    }
}

#[test]
#[ignore = "timing"]
fn given_sql_growing_in_size_when_formatting_then_time_grows_linearly() {
    let test_cases = [test_types::ScalingSqlTestCase {
        description: "a BigQuery dashed name of many segments, whose touching run the guide marks once",
        dialect: FormatDialect::BigQuery,
        sql: |segments| format!("select x from {}.ds.t", vec!["a"; segments].join("-")),
        sizes: (2_000, 8_000),
        expected_max_ratio: 8,
    }];
    for test_case in test_cases {
        let (small, large) = test_case.sizes;
        let laid_out = format_sql(
            &(test_case.sql)(large),
            &FormatOptions::new(test_case.dialect),
        );
        assert!(laid_out.is_ok(), "{}: refused", test_case.description);
        let short = format_time(&(test_case.sql)(small), test_case.dialect);
        let long = format_time(&(test_case.sql)(large), test_case.dialect);
        assert!(
            long <= short * test_case.expected_max_ratio,
            "{}: {large} took {long:?}, {small} took {short:?}",
            test_case.description
        );
    }
}
