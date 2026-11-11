use crate::format::constants::{
    DEPTH_FAILURE, PARSE_FAILURE, SEPARATION_FAILURE, TYPE_DEPTH_FAILURE,
    UNSUPPORTED_SYNTAX_FAILURE,
};
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::{format_outcome, refusal};
use crate::snowflake::profile::tests::test_types;

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
            expected_outcome: Ok("SELECT a\nFROM t\nJOIN u ON t.id = u.id"),
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
            description: "a path after a qualified column spelled like a keyword is laid out, not taken for a bind variable",
            sql: "select s.statement:gross::int as g, s.body:ts_ms::number(38, 0) as t from src as s",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "SELECT\n  s.statement:gross::int AS g,\n  s.body:ts_ms::number(38, 0) AS t\nFROM src AS s",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "a positional bind variable is still left as written",
            sql: "select x from t where id = :1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(UNSUPPORTED_SYNTAX_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a named bind variable after a keyword is still left as written",
            sql: "select x from t where id in (:ids) and body = :body",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Err(UNSUPPORTED_SYNTAX_FAILURE),
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
            expected_outcome: Ok("SELECT *\nFROM t PIVOT(SUM(v) FOR k IN (1+1, 2-1))\nORDER BY 1"),
        },
        test_types::FormatSqlTestCase {
            description: "PIVOT IN concatenations keep their spacing",
            sql: "SELECT * FROM t PIVOT (SUM(v) FOR k IN ('a'||'b', 'c')) ORDER BY 1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok(
                "SELECT *\nFROM t PIVOT(SUM(v) FOR k IN ('a'||'b', 'c'))\nORDER BY 1",
            ),
        },
        test_types::FormatSqlTestCase {
            description: "PIVOT IN values keep a spaced unary minus",
            sql: "SELECT * FROM t PIVOT (SUM(v) FOR k IN (- 1, 2)) ORDER BY 1",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("SELECT *\nFROM t PIVOT(SUM(v) FOR k IN (- 1, 2))\nORDER BY 1"),
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
            description: "an UPDATE's SET assignments written touching are spaced, and a comparison in SET stays one",
            sql: "update t set a=1, b=c=d where b=2",
            line_width: None,
            dialect: FormatDialect::Snowflake,
            expected_outcome: Ok("UPDATE t SET a = 1, b = c = d WHERE b = 2"),
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
            description: "1,500 parentheses around a FROM source are refused by the parser's nesting guard",
            sql: || format!("SELECT * FROM {}t{}", "(".repeat(1_500), ")".repeat(1_500)),
            expected_refusal: Some(PARSE_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 parentheses around an expression are refused by the parser's nesting guard",
            sql: || format!("SELECT {}1{}", "(".repeat(1_500), ")".repeat(1_500)),
            expected_refusal: Some(PARSE_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "1,500 nested subqueries are refused by the parser's depth guard",
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
            sql: || format!("SELECT {}TRUE", "NOT ".repeat(1_500)),
            expected_refusal: Some(PARSE_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a chain of 5,000 added terms is refused before its tree is walked",
            sql: || format!("SELECT {}", vec!["1"; 5_000].join(" + ")),
            expected_refusal: Some(DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a chain of 513 added terms, one level past the limit, is refused",
            sql: || format!("SELECT {}", vec!["1"; 513].join(" + ")),
            expected_refusal: Some(DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to a type with 300 `[]` suffixes is refused before parsing",
            sql: || format!("SELECT CAST(NULL AS INT{})", "[]".repeat(300)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to a type with 300 `[3]` suffixes is refused before parsing",
            sql: || format!("SELECT CAST(NULL AS INT{})", "[3]".repeat(300)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a column of a type with 65 `[]` suffixes, one past the limit, is refused",
            sql: || format!("CREATE TABLE t (c INT{})", "[]".repeat(65)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a column of a type with 64 `[]` suffixes, at the limit, is laid out",
            sql: || format!("CREATE TABLE t (c INT{})", "[]".repeat(64)),
            expected_refusal: None,
        },
        test_types::DeepSqlTestCase {
            description: "a DuckDB `::` cast to a type with 300 `[]` suffixes is refused before parsing",
            sql: || format!("SELECT NULL::INT{}", "[]".repeat(300)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a type alternating angle brackets with suffix runs, depth 182, is refused",
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
            sql: || format!("SELECT CAST(NULL AS ARRAY<ARRAY<INT{}>>)", "[]".repeat(63)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to 2 angle brackets around 62 suffixes, depth 64, is laid out",
            sql: || format!("SELECT CAST(NULL AS ARRAY<ARRAY<INT{}>>)", "[]".repeat(62)),
            expected_refusal: None,
        },
        test_types::DeepSqlTestCase {
            description: "a struct of 100 one-suffix fields stays shallow and is laid out",
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
            sql: || {
                let run = "[]".repeat(40);
                format!("CREATE TABLE t (c \"STRUCT\"<a \"ARRAY\"<INT{run}>{run}>{run})")
            },
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a column alternating STRUCT(...), MAP(...) and ARRAY<...> with suffix runs is refused",
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
            sql: || format!("SELECT CAST(NULL AS ARRAY(ARRAY(INT{})))", "[]".repeat(63)),
            expected_refusal: Some(TYPE_DEPTH_FAILURE),
        },
        test_types::DeepSqlTestCase {
            description: "a cast to 2 parenthesised constructors around 62 suffixes, depth 64, is laid out",
            sql: || format!("SELECT CAST(NULL AS ARRAY(ARRAY(INT{})))", "[]".repeat(62)),
            expected_refusal: None,
        },
        test_types::DeepSqlTestCase {
            description: "a cast to 300 nested ARRAY<...> types is refused before parsing",
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
            sql: || format!("SELECT {}", vec!["1"; 512].join(" + ")),
            expected_refusal: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            refusal(&(test_case.sql)(), FormatDialect::Snowflake),
            test_case.expected_refusal.map(str::to_string),
            "{}",
            test_case.description
        );
    }
}
