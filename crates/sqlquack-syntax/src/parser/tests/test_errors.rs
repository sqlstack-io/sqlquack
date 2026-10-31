use crate::parser::tests::helpers::configured_outcome;
use crate::parser::tests::helpers::decisions_outside;
use crate::parser::tests::helpers::kind_at;
use crate::parser::tests::helpers::listed;
use crate::parser::tests::helpers::parse_error;
use crate::parser::tests::helpers::parse_sql_outcome;
use crate::parser::tests::test_types::Decision;
use crate::parser::tests::test_types::GuardedParseTestCase;
use crate::parser::tests::test_types::ParseErrorTestCase;
use crate::parser::tests::test_types::ParseKindTestCase;
use crate::parser::tests::test_types::SourceEntryTestCase;
use crate::parser::tests::test_types::TerminationTestCase;
use crate::tokens::models::DialectType;

#[test]
fn given_incomplete_sql_when_parsing_then_parsing_fails() {
    let test_cases = [
        ParseErrorTestCase {
            description: "an unclosed parenthesis is rejected",
            dialect: None,
            sql: "SELECT (1 + 2",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an unopened parenthesis is rejected",
            dialect: None,
            sql: "SELECT 1 + 2)",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an unclosed nested parenthesis is rejected",
            dialect: None,
            sql: "SELECT ((1 + 2)",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an empty WHERE is rejected",
            dialect: None,
            sql: "SELECT * FROM users WHERE",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a dangling AND is rejected",
            dialect: None,
            sql: "SELECT * FROM users WHERE a AND",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a dangling OR is rejected",
            dialect: None,
            sql: "SELECT * FROM users WHERE a OR",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a JOIN without a table is rejected",
            dialect: None,
            sql: "SELECT * FROM users JOIN",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an empty ORDER BY is rejected",
            dialect: None,
            sql: "SELECT * FROM users ORDER BY",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an empty GROUP BY is rejected",
            dialect: None,
            sql: "SELECT * FROM users GROUP BY",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an unclosed string is rejected",
            dialect: None,
            sql: "SELECT 'unclosed string",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a double comma is rejected",
            dialect: None,
            sql: "SELECT a,, b FROM users",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a BETWEEN without bounds is rejected",
            dialect: None,
            sql: "SELECT x BETWEEN",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a BETWEEN without a high bound is rejected",
            dialect: None,
            sql: "SELECT x BETWEEN 1 AND",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a LIKE without a pattern is rejected",
            dialect: None,
            sql: "SELECT x LIKE",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an ILIKE without a pattern is rejected",
            dialect: None,
            sql: "SELECT x ILIKE",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an IN without a list is rejected",
            dialect: None,
            sql: "SELECT x IN",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "an IS DISTINCT FROM without a right side is rejected",
            dialect: None,
            sql: "SELECT x IS DISTINCT FROM",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a dangling bitwise AND is rejected",
            dialect: None,
            sql: "SELECT x &",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a dangling bitwise OR is rejected",
            dialect: None,
            sql: "SELECT x |",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a dangling bitwise XOR is rejected",
            dialect: None,
            sql: "SELECT x ^",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a dangling bitwise NOT is rejected",
            dialect: None,
            sql: "SELECT ~",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a JSON_OBJECT key without a value is rejected",
            dialect: None,
            sql: "SELECT JSON_OBJECT('a')",
            expected_error: "error",
        },
        ParseErrorTestCase {
            description: "a JSON_OBJECT key with an empty value is rejected",
            dialect: None,
            sql: "SELECT JSON_OBJECT('a':)",
            expected_error: "error",
        },
    ];

    for test_case in test_cases {
        let error = parse_error(test_case.dialect, test_case.sql);
        assert!(
            error.contains(test_case.expected_error),
            "{}: {error}",
            test_case.description
        );
    }
}

