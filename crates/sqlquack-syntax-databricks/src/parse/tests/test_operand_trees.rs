//! Operands kept in shared nodes: a retention period, a datetime unit and a JSON path's segments.

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
        ValueTestCase {
            description: "a path's name is a key holding the name",
            sql: "SELECT c1:a1.b1 FROM t1",
            pointer: "/0/select/expressions/0/j_s_o_n_extract/expression/j_s_o_n_path/expressions/1/j_s_o_n_path_key/this/identifier/name",
            expected_json: "\"b1\"",
        },
        ValueTestCase {
            description: "a path's backquoted name stays quoted",
            sql: "SELECT c1:`a b` FROM t1",
            pointer: "/0/select/expressions/0/j_s_o_n_extract/expression/j_s_o_n_path/expressions/0/j_s_o_n_path_key/this/identifier/quoted",
            expected_json: "true",
        },
        ValueTestCase {
            description: "a path's bracketed string is a subscript of that string",
            sql: "SELECT c1:['a1'] FROM t1",
            pointer: "/0/select/expressions/0/j_s_o_n_extract/expression/j_s_o_n_path/expressions/0/j_s_o_n_path_subscript/this/literal/value",
            expected_json: "\"a1\"",
        },
        ValueTestCase {
            description: "a path's index is a subscript of that number",
            sql: "SELECT c1:a1[0] FROM t1",
            pointer: "/0/select/expressions/0/j_s_o_n_extract/expression/j_s_o_n_path/expressions/1/j_s_o_n_path_subscript/this/literal/literal_type",
            expected_json: "\"number\"",
        },
        ValueTestCase {
            description: "a path's bracketed name is a subscript of a key",
            sql: "SELECT c1:a1[b1] FROM t1",
            pointer: "/0/select/expressions/0/j_s_o_n_extract/expression/j_s_o_n_path/expressions/1/j_s_o_n_path_subscript/this/j_s_o_n_path_key/this/identifier/name",
            expected_json: "\"b1\"",
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
