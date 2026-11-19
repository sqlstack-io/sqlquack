//! Function-call nesting: refused cleanly at the caller's limit, never by the thread's stack.

use sqlquack_syntax::Dialect;

use crate::parsing::main::checked_call_depth::checked_call_depth;
use crate::parsing::main::parse_source_within::parse_source_within;
use crate::parsing::tests::test_types::{CallDepthTestCase, CheckedDepthTestCase};

/// A thread stack smaller than any SQLQuack runs on, so passing shows the stack grows on demand.
const SMALL_STACK_BYTES: usize = 256 * 1024;

#[test]
fn given_nested_function_calls_when_parsed_on_a_small_stack_then_the_limit_decides_cleanly() {
    let test_cases = [
        CallDepthTestCase {
            description: "the default limit reads 256 nested calls",
            limit: None,
            levels: 256,
            expected_outcome: Ok(1),
        },
        CallDepthTestCase {
            description: "the default limit refuses 257 nested calls and names the limit",
            limit: None,
            levels: 257,
            expected_outcome: Err(
                "E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED: value 257 exceeds configured limit 256",
            ),
        },
        CallDepthTestCase {
            description: "a raised limit of 300 reads 300 nested calls",
            limit: Some(300),
            levels: 300,
            expected_outcome: Ok(1),
        },
        CallDepthTestCase {
            description: "a raised limit of 300 refuses 301 nested calls",
            limit: Some(300),
            levels: 301,
            expected_outcome: Err("value 301 exceeds configured limit 300"),
        },
        CallDepthTestCase {
            description: "the ceiling of 512 reads 511 nested calls inside a SELECT",
            limit: Some(512),
            levels: 511,
            expected_outcome: Ok(1),
        },
        CallDepthTestCase {
            description: "the ceiling of 512 refuses 512 nested calls, saying why fewer fit",
            limit: Some(512),
            levels: 512,
            expected_outcome: Err(
                "value 513 exceeds configured limit 512. The maximum is 512. The statement and \
                 every expression around the calls also count toward that limit, so at 512 a bare \
                 SELECT can nest at most 511 calls, and fewer fit inside an alias, comparison or \
                 CTE.",
            ),
        },
        CallDepthTestCase {
            description: "a limit of 1000 is refused, naming the ceiling of 512",
            limit: Some(1000),
            levels: 1000,
            expected_outcome: Err(
                "must be between 1 and 512, SQLQuack's fixed nesting ceiling, not 1000",
            ),
        },
        CallDepthTestCase {
            description: "at the ceiling, 600 nested calls are refused, naming the ceiling as fixed",
            limit: Some(512),
            levels: 600,
            expected_outcome: Err(
                "E_GUARD_NESTING_DEPTH_EXCEEDED: value 513 exceeds configured limit 512; 512 is \
                 SQLQuack's fixed nesting ceiling, which no setting raises",
            ),
        },
        CallDepthTestCase {
            description: "a lowered limit of 10 refuses 11 nested calls",
            limit: Some(10),
            levels: 11,
            expected_outcome: Err("value 11 exceeds configured limit 10"),
        },
    ];

    for test_case in test_cases {
        let sql = format!(
            "SELECT {}1{}",
            "ABS(".repeat(test_case.levels),
            ")".repeat(test_case.levels)
        );
        let limit = test_case.limit;
        let outcome: Result<usize, String> = std::thread::Builder::new()
            .stack_size(SMALL_STACK_BYTES)
            .spawn(move || {
                parse_source_within(&sql, Dialect::Snowflake, limit)
                    .map(|statements| statements.len())
                    .map_err(|failure| failure.message)
            })
            .expect("the small-stack thread starts")
            .join()
            .unwrap_or_else(|_| panic!("{}: the parse panicked", test_case.description));
        let ends_as_expected: Result<usize, bool> =
            outcome.as_ref().map(|count| *count).map_err(|message| {
                message.ends_with(test_case.expected_outcome.err().unwrap_or_default())
            });
        assert_eq!(
            ends_as_expected,
            test_case.expected_outcome.map_err(|_| true),
            "{}: {outcome:?}",
            test_case.description
        );
    }
}

#[test]
fn given_a_requested_call_depth_when_checked_then_only_1_to_512_is_accepted() {
    let test_cases = [
        CheckedDepthTestCase {
            description: "zero is refused",
            depth: 0,
            expected_result: Err("the function-call depth limit must be between 1 and 512, \
                SQLQuack's fixed nesting ceiling, not 0"),
        },
        CheckedDepthTestCase {
            description: "one is the lowest limit",
            depth: 1,
            expected_result: Ok(1),
        },
        CheckedDepthTestCase {
            description: "the ceiling itself is accepted",
            depth: 512,
            expected_result: Ok(512),
        },
        CheckedDepthTestCase {
            description: "one past the ceiling is refused",
            depth: 513,
            expected_result: Err("the function-call depth limit must be between 1 and 512, \
                SQLQuack's fixed nesting ceiling, not 513"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            checked_call_depth(test_case.depth),
            test_case.expected_result.map_err(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
