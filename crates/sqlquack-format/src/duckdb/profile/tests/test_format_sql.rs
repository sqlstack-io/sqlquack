use crate::duckdb::profile::tests::test_types;
use crate::format::constants::{DEPTH_FAILURE, JOIN_FAILURE, PARSE_FAILURE, TYPE_DEPTH_FAILURE};
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::{format_outcome, refusal};

#[test]
fn given_sql_whose_spacing_carries_meaning_when_formatting_then_it_is_kept_or_refused() {
    let test_cases = [
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
            refusal(&(test_case.sql)(), FormatDialect::DuckDB),
            test_case.expected_refusal.map(str::to_string),
            "{}",
            test_case.description
        );
    }
}
