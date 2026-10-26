use crate::ported::compat::{parse_data_type, DataType, DialectType};

fn int_type() -> DataType {
    DataType::Int {
        length: None,
        integer_spelling: false,
    }
}

#[test]
fn parse_standalone_decimal_type() {
    let data_type =
        parse_data_type("DECIMAL(10, 2)", DialectType::DuckDB).expect("decimal should parse");

    assert_eq!(
        data_type,
        DataType::Decimal {
            precision: Some(10),
            scale: Some(2),
        }
    );
}

#[test]
fn parse_standalone_array_type() {
    let data_type = parse_data_type("INT[]", DialectType::DuckDB).expect("array should parse");

    match data_type {
        DataType::Array {
            element_type,
            dimension,
        } => {
            assert_eq!(
                *element_type,
                DataType::Int {
                    length: None,
                    integer_spelling: false,
                }
            );
            assert_eq!(dimension, None);
        }
        other => panic!("expected array data type, got {other:?}"),
    }
}

#[test]
fn parse_standalone_custom_type_preserves_name() {
    let data_type =
        parse_data_type("MyCustomType", DialectType::DuckDB).expect("custom type should parse");

    assert_eq!(
        data_type,
        DataType::Custom {
            name: "MyCustomType".to_string(),
        }
    );
}

#[test]
fn parse_standalone_data_type_rejects_trailing_sql() {
    let error = parse_data_type("DECIMAL(10, 2) SELECT 1", DialectType::DuckDB)
        .expect_err("trailing SQL should fail");

    assert!(error
        .to_string()
        .contains("Unexpected token after data type"));
}

/// A caller-supplied `TokenType::Eof` ends input for the standalone type parser too.
#[test]
fn parse_standalone_data_type_treats_an_eof_token_as_end_of_input() {
    use crate::ported::compat::{Parser, Token, TokenType, Tokenizer};
    use crate::tokens::Span;

    let stream = |parts: &[&str]| {
        let mut out: Vec<Token> = Vec::new();
        for part in parts {
            if *part == "<EOF>" {
                out.push(Token::new(TokenType::Eof, "", Span::default()));
            } else {
                out.extend(Tokenizer::default().tokenize(part).expect("tokenizing"));
            }
        }
        out
    };

    // A trailing terminator is not a trailing token.
    assert_eq!(
        Parser::new(stream(&["INT", "<EOF>"]))
            .parse_standalone_data_type()
            .expect("a trailing terminator is end of input"),
        int_type()
    );
    assert_eq!(
        Parser::new(stream(&["INT"]))
            .parse_standalone_data_type()
            .expect("and so is the end of the stream"),
        int_type()
    );

    // Tokens after one are a stream built wrongly. Refused, not dropped: the parse used to
    // stop at the terminator and return `Ok(Int)` with `SELECT 2` ignored.
    let error = Parser::new(stream(&["INT", "<EOF>", "SELECT 2"]))
        .parse_standalone_data_type()
        .expect_err("tokens after the terminator should fail");
    assert!(
        error
            .to_string()
            .contains("Unexpected token after end of input"),
        "unexpected error: {error}"
    );
}

/// A terminator at the front leaves the standalone type parser nothing to read.
#[test]
fn parse_standalone_data_type_errors_on_an_empty_or_eof_first_stream() {
    use crate::ported::compat::{Parser, Token, TokenType, Tokenizer};
    use crate::tokens::Span;

    let eof = || Token::new(TokenType::Eof, "", Span::default());

    // Nothing to read: end of input, not a panic and not a type.
    for tokens in [vec![eof()], Vec::new()] {
        let error = Parser::new(tokens)
            .parse_standalone_data_type()
            .expect_err("an empty stream is not a data type");
        assert!(
            error.to_string().contains("Unexpected end of input"),
            "unexpected error: {error}"
        );
    }

    // A terminator with a type after it is a stream built wrongly, and the error names the
    // token that followed rather than the empty remainder.
    let mut leading = vec![eof()];
    leading.extend(Tokenizer::default().tokenize("INT").expect("tokenizing"));
    let error = Parser::new(leading)
        .parse_standalone_data_type()
        .expect_err("a type after the terminator should not be read");
    assert!(
        error
            .to_string()
            .contains("Unexpected token after end of input"),
        "unexpected error: {error}"
    );

    // Two terminators: the second is a token after the first.
    let error = Parser::new(vec![eof(), eof()])
        .parse_standalone_data_type()
        .expect_err("a second terminator is a token after the first");
    assert!(
        error
            .to_string()
            .contains("Unexpected token after end of input"),
        "unexpected error: {error}"
    );
}
#[test]
fn standalone_data_types_honor_parse_options() {
    use crate::ported::compat::{
        parse_data_type_with_options, ComplexityGuardOptions, DialectType, ParseOptions,
    };
    for (guard, code) in [
        (
            ComplexityGuardOptions {
                max_input_bytes: Some(2),
                ..Default::default()
            },
            "E_GUARD_INPUT_TOO_LARGE",
        ),
        (
            ComplexityGuardOptions {
                max_ast_nodes: Some(0),
                ..Default::default()
            },
            "E_GUARD_AST_BUDGET_EXCEEDED",
        ),
        (
            ComplexityGuardOptions {
                max_parser_depth: Some(0),
                ..Default::default()
            },
            "E_GUARD_PARSER_DEPTH_EXCEEDED",
        ),
    ] {
        let error = parse_data_type_with_options(
            "DECIMAL(10, 2)",
            DialectType::Snowflake,
            &ParseOptions {
                complexity_guard: Some(guard),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains(code), "{error}");
    }
    assert!(parse_data_type_with_options(
        "DECIMAL(10, 2)",
        DialectType::Snowflake,
        &ParseOptions::default()
    )
    .is_ok());
}
