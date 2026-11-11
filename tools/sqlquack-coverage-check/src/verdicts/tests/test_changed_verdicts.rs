use crate::verdicts::main::changed_verdicts::changed_verdicts;
use crate::verdicts::tests::helpers::one_file;
use crate::verdicts::tests::test_types::ChangedTestCase;

#[test]
fn given_recorded_and_current_verdicts_when_compared_then_each_moved_statement_is_named() {
    let test_cases = [
        ChangedTestCase {
            description: "the same verdicts",
            recorded: vec![("duckdb", 3, vec![1])],
            current: vec![("duckdb", 3, vec![1])],
            expected_lines: vec![],
        },
        ChangedTestCase {
            description: "one statement refused now and another parsed now",
            recorded: vec![("snowflake", 4, vec![0, 2])],
            current: vec![("snowflake", 4, vec![2, 3])],
            expected_lines: vec![
                "c.json snowflake #3: now refused, recorded as parsed",
                "c.json snowflake #0: now parsed, recorded as refused",
            ],
        },
        ChangedTestCase {
            description: "a different statement count is one line, without positions",
            recorded: vec![("sqlite", 4, vec![0])],
            current: vec![("sqlite", 5, vec![1])],
            expected_lines: vec!["c.json sqlite: 5 statements, recorded 4"],
        },
        ChangedTestCase {
            description: "a dialect only one side has",
            recorded: vec![("postgres", 2, vec![])],
            current: vec![("duckdb", 1, vec![0])],
            expected_lines: vec![
                "c.json duckdb: 1 statements, recorded 0",
                "c.json postgres: 0 statements, recorded 2",
            ],
        },
    ];
    for test_case in test_cases {
        let lines = changed_verdicts(&one_file(test_case.recorded), &one_file(test_case.current));
        assert_eq!(lines, test_case.expected_lines, "{}", test_case.description);
    }
}
