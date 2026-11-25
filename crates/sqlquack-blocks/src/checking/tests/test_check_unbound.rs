use crate::checking::errors::CheckStage;
use crate::checking::main::check_unbound::check_unbound;
use crate::checking::tests::test_types::CheckTestCase;

#[test]
fn given_printed_sql_when_checked_without_binding_then_only_its_lexical_failures_refuse() {
    let test_cases = [
        CheckTestCase {
            description: "an unknown column is left to the caller that runs the SQL",
            sql: "SELECT \"MISSING\" FROM \"ORDERS\"".to_owned(),
            expected_stage: None,
        },
        CheckTestCase {
            description: "an unknown relation is left to the caller that runs the SQL",
            sql: "SELECT 1 FROM \"CUSTOMERS\"".to_owned(),
            expected_stage: None,
        },
        CheckTestCase {
            description: "a type error is left to the caller that runs the SQL",
            sql: "SELECT date_trunc('day', \"ID\") FROM \"ORDERS\"".to_owned(),
            expected_stage: None,
        },
        CheckTestCase {
            description: "a lambda parameter read with a dot is still refused",
            sql: "SELECT list_transform([struct_pack(l := 1)], lambda p: p.l) FROM \"ORDERS\" AS p"
                .to_owned(),
            expected_stage: Some(CheckStage::CapturedParameter),
        },
        CheckTestCase {
            description: "two statements are still refused",
            sql: "SELECT 1; SELECT 2".to_owned(),
            expected_stage: Some(CheckStage::StatementCount),
        },
    ];
    for test_case in test_cases {
        let stage = check_unbound(&test_case.sql)
            .err()
            .map(|failure| failure.stage);

        assert_eq!(stage, test_case.expected_stage, "{}", test_case.description);
    }
}
