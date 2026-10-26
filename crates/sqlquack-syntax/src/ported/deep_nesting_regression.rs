use crate::ported::compat::ComplexityGuardOptions;

#[test]
fn guard_json_rejects_invalid_limits_and_preserves_null() {
    for name in [
        "maxParserDepth",
        "maxInputBytes",
        "maxTokens",
        "maxAstNodes",
        "maxAstDepth",
        "maxParenthesisDepth",
        "maxFunctionCallDepth",
    ] {
        for value in ["-1", "true", "1.0", "\"128\"", "18446744073709551616"] {
            assert!(serde_json::from_str::<ComplexityGuardOptions>(&format!(
                "{{\"{name}\":{value}}}"
            ))
            .is_err());
        }
        for value in ["0", "128", "null"] {
            let options: ComplexityGuardOptions =
                serde_json::from_str(&format!("{{\"{name}\":{value}}}")).unwrap();
            assert_eq!(
                serde_json::to_value(options).unwrap()[name],
                serde_json::from_str::<serde_json::Value>(value).unwrap()
            );
        }
    }
    assert!(
        serde_json::from_str::<ComplexityGuardOptions>(r#"{"max_function_call_depth":128}"#)
            .is_err()
    );
}

fn build_nested_unary_function_sql(depth: usize, name: &str) -> String {
    let mut expr = "id".to_string();
    for _ in 0..depth {
        expr = format!("{name}({expr})");
    }
    format!("SELECT {expr} FROM t")
}

#[test]
fn parser_depth_options_and_independent_guards() {
    use crate::parser::{Parser, ParserConfig};
    use crate::tokens::Tokenizer;
    let parse = |sql: &str, limit| {
        let tokens = Tokenizer::default().tokenize(sql).unwrap();
        Parser::with_config(
            tokens,
            ParserConfig {
                complexity_guard: ComplexityGuardOptions {
                    max_parser_depth: limit,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .parse()
    };
    assert!(parse("SELECT 1", Some(0))
        .unwrap_err()
        .to_string()
        .contains("E_GUARD_PARSER_DEPTH_EXCEEDED"));
    let sql = format!("SELECT {}1", "~ ".repeat(12));
    assert!(parse(&sql, Some(8))
        .unwrap_err()
        .to_string()
        .contains("E_GUARD_PARSER_DEPTH_EXCEEDED"));
    assert!(parse(&sql, Some(32)).is_ok());
    assert!(parse(&sql, None).is_ok());
    // Siblings and sequential statements release their depth allocations.
    assert!(parse("SELECT ~1, ~2, ~3; SELECT ~4", Some(8)).is_ok());
    assert!(parse("SELECT (1", None).is_err());
    // Disabling parser depth does not disable function-nesting protection.
    let nested = build_nested_unary_function_sql(100, "abs");
    assert!(parse(&nested, None)
        .unwrap_err()
        .to_string()
        .contains("E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED"));
}
