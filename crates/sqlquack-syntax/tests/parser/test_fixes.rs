use serde_json::json;

use sqlquack_syntax::Dialect;

use crate::helpers::{accepted, cast_targets, projection_at};
use crate::test_types::{AcceptanceTestCase, CastTargetTestCase, ProjectionTestCase};

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

#[test]
fn given_a_parenthesized_typed_lambda_when_parsing_then_it_keeps_names_and_types() {
    let test_cases = [
        ProjectionTestCase {
            description: "the parameter's name",
            sql: "SELECT TRANSFORM(a, (leg VARCHAR) -> leg::INT) FROM t",
            pointer: "/function/args/1/lambda/parameters/0/name",
            expected_value: json!("leg"),
        },
        ProjectionTestCase {
            description: "the parameter's type",
            sql: "SELECT TRANSFORM(a, (leg VARCHAR) -> leg::INT) FROM t",
            pointer: "/function/args/1/lambda/parameter_types/0/data_type",
            expected_value: json!("var_char"),
        },
        ProjectionTestCase {
            description: "the second of two typed parameters' name",
            sql: "SELECT REDUCE(a, 0, (acc NUMBER(18, 2), i VARIANT) -> acc + i::INT) FROM t",
            pointer: "/function/args/2/lambda/parameters/1/name",
            expected_value: json!("i"),
        },
        ProjectionTestCase {
            description: "a parameterised parameter type keeps its precision and scale",
            sql: "SELECT REDUCE(a, 0, (acc NUMBER(18, 2), i VARIANT) -> acc + i::INT) FROM t",
            pointer: "/function/args/2/lambda/parameter_types/0/name",
            expected_value: json!("NUMBER(18, 2)"),
        },
        ProjectionTestCase {
            description: "the second parameter's type",
            sql: "SELECT REDUCE(a, 0, (acc NUMBER(18, 2), i VARIANT) -> acc + i::INT) FROM t",
            pointer: "/function/args/2/lambda/parameter_types/1/name",
            expected_value: json!("VARIANT"),
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
fn given_double_star_exponentiation_when_parsing_then_only_dialects_that_have_it_accept_it() {
    let test_cases = [
        AcceptanceTestCase {
            description: "Snowflake rejects ** (recorded errno 1003)",
            sql: "SELECT 3 ** 5",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake still accepts POWER",
            sql: "SELECT POWER(3, 5)",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DuckDB keeps ** as exponentiation",
            sql: "SELECT 3 ** 5",
            dialect: Dialect::DuckDB,
            expected_accepted: true,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            accepted(test_case.sql, test_case.dialect),
            test_case.expected_accepted,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_unpivot_in_list_when_parsing_snowflake_then_recorded_rejections_are_refused() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a string alias is a 1003 syntax error",
            sql: "SELECT * FROM orders UNPIVOT (amount FOR quarter IN (q1 AS 'first', q2 AS 'second'))",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an empty IN list is a 1003 syntax error",
            sql: "SELECT * FROM orders UNPIVOT(amount FOR quarter IN ())",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a trailing comma is a 1003 syntax error",
            sql: "SELECT * FROM orders UNPIVOT(amount FOR quarter IN (q1,))",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a plain column list still parses",
            sql: "SELECT * FROM orders UNPIVOT(amount FOR quarter IN (q1, q2))",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DuckDB keeps string aliases in UNPIVOT IN",
            sql: "SELECT * FROM orders UNPIVOT (amount FOR quarter IN (q1 AS 'first', q2 AS 'second'))",
            dialect: Dialect::DuckDB,
            expected_accepted: true,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            accepted(test_case.sql, test_case.dialect),
            test_case.expected_accepted,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_bare_trailing_limit_when_parsing_then_only_snowflake_accepts_it() {
    let test_cases = [
        AcceptanceTestCase {
            description: "Snowflake accepts a bare LIMIT at the end (recorded as accepted)",
            sql: "SELECT 1 LIMIT",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "Snowflake accepts a bare LIMIT before the statement's semicolon",
            sql: "WITH t AS (SELECT 1 AS a) SELECT a FROM t LIMIT;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "Snowflake rejects LIMIT OFFSET 1 (recorded errno 1003)",
            sql: "WITH t AS (SELECT 1 AS a) SELECT a FROM t LIMIT OFFSET 1",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "DuckDB still needs a LIMIT count",
            sql: "SELECT 1 LIMIT",
            dialect: Dialect::DuckDB,
            expected_accepted: false,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            accepted(test_case.sql, test_case.dialect),
            test_case.expected_accepted,
            "{}",
            test_case.description
        );
    }
}
