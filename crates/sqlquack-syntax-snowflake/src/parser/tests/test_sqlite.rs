//! SQLite syntax polyglot parses: operator tiers, NOT IN, JSON arrows and identifier quotes.

use crate::parser::tests::helpers::kind_at;
use crate::parser::tests::test_types::ParseKindTestCase;
use sqlquack_syntax::tokens::models::DialectType;

#[test]
fn given_sqlite_syntax_when_parsing_then_nodes_match_polyglot() {
    let test_cases = [
        ParseKindTestCase {
            description: "NOT IN becomes NOT over IN",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT x NOT IN (1, 2) FROM t",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("not"),
        },
        ParseKindTestCase {
            description: "a comparison after a rewritten NOT IN keeps it parenthesized",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT x NOT IN (1) IS NULL FROM t",
            pointer: "/0/select/expressions/0/is_null/this",
            expected_kind: Ok("paren"),
        },
        ParseKindTestCase {
            description: "concatenation binds tighter than multiplication",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT 2 * 'a' || 'b'",
            pointer: "/0/select/expressions/0/mul/right",
            expected_kind: Ok("concat"),
        },
        ParseKindTestCase {
            description: "-> extracts JSON",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT j -> '$.a' FROM t",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("json_extract"),
        },
        ParseKindTestCase {
            description: "->> extracts a JSON scalar",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT j ->> '$.a' FROM t",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("json_extract_scalar"),
        },
        ParseKindTestCase {
            description: "& and | share one tier, left to right",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT 1 | 2 & 3",
            pointer: "/0/select/expressions/0/bitwise_and/left",
            expected_kind: Ok("bitwise_or"),
        },
        ParseKindTestCase {
            description: "shifts share the bitwise tier, below addition",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT 1 << 2 + 3",
            pointer: "/0/select/expressions/0/bitwise_left_shift/right",
            expected_kind: Ok("add"),
        },
        ParseKindTestCase {
            description: "an arrow in a call argument is JSON extraction, not a lambda",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT f(x -> '$.a') FROM t",
            pointer: "/0/select/expressions/0/function/args/0",
            expected_kind: Ok("json_extract"),
        },
        ParseKindTestCase {
            description: "a bracket-quoted identifier is a column",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT [order id] FROM t",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("column"),
        },
        ParseKindTestCase {
            description: "a backquoted identifier is a column",
            dialect: Some(DialectType::SQLite),
            sql: "SELECT `order id` FROM t",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("column"),
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
