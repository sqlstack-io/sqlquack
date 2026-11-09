use serde_json::json;

use crate::parser::tests::helpers::configured_outcome;
use crate::parser::tests::helpers::duckdb_outcome;
use crate::parser::tests::helpers::kind_at;
use crate::parser::tests::helpers::nested_calls;
use crate::parser::tests::helpers::value_at;
use crate::parser::tests::helpers::wrapped_lambda;
use crate::parser::tests::test_types::GuardedParseTestCase;
use crate::parser::tests::test_types::ParseKindTestCase;
use crate::parser::tests::test_types::ParseValueTestCase;
use sqlquack_syntax::tokens::models::DialectType;

#[test]
fn given_parser_depth_limits_when_parsing_then_deep_input_is_refused_by_its_guard() {
    let test_cases = [
        GuardedParseTestCase {
            description: "a zero depth limit refuses even SELECT 1",
            sql: "SELECT 1".to_owned(),
            max_parser_depth: Some(0),
            outcome: configured_outcome,
            expected_outcome_part: "E_GUARD_PARSER_DEPTH_EXCEEDED",
        },
        GuardedParseTestCase {
            description: "twelve unary prefixes exceed a depth of eight",
            sql: format!("SELECT {}1", "~ ".repeat(12)),
            max_parser_depth: Some(8),
            outcome: configured_outcome,
            expected_outcome_part: "E_GUARD_PARSER_DEPTH_EXCEEDED",
        },
        GuardedParseTestCase {
            description: "twelve unary prefixes fit a depth of 32",
            sql: format!("SELECT {}1", "~ ".repeat(12)),
            max_parser_depth: Some(32),
            outcome: configured_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "twelve unary prefixes fit without a limit",
            sql: format!("SELECT {}1", "~ ".repeat(12)),
            max_parser_depth: None,
            outcome: configured_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "the depth limit is per statement",
            sql: "SELECT ~1, ~2, ~3; SELECT ~4".to_owned(),
            max_parser_depth: Some(8),
            outcome: configured_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "an unclosed parenthesis is still an error",
            sql: "SELECT (1".to_owned(),
            max_parser_depth: None,
            outcome: configured_outcome,
            expected_outcome_part: "Parse error",
        },
        GuardedParseTestCase {
            description: "the function nesting guard is independent",
            sql: nested_calls(100, "abs"),
            max_parser_depth: None,
            outcome: configured_outcome,
            expected_outcome_part: "E_GUARD_FUNCTION_NESTING_DEPTH_EXCEEDED",
        },
        GuardedParseTestCase {
            description: "16 lambda wrappers consume parser depth",
            sql: wrapped_lambda(16),
            max_parser_depth: Some(8),
            outcome: duckdb_outcome,
            expected_outcome_part: "E_GUARD_PARSER_DEPTH_EXCEEDED",
        },
        GuardedParseTestCase {
            description: "16 lambda wrappers fit a depth of 256",
            sql: wrapped_lambda(16),
            max_parser_depth: Some(256),
            outcome: duckdb_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "32 lambda wrappers consume parser depth",
            sql: wrapped_lambda(32),
            max_parser_depth: Some(8),
            outcome: duckdb_outcome,
            expected_outcome_part: "E_GUARD_PARSER_DEPTH_EXCEEDED",
        },
        GuardedParseTestCase {
            description: "32 lambda wrappers fit a depth of 256",
            sql: wrapped_lambda(32),
            max_parser_depth: Some(256),
            outcome: duckdb_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "64 lambda wrappers consume parser depth",
            sql: wrapped_lambda(64),
            max_parser_depth: Some(8),
            outcome: duckdb_outcome,
            expected_outcome_part: "E_GUARD_PARSER_DEPTH_EXCEEDED",
        },
        GuardedParseTestCase {
            description: "64 lambda wrappers fit a depth of 256",
            sql: wrapped_lambda(64),
            max_parser_depth: Some(256),
            outcome: duckdb_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "SELECT list_reduce([1,2], ((x, y) -> x + y), ((0 + 1))) leaves the following arguments intact",
            sql: "SELECT list_reduce([1,2], ((x, y) -> x + y), ((0 + 1)))".to_owned(),
            max_parser_depth: None,
            outcome: duckdb_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "SELECT list_transform([1,2], ((x -> x + 1))), ((2 + 3)) leaves the following arguments intact",
            sql: "SELECT list_transform([1,2], ((x -> x + 1))), ((2 + 3))".to_owned(),
            max_parser_depth: None,
            outcome: duckdb_outcome,
            expected_outcome_part: "parsed",
        },
        GuardedParseTestCase {
            description: "SELECT list_transform([1,2], x -> x + 1), list_transform([3], ((x -> x))) leaves the following arguments intact",
            sql: "SELECT list_transform([1,2], x -> x + 1), list_transform([3], ((x -> x)))".to_owned(),
            max_parser_depth: None,
            outcome: duckdb_outcome,
            expected_outcome_part: "parsed",
        },
    ];

    for test_case in test_cases {
        let outcome = (test_case.outcome)(&test_case.sql, test_case.max_parser_depth);
        assert!(
            outcome.contains(test_case.expected_outcome_part),
            "{}: {outcome}",
            test_case.description
        );
    }
}

