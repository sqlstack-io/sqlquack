use serde_json::Value;

use crate::parse::tests::helpers::verdict;
use crate::parse::tests::test_types::{ExpectedVerdict, ReceiptsTestCase};

/// Each recorded statement whose verdict contradicts SQL Server's: a tree for a statement it rejects, or a refusal of one it accepts.
fn disagreements(receipts: &str) -> Vec<String> {
    receipts
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|receipt| {
            let sql: &str = receipt["statement"].as_str()?;
            let accepted: bool = receipt["outcome"] == "ok";
            let ours: ExpectedVerdict = verdict(sql);
            let agrees: bool = match ours {
                ExpectedVerdict::Refused { .. } => !accepted,
                ExpectedVerdict::Tree => accepted,
                ExpectedVerdict::Gap(_) => true,
            };
            (!agrees).then(|| format!("{sql}: SQL Server {}, parser {ours:?}", receipt["outcome"]))
        })
        .collect()
}

#[test]
fn given_recorded_sql_server_verdicts_when_parsing_each_statement_then_the_parser_agrees() {
    let test_cases = [ReceiptsTestCase {
        description: "FOR XML, FOR JSON and FOR BROWSE under SQL Server 2022 PARSEONLY",
        receipts: include_str!("fixtures/result_clause_receipts.jsonl"),
        expected_disagreements: Vec::new(),
    }];
    for test_case in test_cases {
        assert_eq!(
            disagreements(test_case.receipts),
            test_case.expected_disagreements,
            "{}",
            test_case.description
        );
    }
}
