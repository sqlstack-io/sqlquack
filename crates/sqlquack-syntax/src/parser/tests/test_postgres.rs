//! PostgreSQL syntax polyglot parses: operators, VARIADIC, array casts, END and EXPLAIN options.

use crate::parser::tests::helpers::kind_at;
use crate::parser::tests::test_types::ParseKindTestCase;
use crate::tokens::models::DialectType;

#[test]
fn given_postgres_syntax_when_parsing_then_nodes_match_polyglot() {
    let test_cases = [
        ParseKindTestCase {
            description: "a VARIADIC argument spreads an array",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT mleast(VARIADIC ARRAY[10, -1])",
            pointer: "/0/select/expressions/0/function/args/0",
            expected_kind: Ok("variadic"),
        },
        ParseKindTestCase {
            description: "VARIADIC before a cast array is one argument",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT mleast(VARIADIC ARRAY[]::NUMERIC[])",
            pointer: "/0/select/expressions/0/function/args/0/variadic/this",
            expected_kind: Ok("cast"),
        },
        ParseKindTestCase {
            description: "a cast takes an array type suffix",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT x::INT[] FROM t",
            pointer: "/0/select/expressions/0/cast/to",
            expected_kind: Ok("data_type"),
        },
        ParseKindTestCase {
            description: "# is exclusive or",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT 5 # 3",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("bitwise_xor"),
        },
        ParseKindTestCase {
            description: "^ is exponentiation",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT 2 ^ 3",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("power"),
        },
        ParseKindTestCase {
            description: "& and | share one tier, left to right",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT 1 | 2 & 3",
            pointer: "/0/select/expressions/0/bitwise_and/left",
            expected_kind: Ok("bitwise_or"),
        },
        ParseKindTestCase {
            description: "shifts share the bitwise tier, below addition",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT 1 << 2 + 3",
            pointer: "/0/select/expressions/0/bitwise_left_shift/right",
            expected_kind: Ok("add"),
        },
        ParseKindTestCase {
            description: "@ is absolute value",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT @ -5",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("abs"),
        },
        ParseKindTestCase {
            description: "@ glued to a column is its absolute value",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT @x FROM t",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("abs"),
        },
        ParseKindTestCase {
            description: "-> is JSON extraction, not a lambda",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT payload -> 'a' FROM t",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("json_extract"),
        },
        ParseKindTestCase {
            description: "END is COMMIT",
            dialect: Some(DialectType::PostgreSQL),
            sql: "END",
            pointer: "/0",
            expected_kind: Ok("commit"),
        },
        ParseKindTestCase {
            description: "END AND NO CHAIN keeps the chain flag",
            dialect: Some(DialectType::PostgreSQL),
            sql: "END WORK AND NO CHAIN",
            pointer: "/0/commit/chain",
            expected_kind: Ok("boolean"),
        },
        ParseKindTestCase {
            description: "an EXPLAIN option list is read",
            dialect: Some(DialectType::PostgreSQL),
            sql: "EXPLAIN (VERBOSE, COSTS OFF) SELECT 1",
            pointer: "/0/describe/target",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "EXPLAIN of a parenthesized query keeps the query",
            dialect: Some(DialectType::PostgreSQL),
            sql: "EXPLAIN (SELECT 1)",
            pointer: "/0",
            expected_kind: Ok("describe"),
        },
        ParseKindTestCase {
            description: "a dollar-quoted string is a literal",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT $$it's$$",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("literal"),
        },
        ParseKindTestCase {
            description: "comments nest",
            dialect: Some(DialectType::PostgreSQL),
            sql: "SELECT /* a /* b */ c */ 1",
            pointer: "/0/select/expressions/0",
            expected_kind: Ok("literal"),
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
