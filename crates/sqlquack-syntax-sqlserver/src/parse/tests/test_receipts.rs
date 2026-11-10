use crate::parse::tests::helpers::disagreements;
use crate::parse::tests::test_types::ReceiptsTestCase;

#[test]
fn given_recorded_sql_server_verdicts_when_parsing_each_statement_then_the_parser_agrees() {
    let test_cases = [
        ReceiptsTestCase {
            description: "FOR XML, FOR JSON and FOR BROWSE under SQL Server 2022 PARSEONLY",
            receipts: include_str!("fixtures/result_clause_receipts.jsonl"),
            expected_disagreements: Vec::new(),
        },
        ReceiptsTestCase {
            description: "FOR SYSTEM_TIME under SQL Server 2022 PARSEONLY, its variables declared, since an undeclared one fails binding (137) rather than syntax",
            receipts: include_str!("fixtures/system_time_receipts.jsonl"),
            expected_disagreements: Vec::new(),
        },
        ReceiptsTestCase {
            description: "TABLESAMPLE under SQL Server 2022 PARSEONLY",
            receipts: include_str!("fixtures/tablesample_receipts.jsonl"),
            expected_disagreements: Vec::new(),
        },
        ReceiptsTestCase {
            description: "join hints under SQL Server 2022 PARSEONLY",
            receipts: include_str!("fixtures/join_hint_receipts.jsonl"),
            expected_disagreements: Vec::new(),
        },
        ReceiptsTestCase {
            description: "table hints under SQL Server 2022 PARSEONLY: every pair of one-word hints, and each one-word hint after an alias without WITH, where an unknown word is refused (321)",
            receipts: include_str!("fixtures/table_hint_receipts.jsonl"),
            expected_disagreements: Vec::new(),
        },
        ReceiptsTestCase {
            description: "table variables under SQL Server 2022 PARSEONLY, each after its declaration, since an undeclared one fails binding (1087) rather than syntax",
            receipts: include_str!("fixtures/table_variable_receipts.jsonl"),
            expected_disagreements: Vec::new(),
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            disagreements(test_case.receipts),
            test_case.expected_disagreements,
            "{}",
            test_case.description
        );
    }
}
