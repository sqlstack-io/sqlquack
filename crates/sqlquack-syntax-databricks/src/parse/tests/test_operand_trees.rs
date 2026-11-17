//! Operands kept in shared nodes: a retention period, a path's schemas and a datetime unit.

use serde_json::Value;
use sqlquack_syntax::Parsed;

use crate::parse::main::parse_strict::parse_strict;
use crate::parse::tests::test_types::ValueTestCase;

#[test]
fn given_an_operand_with_a_shared_node_when_parsing_then_its_tree_uses_that_node() {
    let test_cases = [
        ValueTestCase {
            description: "a new schema's retention period is an interval of its unit",
            sql: "CREATE SCHEMA s1 RETAIN DROPPED FOR 2 DAYS",
            pointer: "/0/create_schema/properties/0/property/value/interval/unit/unit",
            expected_json: "\"Day\"",
        },
        ValueTestCase {
            description: "a retention period keeps a plural unit as written",
            sql: "CREATE SCHEMA s1 RETAIN DROPPED FOR 2 DAYS",
            pointer: "/0/create_schema/properties/0/property/value/interval/unit/use_plural",
            expected_json: "true",
        },
        ValueTestCase {
            description: "an altered schema's retention period is an interval of its count",
            sql: "ALTER SCHEMA s1 SET RETAIN DROPPED TO 1 HOUR",
            pointer: "/0/alter_object/operands/0/interval/this/literal/value",
            expected_json: "\"1\"",
        },
        ValueTestCase {
            description: "a path's schema keeps its catalog apart",
            sql: "SET PATH = system.builtin , workspace.s1",
            pointer: "/0/set_statement/items/0/value/tuple/expressions/1/table/schema/name",
            expected_json: "\"workspace\"",
        },
        ValueTestCase {
            description: "a datetime unit is a keyword, not a string",
            sql: "SELECT dateadd ( MICROSECOND , 1 , c1 )",
            pointer: "/0/select/expressions/0/function/args/0/var/this",
            expected_json: "\"MICROSECOND\"",
        },
        ValueTestCase {
            description: "a difference's datetime unit is a keyword, not a string",
            sql: "SELECT timestampdiff ( HOUR , c1 , c2 )",
            pointer: "/0/select/expressions/0/function/args/0/var/this",
            expected_json: "\"HOUR\"",
        },
    ];
    for test_case in test_cases {
        let parsed: Parsed = parse_strict(test_case.sql).expect(test_case.description);
        assert!(parsed.gaps.is_empty(), "{}", test_case.description);
        let tree: Value = serde_json::to_value(&parsed.statements).expect(test_case.description);
        assert_eq!(
            tree.pointer(test_case.pointer)
                .map(Value::to_string)
                .as_deref(),
            Some(test_case.expected_json),
            "{}",
            test_case.description
        );
    }
}
