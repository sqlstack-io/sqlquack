use crate::parser::tests::helpers::kind_at;
use crate::parser::tests::helpers::parse_error;
use crate::parser::tests::test_types::ParseErrorTestCase;
use crate::parser::tests::test_types::ParseKindTestCase;
use crate::tokens::models::DialectType;

#[test]
fn given_malformed_typed_expressions_when_parsing_then_they_stay_ordinary_calls() {
    let test_cases = [
        ParseKindTestCase {
            description: "a typed expression with one argument stays an ordinary call",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount) FROM orders",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("function"),
        },
        ParseKindTestCase {
            description: "a typed expression with three arguments stays an ordinary call",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount, 'BIGINT', 1) FROM orders",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("function"),
        },
        ParseKindTestCase {
            description: "a typed expression with a number for the type stays an ordinary call",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount, 1) FROM orders",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("function"),
        },
        ParseKindTestCase {
            description: "a typed expression with a column for the type stays an ordinary call",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount, note) FROM orders",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("function"),
        },
        ParseKindTestCase {
            description:
                "a typed expression whose text does not start a type stays an ordinary call",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount, '1') FROM orders",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("function"),
        },
        ParseKindTestCase {
            description: "a typed expression with empty type text stays an ordinary call",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount, '') FROM orders",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("function"),
        },
        ParseKindTestCase {
            description: "a typed expression with DISTINCT stays an ordinary call",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(DISTINCT amount, 'BIGINT') FROM orders",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("function"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_typed_expression_with_a_type_when_parsing_then_it_is_an_annotation() {
    let test_cases = [ParseKindTestCase {
        description: "a complete type makes the call an annotation",
        dialect: Some(DialectType::DuckDB),
        sql: "SELECT __polyglot_typed(amount, 'DECIMAL(10, 2)') FROM orders",
        pointer: "/0/select/expressions/0",
        expected_kind: Ok("annotated"),
    }];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_typed_expression_with_a_malformed_type_when_parsing_then_the_statement_fails() {
    let test_cases = [
        ParseErrorTestCase {
            description: "a type followed by more words fails",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount, 'NOT A TYPE (') FROM orders",
            expected_error: "Unexpected token after data type",
        },
        ParseErrorTestCase {
            description: "a reserved word as the type fails first in Snowflake",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT __polyglot_typed(amount, 'NOT A TYPE (') FROM orders",
            expected_error: "Reserved word NOT cannot be an unquoted type name",
        },
        ParseErrorTestCase {
            description: "a type with trailing tokens fails",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount, 'BIGINT extra') FROM orders",
            expected_error: "Unexpected token after data type",
        },
        ParseErrorTestCase {
            description: "a valid type followed by junk fails",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT __polyglot_typed(amount, 'INT garbage') FROM orders",
            expected_error: "Unexpected token after data type: garbage",
        },
        ParseErrorTestCase {
            description: "a type with unclosed parameters fails",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT __polyglot_typed(amount, 'DECIMAL(10,') FROM orders",
            expected_error: "Parse error",
        },
    ];

    for test_case in test_cases {
        let error = parse_error(test_case.dialect, test_case.sql);
        assert!(
            error.contains(test_case.expected_error),
            "{}: {error}",
            test_case.description
        );
    }
}
