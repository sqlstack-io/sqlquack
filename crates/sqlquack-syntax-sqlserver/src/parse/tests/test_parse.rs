//! Verdicts and trees of the SQL Server parser, each confirmed against SQL Server 2022 under PARSEONLY.

use serde_json::Value;

use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::helpers::{identifier_texts, verdict};
use crate::parse::tests::test_types::{
    ExpectedVerdict, RangeTestCase, SpanTestCase, ValueTestCase, VerdictTestCase,
};

#[test]
fn given_sql_when_parsing_then_the_verdict_is_sql_servers() {
    let test_cases = [
        VerdictTestCase {
            description: "TRIM takes a position and characters before FROM",
            sql: "SELECT TRIM(LEADING 'a' + 'b' FROM 'c' + 'd')",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "parentheses around a lone table are refused (102)",
            sql: "SELECT * FROM (t)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "parentheses around an aliased derived table are refused (102)",
            sql: "SELECT * FROM ((SELECT 1 x) d)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "parentheses hold a join whose first source is an aliased derived table",
            sql: "SELECT * FROM ((SELECT 1 x) d JOIN t ON 1 = 1)",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "parentheses hold a cross join of a derived table aliased with AS",
            sql: "SELECT * FROM (((SELECT 1 x) AS d CROSS JOIN t))",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a derived table's query may itself be parenthesised",
            sql: "SELECT * FROM ((SELECT 1 x)) d",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a derived table's query may be a set operation of parenthesised queries",
            sql: "SELECT * FROM ((SELECT 1 x) UNION (SELECT 2)) u",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a joined table keeps any number of parentheses",
            sql: "SELECT * FROM ((a CROSS JOIN b))",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "TRIM with a position needs characters (156)",
            sql: "SELECT TRIM(LEADING FROM ' b')",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "TRIM with a position takes no comma form (102)",
            sql: "SELECT TRIM(LEADING 'a', 'b')",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "TOP takes a scalar subquery in its parentheses",
            sql: "SELECT TOP (SELECT 1) * FROM t",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "TOP PERCENT takes a set operation as its subquery",
            sql: "SELECT TOP (SELECT 1 UNION SELECT 2) PERCENT * FROM t",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "TOP takes an expression that starts with a parenthesised subquery",
            sql: "SELECT TOP ((SELECT 1) + 1) * FROM t",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "TOP PERCENT takes a product led by a parenthesised subquery",
            sql: "SELECT TOP ((SELECT 1) * 2) PERCENT * FROM t",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a scalar subquery may end in FOR JSON, which is not read",
            sql: "SELECT (SELECT TOP 5 a, b FROM t ORDER BY b DESC FOR JSON PATH) AS j FROM x",
            expected_verdict: ExpectedVerdict::Gap("for_xml_json"),
        },
        VerdictTestCase {
            description: "a derived table may end in FOR JSON, which is not read",
            sql: "SELECT j FROM (SELECT a AS j FROM t FOR JSON PATH) AS x(j)",
            expected_verdict: ExpectedVerdict::Gap("for_xml_json"),
        },
        VerdictTestCase {
            description: "an EXISTS query may end in FOR XML, which is not read",
            sql: "SELECT 1 WHERE EXISTS (SELECT 1 x FOR XML PATH)",
            expected_verdict: ExpectedVerdict::Gap("for_xml_json"),
        },
        VerdictTestCase {
            description: "a subquery takes no OPTION clause (156)",
            sql: "SELECT (SELECT 1 x OPTION (MAXDOP 1)) AS j",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a date part may be a quoted identifier",
            sql: "SELECT DATEPART(\"dd\", a), DATEADD([dd], 1, a) FROM t",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a date part is never a string (1023)",
            sql: "SELECT DATEPART('dd', a) FROM t",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a date part must be one the engine knows (155)",
            sql: "SELECT DATEPART(frob, a) FROM t",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "an alias takes one old-style hint without WITH",
            sql: "SELECT * FROM t AS s(nolock) JOIN u v (NOLOCK) ON 1 = 1",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "an old-style hint is one word (1018)",
            sql: "SELECT * FROM t s (nolock, readpast)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "an old-style hint must be one the engine knows (321)",
            sql: "SELECT * FROM t s (frobnicate)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "JSON_OBJECT pairs end in one null clause",
            sql: "SELECT JSON_OBJECT('a':1, 'b':NULL ABSENT ON NULL)",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "JSON_OBJECT pairs are written with a colon (102)",
            sql: "SELECT JSON_OBJECT('a', 1)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a JSON null clause comes after the last pair only (102)",
            sql: "SELECT JSON_OBJECT('a':1 NULL ON NULL, 'b':2)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a JSON_ARRAY null clause follows a value without a comma (156)",
            sql: "SELECT JSON_ARRAY(1, NULL ON NULL)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a one-part call must name a built-in function (195)",
            sql: "SELECT LENGTH('a')",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a schema-qualified call names a user function",
            sql: "SELECT id, __sqlbuild.udf_scaled_amount(amount, 2) AS scaled FROM stg",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a delimited one-part name is never a call (102)",
            sql: "SELECT [len]('a')",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "COALESCE needs two operands (102)",
            sql: "SELECT COALESCE(1)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "DOUBLE alone names no type (102)",
            sql: "SELECT CAST(1 AS DOUBLE)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "DOUBLE PRECISION is a type",
            sql: "DECLARE @a DOUBLE PRECISION",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a reserved word names no part of a multipart name (156)",
            sql: "SELECT t.select FROM t",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a table function takes DEFAULT arguments and an alias",
            sql: "SELECT * FROM dbo.f(DEFAULT, 1) AS x (a)",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a table function takes no table hints (102)",
            sql: "SELECT * FROM dbo.f(1) WITH (NOLOCK)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "OPENJSON with typed columns, paths and AS JSON",
            sql: "SELECT * FROM t CROSS APPLY OPENJSON(t.j) \
                  WITH (a int '$.a', b nvarchar(max) '$.b' AS JSON, c int) AS k",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "OPENJSON takes no alias column list (102)",
            sql: "SELECT * FROM OPENJSON(N'[]') j (x, y)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "OPENJSON names only the built-in rowset function (102)",
            sql: "SELECT * FROM dbo.OPENJSON(N'[]')",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a scalar function takes its WITH options",
            sql: "CREATE FUNCTION dbo.f() RETURNS int \
                  WITH SCHEMABINDING, RETURNS NULL ON NULL INPUT, EXECUTE AS CALLER \
                  AS BEGIN RETURN 1 END",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "an unknown function option is refused (195)",
            sql: "CREATE FUNCTION dbo.f() RETURNS int WITH FOO AS BEGIN RETURN 1 END",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "an inline table function's body may be a parenthesised WITH query",
            sql: "CREATE FUNCTION dbo.f() RETURNS TABLE AS RETURN (WITH c AS (SELECT 1 AS a) SELECT a FROM c)",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "an inline table function has no BEGIN ... END body (102)",
            sql: "CREATE FUNCTION dbo.f() RETURNS TABLE AS BEGIN RETURN SELECT 1 AS a END",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a multi-statement table function is a gap",
            sql: "CREATE FUNCTION dbo.f() RETURNS @t TABLE (a int) AS BEGIN INSERT @t VALUES (1); RETURN END",
            expected_verdict: ExpectedVerdict::Gap("multi_statement_table_function"),
        },
        VerdictTestCase {
            description: "a parenthesised subquery may open a comparison operand",
            sql: "SELECT 1 WHERE ((SELECT 1) > 0)",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "TRY_CONVERT takes a type, a value and a style",
            sql: "SELECT TRY_CONVERT(int, '1', 2)",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "TRY_CONVERT takes the type first (102)",
            sql: "SELECT TRY_CONVERT('1', int)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "EXEC text is strings and variables only (102)",
            sql: "EXEC ('a' + 1)",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "EXEC options need a word after WITH (102)",
            sql: "EXEC p1 WITH",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a MERGE target needs no alias before USING",
            sql: "MERGE t1 USING t2 ON t1.c1 = t2.c1 WHEN MATCHED THEN UPDATE SET c1 = 1;",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "touching angle brackets are a shift, a gap",
            sql: "SELECT 1 << 2 + 1",
            expected_verdict: ExpectedVerdict::Gap("shift_operator"),
        },
        VerdictTestCase {
            description: "separated angle brackets are no operator (102)",
            sql: "SELECT 1 < < 2",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a method of a value is a gap",
            sql: "SELECT CAST(N'' AS XML).value('.', 'int')",
            expected_verdict: ExpectedVerdict::Gap("method_call"),
        },
        VerdictTestCase {
            description: "a procedure takes parameters with defaults and OUTPUT, and a BEGIN ... END body",
            sql: "CREATE PROCEDURE dbo.p @a int, @b varchar(10) = 'x' OUTPUT AS BEGIN SELECT @a END",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a procedure's body is every statement to the end of the batch",
            sql: "CREATE OR ALTER PROC p AS SELECT 1 SELECT 2",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a procedure's parameters may be parenthesised, and WITH RECOMPILE goes before AS",
            sql: "CREATE PROCEDURE p (@a int) WITH RECOMPILE AS BEGIN SELECT @a END;",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a procedure's body may be empty",
            sql: "CREATE PROCEDURE p AS",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a procedure after another statement is refused (111)",
            sql: "SELECT 1 CREATE PROCEDURE p AS SELECT 1",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "BREAK and CONTINUE are statements of a loop",
            sql: "WHILE 1 = 1 BEGIN BREAK CONTINUE END",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a THROW without arguments rethrows in CATCH",
            sql: "BEGIN TRY SELECT 1 END TRY BEGIN CATCH THROW END CATCH",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a THROW with arguments is a gap",
            sql: "THROW 50000, 'x', 1",
            expected_verdict: ExpectedVerdict::Gap("throw"),
        },
        VerdictTestCase {
            description: "BREAK outside a WHILE is refused (135)",
            sql: "BREAK",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "CONTINUE outside a WHILE is refused (136)",
            sql: "CONTINUE",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "BREAK may be a WHILE's own statement",
            sql: "WHILE 1 = 1 BREAK",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a THROW without arguments outside CATCH is refused (10704)",
            sql: "THROW",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a THROW without arguments may be nested in a CATCH block's IF",
            sql: "BEGIN TRY SELECT 1 END TRY BEGIN CATCH IF 1 = 1 THROW END CATCH",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a procedure's parentheses hold at least one parameter (102)",
            sql: "CREATE PROCEDURE p () AS SELECT 1",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a parameter's default is a constant, not an expression (102)",
            sql: "CREATE PROCEDURE p @a int = 1 + 1 AS SELECT @a",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a parameter's default may not be signed with + (102)",
            sql: "CREATE PROCEDURE p @a int = +1 AS SELECT @a",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a parameter's default may not be parenthesised (102)",
            sql: "CREATE PROCEDURE p @a int = (1) AS SELECT @a",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a parameter's default may not be a call (102)",
            sql: "CREATE PROCEDURE p @a datetime = getdate() AS SELECT @a",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a parameter's default may not be a reserved word (156)",
            sql: "CREATE PROCEDURE p @a int = select AS SELECT 1",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a parameter's default may be a word that is not reserved, such as date",
            sql: "CREATE PROCEDURE p @a varchar(9) = date AS SELECT 1",
            expected_verdict: ExpectedVerdict::Tree,
        },
        VerdictTestCase {
            description: "a parameter's default may not be a variable (137)",
            sql: "CREATE PROCEDURE p @a int = @b AS SELECT @a",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a parameter's default may be a negative number, a binary or money literal, a national string, a bare or bracketed word, or DEFAULT",
            sql: "CREATE PROCEDURE p @a int = - 1, @b varbinary(9) = 0x1F, @c money = $5, @d nvarchar(9) = N'x', @e varchar(9) = abc, @f varchar(9) = [abc], @g varchar(9) = default AS SELECT @a",
            expected_verdict: ExpectedVerdict::Tree,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            verdict(test_case.sql),
            test_case.expected_verdict,
            "{}: {}",
            test_case.description,
            test_case.sql
        );
    }
}

