use crate::helpers::{function_from_stdin, run_function_macro};
use crate::test_types::{
    FunctionMacroOutcome, FunctionMacroTestCase, FunctionStdinOutcome, FunctionStdinTestCase,
};

#[test]
fn given_a_project_declared_sql_function_when_converting_then_its_macro_runs_in_duckdb() {
    let test_cases = [
        FunctionMacroTestCase {
            description: "a scalar body of the declared type becomes a macro computing it",
            returns: "NUMBER(10,2)",
            body: "CAST(price * 2 AS NUMBER(10,2))",
            call: "SELECT CAST(\"DOUBLED\"(1.25) AS VARCHAR)",
            flags: &[],
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(0),
                values: vec!["2.50".to_owned()],
            },
        },
        FunctionMacroTestCase {
            description: "a sqlbuild reference in the body reads the census placeholder relation",
            returns: "NUMBER(10,2)",
            body: "CAST(price + (SELECT MIN(amount) FROM __ref(\"fees\")) AS NUMBER(10,2))",
            call: "SELECT CAST(\"DOUBLED\"(1.25) AS VARCHAR)",
            flags: &[],
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(0),
                values: vec!["2.25".to_owned()],
            },
        },
        FunctionMacroTestCase {
            description: "a NUMBER body of another precision than declared is refused",
            returns: "NUMBER(10,2)",
            body: "price * 2",
            call: "",
            flags: &[],
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(1),
                values: vec![],
            },
        },
        FunctionMacroTestCase {
            description: "a body of another type than declared is refused",
            returns: "BOOLEAN",
            body: "price > 1 AND 'x'",
            call: "",
            flags: &[],
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(1),
                values: vec![],
            },
        },
        FunctionMacroTestCase {
            description: "a quoted lower-case parameter name resolves when quoted names ignore case",
            returns: "NUMBER(10,2)",
            body: "CAST(\"price\" * 2 AS NUMBER(10,2))",
            call: "SELECT CAST(\"DOUBLED\"(1.25) AS VARCHAR)",
            flags: &["--session", "QUOTED_IDENTIFIERS_IGNORE_CASE=true"],
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(0),
                values: vec!["2.50".to_owned()],
            },
        },
        FunctionMacroTestCase {
            description: "the same body is refused under the default session, where case counts",
            returns: "NUMBER(10,2)",
            body: "CAST(\"price\" * 2 AS NUMBER(10,2))",
            call: "",
            flags: &[],
            expected_outcome: FunctionMacroOutcome {
                exit_code: Some(1),
                values: vec![],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            run_function_macro(
                (test_case.returns, test_case.body),
                test_case.call,
                test_case.flags
            ),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_body_on_standard_input_when_converting_then_refusals_print_as_json_and_diagnostics() {
    let test_cases = [
        FunctionStdinTestCase {
            description: "a refused body prints a JSON refusal report and an underlined diagnostic",
            name: "DOUBLED",
            body: "price > 1 AND\n  'x'",
            expected_outcome: FunctionStdinOutcome {
                exit_code: Some(1),
                converted: Some(false),
                first_code: Some("SQ0003".to_owned()),
                stderr_lines: vec![
                    "error[SQ0003]: the operand types fall outside every rule's proven conditions: `node:and`"
                        .to_owned(),
                    " --> <stdin>:1:1".to_owned(),
                ],
            },
        },
        FunctionStdinTestCase {
            description: "a function the project does not declare is a usage error, exit 2",
            name: "TRIPLED",
            body: "price * 3",
            expected_outcome: FunctionStdinOutcome {
                exit_code: Some(2),
                converted: None,
                first_code: None,
                stderr_lines: vec![
                    "sqlquack function: failed: no function TRIPLED is declared; declare it in the project's functions folder".to_owned(),
                ],
            },
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            function_from_stdin(test_case.name, test_case.body),
            test_case.expected_outcome,
            "{}",
            test_case.description
        );
    }
}
