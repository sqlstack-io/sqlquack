//! Verdicts and trees of the SQLite parser, as SQLite 3.53.4's `sqlite3_prepare_v2` parses.

use crate::parse::constants::{INCOMPLETE_INPUT, TOO_DEEP, UNSUPPORTED_FRAME};
use crate::parse::main::parse::parse;
use crate::parse::tests::test_types::{NestingTestCase, VerdictTestCase};

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

#[test]
fn given_queries_when_parsing_then_the_verdicts_match_sqlite() {
    let test_cases = [
        VerdictTestCase {
            description: "the constructs SQLBuild generates, with every identifier quote",
            sql: "WITH totals AS (SELECT customer_id, amount FROM [stg orders]) SELECT t.customer_id, `amount` || 'x', x'00ff', 0xFF, COALESCE(SUM(amount), 0), strftime('%Y', date('now')), ROW_NUMBER() OVER (PARTITION BY t.customer_id ORDER BY 1) FROM totals AS t LEFT JOIN o USING (customer_id) WHERE t.customer_id IN (1, 2) GROUP BY 1 UNION SELECT DISTINCT 1, 2, 3, 4, 5, 6, 7 FROM o",
            expected_verdict: Ok(&[]),
        },
        VerdictTestCase {
            description: "strings as names",
            sql: "SELECT 'a' FROM 'stg_orders' AS 'o'",
            expected_verdict: Ok(&[]),
        },
        VerdictTestCase {
            description: "a duplicate CTE name, compared without case",
            sql: "WITH a AS (SELECT 1), A AS (SELECT 1) SELECT 1",
            expected_verdict: Err("duplicate WITH table name: A"),
        },
        VerdictTestCase {
            description: "a duplicate CTE name raised only when the next token fits",
            sql: "WITH a AS (SELECT 1), a AS (SELECT 1) x",
            expected_verdict: Err("near \"x\": syntax error"),
        },
        VerdictTestCase {
            description: "NOT at the end of a FROM item",
            sql: "SELECT 1 FROM t NOT",
            expected_verdict: Err(INCOMPLETE_INPUT),
        },
        VerdictTestCase {
            description: "NOT before a token it cannot take",
            sql: "SELECT 1 FROM t NOT x",
            expected_verdict: Err("near \"x\": syntax error"),
        },
        VerdictTestCase {
            description: "a call cut off after its star",
            sql: "SELECT count(*",
            expected_verdict: Err(INCOMPLETE_INPUT),
        },
        VerdictTestCase {
            description: "ORDER BY before a compound operator",
            sql: "SELECT 1 ORDER BY 1 UNION SELECT 2",
            expected_verdict: Err("ORDER BY clause should come after UNION not before"),
        },
        VerdictTestCase {
            description: "the compound check raised only when the next token fits",
            sql: "SELECT 1 ORDER BY 1 UNION ! SELECT 2",
            expected_verdict: Err("unrecognized token: \"!\""),
        },
        VerdictTestCase {
            description: "a frame that starts after it ends",
            sql: "SELECT count(*) OVER (ORDER BY 1 RANGE 1 FOLLOWING)",
            expected_verdict: Err(UNSUPPORTED_FRAME),
        },
        VerdictTestCase {
            description: "the frame check raised only when the next token fits",
            sql: "SELECT count(*) OVER (ORDER BY 1 RANGE BETWEEN 1 FOLLOWING AND 1 PRECEDING x)",
            expected_verdict: Err("near \"x\": syntax error"),
        },
        VerdictTestCase {
            description: "ORDER BY in a window function's arguments",
            sql: "SELECT max(1 ORDER BY 1) OVER ()",
            expected_verdict: Err("ORDER BY may not be used with non-aggregate max()"),
        },
        VerdictTestCase {
            description: "a join word SQLite does not know",
            sql: "SELECT * FROM t1 LEFT t2 JOIN t3",
            expected_verdict: Err("unknown join type: LEFT t2"),
        },
        VerdictTestCase {
            description: "INDEXED BY as a gap",
            sql: "SELECT 1 FROM t1 INDEXED BY i1",
            expected_verdict: Ok(&["indexed_by"]),
        },
        VerdictTestCase {
            description: "IN of a table as a gap",
            sql: "SELECT 1 IN t1",
            expected_verdict: Ok(&["in_table"]),
        },
        VerdictTestCase {
            description: "a statement other than a query as a gap",
            sql: "PRAGMA user_version",
            expected_verdict: Ok(&["pragma"]),
        },
    ];
    for test_case in test_cases {
        let verdict: Result<Vec<&str>, String> = parse(test_case.sql)
            .map(|parsed| parsed.gaps.iter().map(|gap| gap.construct).collect())
            .map_err(|error| error.message);
        assert_eq!(
            verdict,
            test_case
                .expected_verdict
                .map(<[&str]>::to_vec)
                .map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