#[test]
fn given_unusual_but_valid_sql_when_parsing_then_it_parses() {
    let test_cases = [
        ParseKindTestCase {
            description: "a parenthesized lambda with typed parameters parses",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT REDUCE(a, 0, (acc NUMBER(18, 2), i VARIANT) -> acc + i) FROM t",
            pointer: "/0/select/expressions/0/function/args/2",
            expected_kind: Ok("lambda"),
        },
        ParseKindTestCase {
            description: "a trailing comma before FROM is tolerated",
            dialect: None,
            sql: "SELECT a, b, FROM users",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "a Unicode string literal parses",
            dialect: None,
            sql: "SELECT '你好世界'",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "a Unicode comment parses",
            dialect: None,
            sql: "SELECT 1 -- 日本語コメント",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "an emoji string parses",
            dialect: None,
            sql: "SELECT '😀🎉'",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "deeply nested parentheses parse",
            dialect: None,
            sql: "SELECT ((((((((1 + 2))))))))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "nested subqueries parse",
            dialect: None,
            sql: "SELECT * FROM (SELECT * FROM (SELECT * FROM users))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "nested CASE expressions parse",
            dialect: None,
            sql: "SELECT CASE WHEN a THEN CASE WHEN b THEN 1 ELSE 2 END ELSE 3 END",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "nested function calls parse",
            dialect: None,
            sql: "SELECT UPPER(LOWER(TRIM(UPPER(LOWER(name)))))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "nested arithmetic parses",
            dialect: None,
            sql: "SELECT (1 + (2 * (3 - (4 / (5 + 6)))))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_doubtful_sql_when_parsing_then_the_parser_decides_without_panicking() {
    let test_cases = [
        TerminationTestCase {
            description: "a missing FROM keyword is parsed or rejected",
            inputs: vec!["SELECT * users".to_owned()],
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
            ],
        },
        TerminationTestCase {
            description: "a missing SELECT keyword is parsed or rejected",
            inputs: vec!["* FROM users".to_owned()],
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
            ],
        },
        TerminationTestCase {
            description: "an empty projection list is parsed or rejected",
            inputs: vec!["SELECT FROM users".to_owned()],
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
            ],
        },
        TerminationTestCase {
            description: "a JOIN without ON is parsed or rejected",
            inputs: vec!["SELECT * FROM users JOIN orders".to_owned()],
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
            ],
        },
        TerminationTestCase {
            description: "the <> operator is parsed or rejected",
            inputs: vec!["SELECT 1 <> 2".to_owned()],
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
            ],
        },
        TerminationTestCase {
            description: "Unicode identifiers is parsed or rejected",
            inputs: vec!["SELECT 用户名 FROM 表".to_owned()],
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
            ],
        },
        TerminationTestCase {
            description: "a lone semicolon is parsed or rejected",
            inputs: vec![";".to_owned()],
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
            ],
        },
        TerminationTestCase {
            description: "several semicolons is parsed or rejected",
            inputs: vec![";;;".to_owned()],
            expected_decisions: &[
                Decision::Parsed,
                Decision::Rejected {
                    by_depth_guard: false,
                },
            ],
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            decisions_outside(&test_case.inputs, test_case.expected_decisions),
            Vec::<(String, Decision)>::new(),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_input_without_statements_when_parsing_then_there_are_none() {
    let test_cases = [
        SourceEntryTestCase {
            description: "whitespace has no statements",
            sql: "   
	  ",
            expected_outcome: "statements: 0",
        },
        SourceEntryTestCase {
            description: "a line comment has no statements",
            sql: "-- this is a comment",
            expected_outcome: "statements: 0",
        },
        SourceEntryTestCase {
            description: "a block comment has no statements",
            sql: "/* block comment */",
            expected_outcome: "statements: 0",
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            parse_sql_outcome(test_case.sql),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_large_statements_when_parsing_then_they_parse() {
    let test_cases = [
        GuardedParseTestCase {
            description: "a hundred projections parse",
            sql: format!("SELECT {} FROM table1", listed("col{}", 100, ", ")),
            max_parser_depth: None,
            outcome: configured_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "a long string literal parses",
            sql: format!("SELECT '{}'", "a".repeat(1024)),
            max_parser_depth: None,
            outcome: configured_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "twenty UNION ALL branches parse",
            sql: listed("SELECT {}", 20, " UNION ALL "),
            max_parser_depth: None,
            outcome: configured_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "nine joins parse",
            sql: format!(
                "SELECT * FROM t1{}",
                listed(" JOIN t{} ON t1.id = t{}.id", 10, "").replacen(
                    " JOIN t1 ON t1.id = t1.id",
                    "",
                    1
                )
            ),
            max_parser_depth: None,
            outcome: configured_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "a hundred-value IN list parses",
            sql: format!(
                "SELECT * FROM users WHERE id IN ({})",
                listed("{}", 100, ", ")
            ),
            max_parser_depth: None,
            outcome: configured_outcome,
            expected_outcome_part: "parsed",
        },
    ];

    for test_case in test_cases {
        let outcome = (test_case.outcome)(&test_case.sql, test_case.max_parser_depth);
        assert!(
            outcome.contains(test_case.expected_outcome_part),
            "{}: {outcome}",
            test_case.description
        );
    }
}
