use crate::checking::errors::CheckStage;
use crate::checking::tests::helpers::{failed_stage, nested_union};
use crate::checking::tests::test_types::CheckTestCase;

#[test]
fn given_printed_sql_when_duckdb_checks_it_then_only_one_bindable_statement_passes() {
    let test_cases = [
        CheckTestCase {
            description: "a lambda parameter read with a dot could be captured by a relation alias",
            sql: "SELECT list_transform([struct_pack(l := 1)], lambda p: p.l) FROM \"ORDERS\" AS p"
                .to_owned(),
            expected_stage: Some(CheckStage::CapturedParameter),
        },
        CheckTestCase {
            description: "bracket access, quoted names and string text are not dotted reads",
            sql: "SELECT list_transform([struct_pack(l := 1)], lambda p: p['l']), \"p\".\"ID\", \
                  'p.l' FROM \"ORDERS\" AS \"p\""
                .to_owned(),
            expected_stage: None,
        },
        CheckTestCase {
            description: "a query over a known relation binds",
            sql: "SELECT \"ORDERS\".\"ID\" + 1 AS \"N\" FROM \"ORDERS\" AS \"ORDERS\"".to_owned(),
            expected_stage: None,
        },
        CheckTestCase {
            description: "a semi-structured column is held as JSON",
            sql: "SELECT json_extract_string(\"DETAILS\", '$.a') FROM \"ORDERS\"".to_owned(),
            expected_stage: None,
        },
        CheckTestCase {
            description: "a nanosecond timestamp column binds",
            sql: "SELECT epoch_ns(\"PLACED\") FROM \"ORDERS\"".to_owned(),
            expected_stage: None,
        },
        CheckTestCase {
            description: "a CTE named like no relation binds",
            sql: "WITH \"INPUT\" AS (SELECT 1 AS \"X\") SELECT \"X\" FROM \"INPUT\"".to_owned(),
            expected_stage: None,
        },
        CheckTestCase {
            description: "a column without a DuckDB type is left out",
            sql: "SELECT \"NOTE\" FROM \"ORDERS\"".to_owned(),
            expected_stage: Some(CheckStage::Bind),
        },
        CheckTestCase {
            description: "an unknown column does not bind",
            sql: "SELECT \"MISSING\" FROM \"ORDERS\"".to_owned(),
            expected_stage: Some(CheckStage::Bind),
        },
        CheckTestCase {
            description: "an unknown relation does not bind",
            sql: "SELECT 1 FROM \"CUSTOMERS\"".to_owned(),
            expected_stage: Some(CheckStage::Bind),
        },
        CheckTestCase {
            description: "a type error does not bind",
            sql: "SELECT date_trunc('day', \"ID\") FROM \"ORDERS\"".to_owned(),
            expected_stage: Some(CheckStage::Bind),
        },
        CheckTestCase {
            description: "invalid syntax does not parse",
            sql: "SELEC 1".to_owned(),
            expected_stage: Some(CheckStage::Parse),
        },
        CheckTestCase {
            description: "two statements are refused",
            sql: "SELECT 1; SELECT 2".to_owned(),
            expected_stage: Some(CheckStage::StatementCount),
        },
        CheckTestCase {
            description: "a set operation nested beyond JSON parsers' depth limits parses",
            sql: nested_union(100),
            expected_stage: None,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            failed_stage(&test_case.sql),
            test_case.expected_stage,
            "{}",
            test_case.description
        );
    }
}
