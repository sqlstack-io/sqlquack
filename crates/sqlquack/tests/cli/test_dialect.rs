use crate::helpers::{convert_outcome, run_convert};
use crate::test_types::ConvertTestCase;

#[test]
fn given_databricks_sql_when_converting_with_the_dialect_then_proven_forms_convert_and_others_refuse()
 {
    let test_cases = [
        ConvertTestCase {
            description: "a function the Databricks rules prove converts",
            sql: "SELECT abs(-7) AS r",
            flags: &["--dialect", "databricks"],
            mention: "sqlquack convert: done, converted\n",
            expected_outcome: convert_outcome(0, true),
        },
        ConvertTestCase {
            description: "a function no Databricks rule proves is refused by name",
            sql: "SELECT sin(1.0) AS r",
            flags: &["--dialect", "databricks"],
            mention: "no rule converts the function SIN",
            expected_outcome: convert_outcome(1, false),
        },
        ConvertTestCase {
            description: "a project is rejected rather than ignored",
            sql: "SELECT 1 AS r",
            flags: &["--dialect", "databricks", "--project", "."],
            mention: "--dialect databricks takes no project, schema, session",
            expected_outcome: convert_outcome(2, false),
        },
        ConvertTestCase {
            description: "a Snowflake session option is rejected rather than ignored",
            sql: "SELECT 1 AS r",
            flags: &["--dialect", "databricks", "--session", "TIMEZONE=UTC"],
            mention: "--dialect databricks takes no project, schema, session",
            expected_outcome: convert_outcome(2, false),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_convert(test_case.sql, test_case.flags, test_case.mention),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
