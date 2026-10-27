use serde_json::json;

use crate::helpers::{cast_targets, projection_at};
use crate::test_types::{CastTargetTestCase, ProjectionTestCase};

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

#[test]
fn given_a_structured_type_after_double_colon_when_parsing_then_it_is_the_cast_form_type() {
    let test_cases = [
        CastTargetTestCase {
            description: "an OBJECT with typed fields",
            type_text: "OBJECT(a INT, b VARCHAR)",
            expected_same: true,
        },
        CastTargetTestCase {
            description: "an OBJECT with a NOT NULL field and a parameterised type",
            type_text: "OBJECT(a VARCHAR NOT NULL, b NUMBER(10,2))",
            expected_same: true,
        },
        CastTargetTestCase {
            description: "an OBJECT in an ARRAY, holding an ARRAY",
            type_text: "ARRAY(OBJECT(k ARRAY(INT)))",
            expected_same: true,
        },
        CastTargetTestCase {
            description: "an OBJECT as a MAP's value",
            type_text: "MAP(VARCHAR, OBJECT(z NUMBER(10,2) NOT NULL))",
            expected_same: true,
        },
        CastTargetTestCase {
            description: "a lower-case OBJECT",
            type_text: "object(a int, b varchar)",
            expected_same: true,
        },
    ];

    for test_case in test_cases {
        let (double_colon, cast) = cast_targets(test_case.type_text);
        assert_eq!(
            double_colon == cast && !cast.is_null(),
            test_case.expected_same,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_double_colon_object_type_when_parsing_then_its_fields_keep_names_types_and_not_null() {
    let test_cases = [
        ProjectionTestCase {
            description: "the first field's name",
            sql: "SELECT x::OBJECT(a VARCHAR NOT NULL, b ARRAY(INT))",
            pointer: "/cast/to/fields/0/0",
            expected_value: json!("a"),
        },
        ProjectionTestCase {
            description: "the first field is NOT NULL",
            sql: "SELECT x::OBJECT(a VARCHAR NOT NULL, b ARRAY(INT))",
            pointer: "/cast/to/fields/0/2",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "the second field's ARRAY element type",
            sql: "SELECT x::OBJECT(a VARCHAR NOT NULL, b ARRAY(INT))",
            pointer: "/cast/to/fields/1/1/element_type/data_type",
            expected_value: json!("int"),
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
