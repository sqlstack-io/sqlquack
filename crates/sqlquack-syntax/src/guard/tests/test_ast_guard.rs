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
