use crate::bigquery::profile::constants::OPAQUE_STATEMENT_FAILURE;
use crate::bigquery::profile::tests::test_types;
use crate::format::constants::SEPARATION_FAILURE;
use crate::format::main::format_sql::format_sql;
use crate::format::models::FormatDialect;
use crate::format::models::FormatOptions;
use crate::format::tests::helpers::{format_outcome, format_time};

#[test]
fn given_sql_whose_spacing_carries_meaning_when_formatting_then_it_is_kept_or_refused() {
    let test_cases = [
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
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
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
