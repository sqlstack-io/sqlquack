use crate::guard::tests::helpers::guarded_parse;
use crate::guard::tests::test_types::AstDepthTestCase;

#[test]
fn given_a_long_flat_operator_chain_when_parsing_then_only_real_nesting_counts_as_depth() {
    let test_cases = [
        AstDepthTestCase {
            description: "50,000 added terms are one flat chain",
            sql: || format!("SELECT {}", vec!["1"; 50_000].join(" + ")),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a long chain mixing products, sums and concatenation stays flat",
            sql: || format!("SELECT {}", vec!["2 * 3 - 1 || 'x'"; 5_000].join(" + ")),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "600 nested unary minus signs are real nesting",
            sql: || format!("SELECT {}1", "- ".repeat(600)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
    ];
    for test_case in test_cases {
        let outcome: Result<(), String> = guarded_parse(&(test_case.sql)());
        let code: Option<String> = outcome.as_ref().err().map(|error| {
            error
                .split(|c: char| !(c.is_ascii_uppercase() || c == '_'))
                .find(|word| word.starts_with("E_"))
                .unwrap_or(error)
                .to_owned()
        });
        assert_eq!(
            code.as_deref(),
            test_case.expected_error,
            "{}: {outcome:?}",
            test_case.description
        );
    }
}

#[test]
fn given_a_70k_term_chain_when_parsing_on_a_2_mb_thread_then_it_parses_or_is_refused_without_overflow(
) {
    let test_cases = [
        AstDepthTestCase {
            description: "a + chain is flat and parses",
            sql: || format!("SELECT 1{}", "+1".repeat(70_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a || chain is flat and parses",
            sql: || format!("SELECT 'a'{}", "||'a'".repeat(70_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a :: chain is refused",
            sql: || format!("SELECT 1{}", "::INT".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "an = chain is refused",
            sql: || format!("SELECT 1{}", "=1".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a [1] chain is refused",
            sql: || format!("SELECT v{}", "[1]".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a .a chain is refused",
            sql: || format!("SELECT v{}", ".a".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a COLLATE chain is refused",
            sql: || format!("SELECT 'a'{}", " COLLATE 'en'".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "an IS NULL chain is refused",
            sql: || format!("SELECT 1{}", " IS NULL".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "an IN chain is refused",
            sql: || format!("SELECT 1{}", " IN (1)".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a : path chain is one path and parses",
            sql: || format!("SELECT v{}", ":a".repeat(70_000)),
            expected_error: None,
        },
        AstDepthTestCase {
            description: "a chain alternating = and IS NULL is refused",
            sql: || format!("SELECT 1{}", "=1 IS NULL".repeat(70_000)),
            expected_error: Some("E_GUARD_AST_DEPTH_EXCEEDED"),
        },
        AstDepthTestCase {
            description: "a :: chain cut off by a syntax error is refused",
            sql: || format!("SELECT 1{}::", "::INT".repeat(70_000)),
            expected_error: Some("Expected identifier"),
        },
    ];
    for test_case in test_cases {
        let sql = (test_case.sql)();
        let outcome = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || guarded_parse(&sql))
            .expect("the parse thread starts")
            .join()
            .expect("the parse thread finishes");
        assert_eq!(
            outcome
                .as_ref()
                .err()
                .map(|error| error.contains(test_case.expected_error.unwrap_or("\0"))),
            test_case.expected_error.map(|_| true),
            "{}: {outcome:?}",
            test_case.description
        );
    }
}
