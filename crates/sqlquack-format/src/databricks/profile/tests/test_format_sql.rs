use crate::databricks::profile::constants::{
    HINT_FAILURE, LINE_CONTINUATION_FAILURE, OPAQUE_STATEMENT_FAILURE,
};
use crate::databricks::profile::tests::test_types;
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::format_outcome;

#[test]
fn given_databricks_sql_when_formatting_then_it_is_laid_out_or_refused() {
    let test_cases = [
        test_types::FormatSqlTestCase {
            description: "Databricks lays out a query through its own scanner and parser",
            sql: "select a from t",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Ok("SELECT a\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "a Databricks CLUSTER BY starts its own clause line",
            sql: "select a, b from t cluster by a, b",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Ok("SELECT\n  a,\n  b\nFROM t\nCLUSTER BY\n  a,\n  b"),
        },
        test_types::FormatSqlTestCase {
            description: "Databricks DISTRIBUTE BY and SORT BY each start a clause line",
            sql: "select a from t distribute by a sort by a",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Ok("SELECT a\nFROM t\nDISTRIBUTE BY a\nSORT BY a"),
        },
        test_types::FormatSqlTestCase {
            description: "a Databricks path, bracket path and try-cast stay touching",
            sql: "select c:a.b, c:['k'], '1'?::INT",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Ok("SELECT\n  c:a.b,\n  c:['k'],\n  '1'?::INT"),
        },
        test_types::FormatSqlTestCase {
            description: "a Databricks hint is refused",
            sql: "select /*+ BROADCAST(t) */ a from t",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(HINT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a Databricks `--` comment continued by a backslash is refused",
            sql: "select a -- note \\\nfrom t",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(LINE_CONTINUATION_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a nested Databricks block comment stays whole",
            sql: "select /* a /* b */ c */ x from t",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Ok("SELECT /* a /* b */ c */\n  x\nFROM t"),
        },
        test_types::FormatSqlTestCase {
            description: "ADD of resources, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "ADD FILE f1",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "LIST of resources, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "LIST JARS",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "RESTORE, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "RESTORE t1 VERSION AS OF 1",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "DENY, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "DENY SELECT ON TABLE t1 TO u1",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "CACHE SELECT as documented, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "CACHE SELECT c1 FROM t1",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "REFRESH of a resource path, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "REFRESH 'p1'",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "DESCRIBE of a column, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "DESCRIBE TABLE t1 c1",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "SHOW of a partition, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "SHOW PARTITIONS t1 PARTITION ( c1 = 1 )",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a Hive row format, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "CREATE TABLE t1 ( c1 INT ) ROW FORMAT DELIMITED",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a Hive storage format, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "CREATE TABLE t1 ( c1 INT ) STORED AS PARQUET",
            line_width: None,
            dialect: FormatDialect::Databricks,
            expected_outcome: Err(OPAQUE_STATEMENT_FAILURE),
        },
        test_types::FormatSqlTestCase {
            description: "a constraint's validation words, read as a tree not yet reviewed for layout, is refused as opaque",
            sql: "ALTER TABLE t1 ADD CONSTRAINT k1 CHECK ( c1 > 0 ) ENABLE",
            line_width: None,
            dialect: FormatDialect::Databricks,
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