#[test]
fn given_statements_when_parsing_then_each_records_its_text_and_depth() {
    let test_cases = [
        RangeTestCase {
            description: "statements without semicolons, a branch, a block and a loop",
            sql: "select 1 select 2; if 1 = 1 begin select 3 end else while 1 = 0 break;",
            expected_statements: &[
                ("select 1", 0),
                ("select 2;", 0),
                ("if 1 = 1 begin select 3 end else while 1 = 0 break;", 0),
                ("begin select 3 end", 1),
                ("select 3", 2),
                ("while 1 = 0 break;", 1),
                ("break;", 2),
            ],
        },
        RangeTestCase {
            description: "a procedure and the statements of its body",
            sql: "create procedure p @a int as select @a; select 2",
            expected_statements: &[
                ("create procedure p @a int as select @a; select 2", 0),
                ("select @a;", 1),
                ("select 2", 1),
            ],
        },
    ];
    for test_case in test_cases {
        let parsed = parse_strict(test_case.sql).expect(test_case.description);
        let statements: Vec<(&str, usize)> = parsed
            .ranges
            .iter()
            .map(|range| (&test_case.sql[range.start..range.end], range.depth))
            .collect();
        assert_eq!(
            statements, test_case.expected_statements,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_names_when_parsing_then_every_identifier_records_the_range_of_its_own_text() {
    let test_cases = [SpanTestCase {
        description: "aliases, alias column lists, CTE names, #temp names, OPENJSON columns and function name parts",
        sql: "WITH cte (k) AS (SELECT 1 AS [type]) \
            SELECT t.type AS type, dbo.g(t.type) AS g, x.type \
            FROM #tmp AS t CROSS APPLY dbo.f(1) AS x (type) \
            CROSS APPLY OPENJSON(t.j) WITH (type int '$.type') AS y JOIN cte ON cte.k = t.type",
        expected_identifiers: 28,
    }];
    for test_case in test_cases {
        let (names, texts): (Vec<String>, Vec<Option<String>>) =
            identifier_texts(test_case.sql).into_iter().unzip();
        assert_eq!(
            names.len(),
            test_case.expected_identifiers,
            "{}: {names:?}",
            test_case.description
        );
        assert_eq!(
            texts,
            names.into_iter().map(Some).collect::<Vec<Option<String>>>(),
            "{}",
            test_case.description
        );
    }
}

/// SQLBuild's analysis-only markers: SQL Server gives each 195; the parser lowers two shapes.
#[test]
fn given_sqlbuild_type_markers_when_parsing_then_well_formed_ones_are_annotations() {
    let test_cases = [
        ValueTestCase {
            description: "a type marker asserts its type around its expression",
            sql: "SELECT __polyglot_typed(t.x, 'varchar(5)') AS v FROM t",
            pointer: "/0/select/expressions/0/alias/this/annotated/type_assertion/data_type",
            expected_json: r#""var_char""#,
        },
        ValueTestCase {
            description: "the marker name is compared without case",
            sql: "SELECT __POLYGLOT_TYPED(x, 'BIGINT') FROM t",
            pointer: "/0/select/expressions/0/annotated/type_assertion/data_type",
            expected_json: r#""big_int""#,
        },
        ValueTestCase {
            description: "a typed-like marker keeps the expression it is typed like",
            sql: "SELECT __polyglot_typed_like(x, y + 1) FROM t",
            pointer: "/0/select/expressions/0/annotated/type_like/add/right/literal/value",
            expected_json: r#""1""#,
        },
    ];
    for test_case in test_cases {
        let parsed = parse_strict(test_case.sql).expect(test_case.description);
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
    let refused = [
        VerdictTestCase {
            description: "a type text T-SQL cannot read leaves a call to no built-in (195)",
            sql: "SELECT __polyglot_typed(x, 'int garbage') FROM t",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a marker of one argument is no built-in (195)",
            sql: "SELECT __polyglot_typed(x) FROM t",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
        VerdictTestCase {
            description: "a bracketed marker name is a call of no schema (102)",
            sql: "SELECT [__polyglot_typed](x, 'INT') FROM t",
            expected_verdict: ExpectedVerdict::Refused { syntax: true },
        },
    ];
    for test_case in refused {
        assert_eq!(
            verdict(test_case.sql),
            test_case.expected_verdict,
            "{}",
            test_case.description
        );
    }
}