#[test]
fn given_duckdb_json_arrows_when_parsing_then_extraction_binds_around_arithmetic() {
    let test_cases = [
        ParseKindTestCase {
            description: "a scalar arrow owns its arithmetic path",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload ->> 0 + 1",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("json_extract_scalar"),
        },
        ParseKindTestCase {
            description: "the scalar arrow's path is the sum",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload ->> 0 + 1",
            pointer: "/0/select/expressions/0/json_extract_scalar/path",
            expected_kind: Ok("add"),
        },
        ParseKindTestCase {
            description: "concatenation follows scalar extraction",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload ->> 'a' || 'b'",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("concat"),
        },
        ParseKindTestCase {
            description: "concatenation's left side is the extraction",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload ->> 'a' || 'b'",
            pointer: "/0/select/expressions/0/concat/left",
            expected_kind: Ok("json_extract_scalar"),
        },
        ParseKindTestCase {
            description: "comparison follows scalar extraction",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload ->> 0 + 1 = other",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("eq"),
        },
        ParseKindTestCase {
            description: "comparison's left side is the extraction",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload ->> 0 + 1 = other",
            pointer: "/0/select/expressions/0/eq/left",
            expected_kind: Ok("json_extract_scalar"),
        },
        ParseKindTestCase {
            description: "a scalar arrow after a JSON arrow extracts from it",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload -> 'a' ->> 0",
            pointer: "/0/select/expressions/0/json_extract_scalar/this",
            expected_kind: Ok("json_extract"),
        },
        ParseKindTestCase {
            description: "a scalar arrow after a concatenated path extracts from the JSON arrow",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload -> 'a' || 'b' ->> 0",
            pointer: "/0/select/expressions/0/json_extract_scalar/this",
            expected_kind: Ok("json_extract"),
        },
        ParseKindTestCase {
            description: "a parenthesized scalar arrow stays inside the path",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload -> ('a' || ('b' ->> 0))",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("json_extract"),
        },
        ParseKindTestCase {
            description: "a scalar arrow inside a call stays inside the path",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT payload -> concat('a', other ->> 0)",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("json_extract"),
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
fn given_quoted_keyword_relation_aliases_when_parsing_duckdb_then_their_spelling_is_kept() {
    let test_cases = [
        ParseValueTestCase {
            description: "the quoted keyword alias \"Owned\" without AS keeps its spelling",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT \"Owned\".id FROM samples \"Owned\"",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_value: Ok(json!("Owned")),
        },
        ParseValueTestCase {
            description: "the quoted keyword alias \"Owned\" without AS stays quoted",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT \"Owned\".id FROM samples \"Owned\"",
            pointer: "/0/select/from/expressions/0/table/alias/quoted",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "the quoted keyword alias \"Owned\" with AS keeps its spelling",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT \"Owned\".id FROM samples AS \"Owned\"",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_value: Ok(json!("Owned")),
        },
        ParseValueTestCase {
            description: "the quoted keyword alias \"Owned\" with AS stays quoted",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT \"Owned\".id FROM samples AS \"Owned\"",
            pointer: "/0/select/from/expressions/0/table/alias/quoted",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "the quoted keyword alias \"Match\" without AS keeps its spelling",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT \"Match\".id FROM samples \"Match\"",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_value: Ok(json!("Match")),
        },
        ParseValueTestCase {
            description: "the quoted keyword alias \"Match\" without AS stays quoted",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT \"Match\".id FROM samples \"Match\"",
            pointer: "/0/select/from/expressions/0/table/alias/quoted",
            expected_value: Ok(json!(true)),
        },
        ParseValueTestCase {
            description: "the quoted keyword alias \"Match\" with AS keeps its spelling",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT \"Match\".id FROM samples AS \"Match\"",
            pointer: "/0/select/from/expressions/0/table/alias/name",
            expected_value: Ok(json!("Match")),
        },
        ParseValueTestCase {
            description: "the quoted keyword alias \"Match\" with AS stays quoted",
            dialect: Some(DialectType::DuckDB),
            sql: "SELECT \"Match\".id FROM samples AS \"Match\"",
            pointer: "/0/select/from/expressions/0/table/alias/quoted",
            expected_value: Ok(json!(true)),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            value_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
