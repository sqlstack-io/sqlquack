use serde_json::json;

use crate::helpers::projection_at;
use crate::test_types::ProjectionTestCase;

#[test]
fn given_an_index_followed_by_a_key_when_parsing_then_it_is_a_path_on_the_element() {
    let test_cases = [
        ProjectionTestCase {
            description: "SELECT v[3]:a FROM t extracts from the subscript",
            sql: "SELECT v[3]:a FROM t",
            pointer: "/j_s_o_n_extract/this/subscript/index/literal/value",
            expected_value: json!("3"),
        },
        ProjectionTestCase {
            description: "SELECT v[3]:a FROM t keeps the path a",
            sql: "SELECT v[3]:a FROM t",
            pointer: "/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("a"),
        },
        ProjectionTestCase {
            description: "SELECT v[0]:a.b FROM t extracts from the subscript",
            sql: "SELECT v[0]:a.b FROM t",
            pointer: "/j_s_o_n_extract/this/subscript/index/literal/value",
            expected_value: json!("0"),
        },
        ProjectionTestCase {
            description: "SELECT v[0]:a.b FROM t keeps the path a.b",
            sql: "SELECT v[0]:a.b FROM t",
            pointer: "/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("a.b"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            projection_at(test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
