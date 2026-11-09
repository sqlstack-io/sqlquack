use crate::format::constants::{DEPTH_FAILURE, PARSE_FAILURE, TYPE_DEPTH_FAILURE};
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::{format_outcome, refusal};
use crate::postgres::profile::constants::SCANNER_MISMATCH_FAILURE;
use crate::postgres::profile::tests::test_types;

#[test]
fn given_sql_whose_spacing_carries_meaning_when_formatting_then_it_is_kept_or_refused() {
    let test_cases = [
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
            description: "a chain of 5,000 added terms is refused by the PostgreSQL parser's depth limit",
            sql: || format!("SELECT {}", vec!["1"; 5_000].join(" + ")),
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
            description: "a chain of 512 added terms, at the limit, is laid out",
            sql: || format!("SELECT {}", vec!["1"; 512].join(" + ")),
            expected_refusal: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            refusal(&(test_case.sql)(), FormatDialect::PostgreSQL),
            test_case.expected_refusal.map(str::to_string),
            "{}",
            test_case.description
        );
    }
}
