//! Verdicts and trees of the SQLite parser, as SQLite 3.53.4's `sqlite3_prepare_v2` parses.

use crate::parse::constants::TOO_DEEP;
use crate::parse::main::parse::parse;
use crate::parse::tests::test_types::NestingTestCase;

/// The stack of the thread the nesting cases run on: Rust's default for a spawned thread.
const TEST_STACK: usize = 2 * 1024 * 1024;

#[test]
fn given_deep_nesting_when_parsing_then_it_fails_cleanly_without_overflowing_the_stack() {
    let test_cases = [
        NestingTestCase {
            description: "parentheses",
            prefix: "(",
            core: "1",
            suffix: ")",
            expected_message: TOO_DEEP,
        },
        NestingTestCase {
            description: "prefix operators",
            prefix: "- ",
            core: "1",
            suffix: "",
            expected_message: TOO_DEEP,
        },
        NestingTestCase {
            description: "NOT",
            prefix: "NOT ",
            core: "1",
            suffix: "",
            expected_message: TOO_DEEP,
        },
        NestingTestCase {
            description: "a chain of additions",
            prefix: "",
            core: "1",
            suffix: " + 1",
            expected_message: TOO_DEEP,
        },
        NestingTestCase {
            description: "calls",
            prefix: "abs(",
            core: "1",
            suffix: ")",
            expected_message: TOO_DEEP,
        },
        NestingTestCase {
            description: "scalar subqueries",
            prefix: "(SELECT ",
            core: "1",
            suffix: ")",
            expected_message: TOO_DEEP,
        },
        NestingTestCase {
            description: "CASE",
            prefix: "CASE WHEN 1 THEN ",
            core: "1",
            suffix: " END",
            expected_message: TOO_DEEP,
        },
    ];
    for test_case in test_cases {
        let sql: String = format!(
            "SELECT {}{}{}",
            test_case.prefix.repeat(1500),
            test_case.core,
            test_case.suffix.repeat(1500)
        );
        let refused: Option<(String, bool)> = std::thread::Builder::new()
            .stack_size(TEST_STACK)
            .spawn(move || parse(&sql).err().map(|error| (error.message, error.syntax)))
            .expect("a thread")
            .join()
            .expect("no stack overflow");
        assert_eq!(
            refused,
            Some((test_case.expected_message.to_owned(), false)),
            "{}",
            test_case.description
        );
    }
}
