use crate::databricks::profile::constants::UNSUPPORTED_FAILURE;
use crate::databricks::profile::tests::test_types;
use crate::format::models::FormatDialect;
use crate::format::tests::helpers::format_outcome;

#[test]
fn given_databricks_sql_when_formatting_then_it_is_refused_until_it_has_a_format_gate() {
    let test_cases = [test_types::FormatSqlTestCase {
        description: "Databricks is refused until it has a format gate",
        sql: "SELECT a FROM t",
        line_width: None,
        dialect: FormatDialect::Databricks,
        expected_outcome: Err(UNSUPPORTED_FAILURE),
    }];
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
