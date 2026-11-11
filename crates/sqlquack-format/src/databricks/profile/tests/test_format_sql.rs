use crate::databricks::profile::constants::{HINT_FAILURE, LINE_CONTINUATION_FAILURE};
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
