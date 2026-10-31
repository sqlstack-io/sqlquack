use crate::verify::tests::helpers::undeclared_compile_error_for;
use crate::verify::tests::test_types::CompileErrorTestCase;

#[test]
fn given_a_recorded_compile_class_error_when_verifying_then_the_case_must_declare_it() {
    let test_cases = [
        CompileErrorTestCase {
            description: "an undeclared syntax error fails",
            sqlstate: "42000",
            declared: false,
            with_setup: false,
            expected_failed: true,
        },
        CompileErrorTestCase {
            description: "an undeclared unsupported-feature error fails",
            sqlstate: "0A000",
            declared: false,
            with_setup: false,
            expected_failed: true,
        },
        CompileErrorTestCase {
            description: "a declared compile error passes the check",
            sqlstate: "42601",
            declared: true,
            with_setup: false,
            expected_failed: false,
        },
        CompileErrorTestCase {
            description: "a case creating functions is not pre-checked, so it needs no declaration",
            sqlstate: "42P13",
            declared: false,
            with_setup: true,
            expected_failed: false,
        },
        CompileErrorTestCase {
            description: "a data exception is the case's outcome, never a compile error",
            sqlstate: "22012",
            declared: false,
            with_setup: false,
            expected_failed: false,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            undeclared_compile_error_for(
                test_case.sqlstate,
                test_case.declared,
                test_case.with_setup
            ),
            test_case.expected_failed,
            "{}",
            test_case.description
        );
    }
}
