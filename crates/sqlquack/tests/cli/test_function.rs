use crate::helpers::run_function_macro;
use crate::test_types::{FunctionMacroOutcome, FunctionMacroTestCase};

#[test]
fn given_a_schema_declared_sql_function_when_converting_then_its_macro_runs_in_duckdb() {
    let test_cases = [
        FunctionMacroTestCase {
            description: "a scalar body of the declared type becomes a macro computing it",
            returns: "\"NUMBER(10,2)\"",
            body: "CAST(price * 2 AS NUMBER(10,2))",
            call: "SELECT CAST(\"DOUBLED\"(1.25) AS VARCHAR)",
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(0),
                values: vec!["2.50".to_owned()],
            },
        },
        FunctionMacroTestCase {
            description: "a NUMBER body of another precision than declared is refused",
            returns: "\"NUMBER(10,2)\"",
            body: "price * 2",
            call: "",
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(1),
                values: vec![],
            },
        },
        FunctionMacroTestCase {
            description: "a body of another type than declared is refused",
            returns: "\"BOOLEAN\"",
            body: "price > 1 AND 'x'",
            call: "",
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(1),
                values: vec![],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_function_macro((test_case.returns, test_case.body), test_case.call),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
