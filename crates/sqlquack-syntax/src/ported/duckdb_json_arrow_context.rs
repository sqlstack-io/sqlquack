use crate::ported::compat::{parse_one, DialectType, Expression};

fn projected(sql: &str) -> Expression {
    let Expression::Select(select) = parse_one(sql, DialectType::DuckDB).unwrap() else {
        panic!("expected SELECT")
    };
    select.expressions[0].clone()
}

#[test]
fn parenthesized_lambdas_respect_parser_depth_and_leave_following_arguments_intact() {
    use crate::ported::compat::{parse_one_with_options, ComplexityGuardOptions, ParseOptions};
    for wrappers in [16, 32, 64] {
        let sql = format!(
            "SELECT list_transform([1,2], {}x -> x + 1{})",
            "(".repeat(wrappers),
            ")".repeat(wrappers)
        );
        let options = ParseOptions {
            complexity_guard: Some(ComplexityGuardOptions {
                max_parser_depth: Some(8),
                ..Default::default()
            }),
        };
        let error = parse_one_with_options(&sql, DialectType::DuckDB, &options)
            .expect_err("lambda wrappers must consume parser depth");
        assert!(
            error.to_string().contains("E_GUARD_PARSER_DEPTH_EXCEEDED"),
            "{error}"
        );
        let options = ParseOptions {
            complexity_guard: Some(ComplexityGuardOptions {
                max_parser_depth: Some(256),
                ..Default::default()
            }),
        };
        assert!(parse_one_with_options(&sql, DialectType::DuckDB, &options).is_ok());
    }
    for sql in [
        "SELECT list_reduce([1,2], ((x, y) -> x + y), ((0 + 1)))",
        "SELECT list_transform([1,2], ((x -> x + 1))), ((2 + 3))",
        "SELECT list_transform([1,2], x -> x + 1), list_transform([3], ((x -> x)))",
    ] {
        assert!(parse_one(sql, DialectType::DuckDB).is_ok(), "{sql}");
    }
}

#[test]
fn scalar_json_paths_bind_arithmetic_but_not_concatenation() {
    let expression = projected("SELECT payload ->> 0 + 1");
    let Expression::JsonExtractScalar(extract) = expression else {
        panic!("scalar arrow must own its arithmetic path")
    };
    assert!(matches!(extract.path, Expression::Add(_)));
    let expression = projected("SELECT payload ->> 'a' || 'b'");
    let Expression::Concat(concat) = expression else {
        panic!("concatenation must follow scalar extraction")
    };
    assert!(matches!(concat.left, Expression::JsonExtractScalar(_)));
    let expression = projected("SELECT payload ->> 0 + 1 = other");
    let Expression::Eq(comparison) = expression else {
        panic!("comparison must follow scalar extraction")
    };
    assert!(matches!(comparison.left, Expression::JsonExtractScalar(_)));
}

#[test]
fn mixed_arrow_chains_keep_extraction_on_the_input_side() {
    for sql in [
        "SELECT payload -> 'a' ->> 0",
        "SELECT payload -> 'a' || 'b' ->> 0",
    ] {
        let expression = projected(sql);
        let Expression::JsonExtractScalar(extract) = expression else {
            panic!("scalar extraction must follow JSON extraction: {sql}")
        };
        assert!(matches!(extract.this, Expression::JsonExtract(_)), "{sql}");
    }
    for sql in [
        "SELECT payload -> ('a' || ('b' ->> 0))",
        "SELECT payload -> concat('a', other ->> 0)",
    ] {
        let expression = projected(sql);
        assert!(matches!(expression, Expression::JsonExtract(_)), "{sql}");
    }
}
