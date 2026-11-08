use crate::verify::tests::helpers::compile_error_judgement_for;
use crate::verify::tests::test_types::CompileErrorTestCase;

#[test]
fn given_a_recorded_error_when_verifying_then_compile_errors_are_declared_and_match_a_refusal() {
    let test_cases = [
        CompileErrorTestCase {
            description: "an undeclared syntax error fails",
            sqlstate: "42000",
            declared: false,
            with_setup: false,
            expected_failed: true,
            expected_rejected: false,
        },
        CompileErrorTestCase {
            description: "an undeclared unsupported-feature error fails",
            sqlstate: "0A000",
            declared: false,
            with_setup: false,
            expected_failed: true,
            expected_rejected: false,
        },
        CompileErrorTestCase {
            description: "a declared compile error passes the check",
            sqlstate: "42601",
            declared: true,
            with_setup: false,
            expected_failed: false,
            expected_rejected: true,
        },
        CompileErrorTestCase {
            description: "a case creating functions is not pre-checked, so it needs no declaration",
            sqlstate: "42P13",
            declared: false,
            with_setup: true,
            expected_failed: false,
            expected_rejected: false,
        },
        CompileErrorTestCase {
            description: "a data exception is the case's outcome, never a compile error",
            sqlstate: "22012",
            declared: false,
            with_setup: false,
            expected_failed: false,
            expected_rejected: false,
        },
        CompileErrorTestCase {
            description: "a declared case that recorded a runtime error stays a plain refusal",
            sqlstate: "22012",
            declared: true,
            with_setup: false,
            expected_failed: false,
            expected_rejected: false,
        },
        CompileErrorTestCase {
            description: "a declared unsupported-feature error matches a refusal",
            sqlstate: "0A000",
            declared: true,
            with_setup: false,
            expected_failed: false,
            expected_rejected: true,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            compile_error_judgement_for(
                test_case.sqlstate,
                test_case.declared,
                test_case.with_setup
            ),
            (test_case.expected_failed, test_case.expected_rejected),
            "{}",
            test_case.description
        );
    }
}
