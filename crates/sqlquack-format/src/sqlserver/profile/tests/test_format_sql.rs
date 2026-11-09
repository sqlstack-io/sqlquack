use crate::format::constants::SEPARATION_FAILURE;
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::{format_outcome, format_time, refusal};
use crate::sqlserver::profile::constants::{BATCH_SEPARATOR_FAILURE, OPAQUE_STATEMENT_FAILURE};
use crate::sqlserver::profile::tests::test_types;

#[test]
fn given_sql_whose_spacing_carries_meaning_when_formatting_then_it_is_kept_or_refused() {
    let test_cases = [
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
            description: "a comparison of a SQL Server variable with a literal written touching is spaced",
            sql: "select 1 where @a=1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT 1\nWHERE @a = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a comparison of a literal with a SQL Server variable written touching is spaced",
            sql: "select 1 where 1=@a",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT 1\nWHERE 1 = @a"),
        },
        test_types::FormatSqlTestCase {
            description: "a comparison touching only its literal is spaced",
            sql: "select 1 where @a =1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT 1\nWHERE @a = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "an IF condition over a SQL Server variable written touching is spaced",
            sql: "if @a=1 select 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("IF @a = 1\n  SELECT 1"),
        },
        test_types::FormatSqlTestCase {
            description: "arithmetic on a SQL Server variable written touching is spaced",
            sql: "select @a+1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT @a + 1"),
        },
        test_types::FormatSqlTestCase {
            description: "arithmetic on a SQL Server system variable written touching is spaced",
            sql: "select @@rowcount+1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT @@rowcount + 1"),
        },
        test_types::FormatSqlTestCase {
            description: "comparisons over SQL Server variables inside an AND are spaced",
            sql: "select 1 where @a=1 and b=2",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT 1\nWHERE\n  @a = 1 AND b = 2"),
        },
        test_types::FormatSqlTestCase {
            description: "a DECLARE value written touching its type is spaced",
            sql: "declare @a int=1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("DECLARE @a int = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a procedure parameter default written touching its type is spaced",
            sql: "create procedure p @a int=1 as select 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("CREATE PROCEDURE p @a int = 1\nAS\nSELECT 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a comparison in a procedure body's IF written touching is spaced",
            sql: "create procedure p @a int = 1 as begin if @a=1 select 1 end",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok(
                "CREATE PROCEDURE p @a int = 1\nAS\nBEGIN\n  IF @a = 1\n    SELECT 1\nEND",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a SELECT assignment to a SQL Server variable written touching is spaced",
            sql: "select @a=1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT @a = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a SET assignment written touching is spaced",
            sql: "set @a=1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SET @a = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a SET assignment of arithmetic on the variable written touching is spaced",
            sql: "set @a=@a+1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SET @a = @a + 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a named EXEC argument written touching is spaced",
            sql: "exec p @a=1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("EXEC p @a = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a compound assignment stays refused when touching, since `+=` is one operator",
            sql: "set @a+=1",
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
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a FOREIGN KEY ... REFERENCES table is kept opaque by the parser and refused",
            sql: "create table c (a int, constraint fk foreign key (a) references t (id))",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
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
            expected_outcome: Ok(
                "CREATE FUNCTION f(@a int) RETURNS int\nAS\nBEGIN\n  RETURN @a\nEND",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "PRINT is kept opaque by the parser and refused",
            sql: "print 'x'",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a procedure name keeps its case before its parameter list",
            sql: "create procedure p (@a int) as select @a",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("CREATE PROCEDURE p(@a int)\nAS\nSELECT @a"),
        },
        test_types::FormatSqlTestCase {
            description: "UPDATE STATISTICS is kept opaque by the parser and refused",
            sql: "update statistics t (ix)",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
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
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            format_outcome(test_case.sql, test_case.dialect, test_case.line_width),
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
fn given_sql_server_batches_when_formatting_then_each_statement_starts_its_own_line() {
    let test_cases = [
        test_types::FormatSqlTestCase {
            description: "statements written without semicolons each start a line",
            sql: "select a from t\nupdate t set a = 2\ndelete from t where a = 1",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("SELECT a\nFROM t\nUPDATE t SET a = 2\nDELETE FROM t WHERE a = 1"),
        },
        test_types::FormatSqlTestCase {
            description: "a branch's statement is indented under IF and ELSE, ELSE IF stays on one line, and a block's words line up with IF",
            sql: "if @a = 1 select 1 else if @a = 2 select 2 else begin select 3 end",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok(
                "IF @a = 1\n  SELECT 1\nELSE IF @a = 2\n  SELECT 2\nELSE\nBEGIN\n  SELECT 3\nEND",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "TRY and CATCH blocks indent their statements",
            sql: "begin try select 1 end try begin catch throw end catch",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("BEGIN try\n  SELECT 1\nEND try\nBEGIN catch\n  throw\nEND catch"),
        },
        test_types::FormatSqlTestCase {
            description: "a loop's block holds a branch and BREAK",
            sql: "while @i < 10 begin if @i > 5 break set @i = @i + 1 end",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok(
                "while @i < 10\nBEGIN\n  IF @i > 5\n    break\n  SET @i = @i + 1\nEND",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a procedure's header, AS and body block each start a line, and the body's first word takes keyword case after AS",
            sql: "create procedure dbo.p @a int, @b int = 0 output as begin set nocount on; select a from t where a = @a end",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok(
                "CREATE PROCEDURE dbo.p @a int, @b int = 0 OUTPUT\nAS\nBEGIN\n  SET nocount ON;\n  SELECT a\n  FROM t\n  WHERE a = @a\nEND",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a procedure's body without a block stays at the procedure's indent",
            sql: "create or alter proc p as select 1 select 2",
            line_width: None,
            dialect: FormatDialect::SqlServer,
            expected_outcome: Ok("CREATE OR ALTER PROC p\nAS\nSELECT 1\nSELECT 2"),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            format_outcome(test_case.sql, test_case.dialect, test_case.line_width),
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
            sql: || format!("SELECT * FROM {}t{}", "(".repeat(1_500), ")".repeat(1_500)),
            expected_refusal: Some(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 nested SQL Server subqueries are kept opaque by the parser's depth guard and refused",
            sql: || {
                format!(
                    "{}SELECT 1 AS c{}",
                    "SELECT * FROM (".repeat(1_500),
                    ") AS q".repeat(1_500)
                )
            },
            expected_refusal: Some(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 nested SQL Server NOTs are kept opaque by the parser's depth guard and refused",
            sql: || format!("SELECT * FROM t WHERE {}1 = 1", "NOT ".repeat(1_500)),
            expected_refusal: Some(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a SQL Server chain of 5,000 added terms is kept opaque by the parser's depth guard and refused",
            sql: || format!("SELECT {}", vec!["1"; 5_000].join(" + ")),
            expected_refusal: Some(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "254 nested SQL Server subqueries, under the parser's depth guard, are laid out",
            sql: || {
                format!(
                    "{}SELECT 1 AS c{}",
                    "SELECT * FROM (".repeat(254),
                    ") AS q".repeat(254)
                )
            },
            expected_refusal: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            refusal(&(test_case.sql)(), FormatDialect::SqlServer),
            test_case.expected_refusal.map(str::to_string),
            "{}",
            test_case.description
        );
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
