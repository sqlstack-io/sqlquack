use crate::format::constants::PARSE_FAILURE;
use crate::sqlserver::profile::constants::OPAQUE_STATEMENT_FAILURE;
use crate::sqlserver::profile::tests::helpers::{recorded_sqlserver_verdicts, sqlserver_refusal};
use crate::sqlserver::profile::tests::test_types;

#[test]
fn given_recorded_sqlserver_verdicts_when_formatting_the_refused_corpus_then_each_refusal_matches_its_receipt()
 {
    let test_cases = [
        test_types::SqlServerReceiptTestCase {
            description: "SELECT * FROM t WHERE NOT c",
            expected_runs: &[
                Some((
                    4145,
                    "An expression of non-boolean type specified in a context where a condition is expected, near 'c'.",
                )),
                Some((
                    4145,
                    "An expression of non-boolean type specified in a context where a condition is expected, near 'c'.",
                )),
            ],
            expected_refusal: PARSE_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT * FROM t WITH (TABLOCK, INDEX(myindex))",
            expected_runs: &[None, None],
            expected_refusal: OPAQUE_STATEMENT_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT Employee_ID, Department_ID FROM @MyTableVar",
            expected_runs: &[
                Some((1087, "Must declare the table variable \"@MyTableVar\".")),
                Some((1087, "Must declare the table variable \"@MyTableVar\".")),
            ],
            expected_refusal: OPAQUE_STATEMENT_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT x FROM @MyTableVar AS m JOIN Employee ON m.EmployeeID = Employee.EmployeeID",
            expected_runs: &[
                Some((1087, "Must declare the table variable \"@MyTableVar\".")),
                Some((1087, "Must declare the table variable \"@MyTableVar\".")),
            ],
            expected_refusal: OPAQUE_STATEMENT_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT TOP 10 PERCENT",
            expected_runs: &[
                Some((102, "Incorrect syntax near 'PERCENT'.")),
                Some((102, "Incorrect syntax near 'PERCENT'.")),
            ],
            expected_refusal: PARSE_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT TOP 10 PERCENT WITH TIES",
            expected_runs: &[
                Some((102, "Incorrect syntax near 'TIES'.")),
                Some((102, "Incorrect syntax near 'TIES'.")),
            ],
            expected_refusal: PARSE_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME FROM c TO d",
            expected_runs: &[
                Some((102, "Incorrect syntax near 'c'.")),
                Some((102, "Incorrect syntax near 'c'.")),
            ],
            expected_refusal: PARSE_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME BETWEEN c AND d",
            expected_runs: &[
                Some((102, "Incorrect syntax near 'c'.")),
                Some((102, "Incorrect syntax near 'c'.")),
            ],
            expected_refusal: PARSE_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT [x] FROM [a].[b] FOR SYSTEM_TIME CONTAINED IN (c, d)",
            expected_runs: &[
                Some((102, "Incorrect syntax near 'c'.")),
                Some((102, "Incorrect syntax near 'c'.")),
            ],
            expected_refusal: PARSE_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT NEXT VALUE FOR db.schema.sequence_name OVER (ORDER BY foo), col",
            expected_runs: &[
                Some((156, "Incorrect syntax near the keyword 'schema'.")),
                Some((156, "Incorrect syntax near the keyword 'schema'.")),
            ],
            expected_refusal: PARSE_FAILURE,
        },
        test_types::SqlServerReceiptTestCase {
            description: "SELECT * FROM @x",
            expected_runs: &[
                Some((1087, "Must declare the table variable \"@x\".")),
                Some((1087, "Must declare the table variable \"@x\".")),
            ],
            expected_refusal: OPAQUE_STATEMENT_FAILURE,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            (
                recorded_sqlserver_verdicts(test_case.description),
                sqlserver_refusal(test_case.description)
            ),
            (
                test_case
                    .expected_runs
                    .iter()
                    .map(|run| run.map(|(code, message)| (code, message.to_string())))
                    .collect::<Vec<_>>(),
                Some(test_case.expected_refusal.to_string())
            ),
            "{}",
            test_case.description
        );
    }
}
