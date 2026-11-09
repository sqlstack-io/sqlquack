use crate::parser::tests::helpers::data_type;
use crate::parser::tests::helpers::guarded;
use crate::parser::tests::helpers::stream_data_type;
use crate::parser::tests::test_types::DataTypeErrorTestCase;
use crate::parser::tests::test_types::DataTypeTestCase;
use crate::parser::tests::test_types::StreamDataTypeTestCase;
use sqlquack_syntax::expressions::types::models::DataType;
use sqlquack_syntax::guard::models::ComplexityGuardOptions;
use sqlquack_syntax::guard::models::ParseOptions;
use sqlquack_syntax::tokens::models::DialectType;

#[test]
fn given_a_standalone_type_when_parsing_then_it_becomes_its_data_type() {
    let test_cases = [
        DataTypeTestCase {
            description: "a decimal keeps its precision and scale",
            dialect: DialectType::DuckDB,
            sql: "DECIMAL(10, 2)",
            options: ParseOptions::default(),
            expected_data_type: DataType::Decimal {
                precision: Some(10),
                scale: Some(2),
            },
        },
        DataTypeTestCase {
            description: "an array keeps its element type",
            dialect: DialectType::DuckDB,
            sql: "INT[]",
            options: ParseOptions::default(),
            expected_data_type: DataType::Array {
                element_type: Box::new(DataType::Int {
                    length: None,
                    integer_spelling: false,
                }),
                dimension: None,
            },
        },
        DataTypeTestCase {
            description: "a custom type keeps its name",
            dialect: DialectType::DuckDB,
            sql: "MyCustomType",
            options: ParseOptions::default(),
            expected_data_type: DataType::Custom {
                name: "MyCustomType".to_string(),
            },
        },
        DataTypeTestCase {
            description: "default options parse a Snowflake decimal",
            dialect: DialectType::Snowflake,
            sql: "DECIMAL(10, 2)",
            options: ParseOptions::default(),
            expected_data_type: DataType::Decimal {
                precision: Some(10),
                scale: Some(2),
            },
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            data_type(test_case.dialect, test_case.sql, &test_case.options),
            Ok(test_case.expected_data_type),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_standalone_type_that_breaks_a_rule_when_parsing_then_the_rule_is_named() {
    let test_cases = [
        DataTypeErrorTestCase {
            description: "SQL after the type is refused",
            dialect: DialectType::DuckDB,
            sql: "DECIMAL(10, 2) SELECT 1",
            options: ParseOptions::default(),
            expected_error: "Unexpected token after data type",
        },
        DataTypeErrorTestCase {
            description: "the input size limit applies",
            dialect: DialectType::Snowflake,
            sql: "DECIMAL(10, 2)",
            options: guarded(ComplexityGuardOptions {
                max_input_bytes: Some(2),
                ..Default::default()
            }),
            expected_error: "E_GUARD_INPUT_TOO_LARGE",
        },
        DataTypeErrorTestCase {
            description: "the AST node budget applies",
            dialect: DialectType::Snowflake,
            sql: "DECIMAL(10, 2)",
            options: guarded(ComplexityGuardOptions {
                max_ast_nodes: Some(0),
                ..Default::default()
            }),
            expected_error: "E_GUARD_AST_BUDGET_EXCEEDED",
        },
        DataTypeErrorTestCase {
            description: "the parser depth limit applies",
            dialect: DialectType::Snowflake,
            sql: "DECIMAL(10, 2)",
            options: guarded(ComplexityGuardOptions {
                max_parser_depth: Some(0),
                ..Default::default()
            }),
            expected_error: "E_GUARD_PARSER_DEPTH_EXCEEDED",
        },
    ];

    for test_case in test_cases {
        let error = data_type(test_case.dialect, test_case.sql, &test_case.options)
            .err()
            .unwrap_or_default();
        assert!(
            error.contains(test_case.expected_error),
            "{}: {error}",
            test_case.description
        );
    }
}

#[test]
fn given_a_token_stream_when_parsing_a_standalone_type_then_a_terminator_ends_the_input() {
    let test_cases = [
        StreamDataTypeTestCase {
            description: "a trailing terminator is the end of input",
            parts: &["INT", "<EOF>"],
            expected_data_type: Ok(DataType::Int {
                length: None,
                integer_spelling: false,
            }),
        },
        StreamDataTypeTestCase {
            description: "the end of the stream is the end of input",
            parts: &["INT"],
            expected_data_type: Ok(DataType::Int {
                length: None,
                integer_spelling: false,
            }),
        },
        StreamDataTypeTestCase {
            description: "a type after a leading terminator is not read",
            parts: &["<EOF>", "INT"],
            expected_data_type: Err(
                "Parse error at line 1, column 4: Unexpected token after end of input: Var"
                    .to_owned(),
            ),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            stream_data_type(test_case.parts),
            test_case.expected_data_type,
            "{}",
            test_case.description
        );
    }
}
