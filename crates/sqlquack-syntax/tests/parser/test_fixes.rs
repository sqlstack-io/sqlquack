use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, cast_targets, projection_at, statement_at, statements};
use crate::test_types::{
    AcceptanceTestCase, CastTargetTestCase, ProjectionTestCase, StatementTestCase,
};

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

#[test]
fn given_a_set_operation_by_name_when_parsing_snowflake_then_only_union_takes_it() {
    let test_cases = [
        AcceptanceTestCase {
            description: "Snowflake accepts UNION ALL BY NAME",
            sql: "SELECT 3 AS id, 7 AS region UNION ALL BY NAME SELECT 11 AS region, 13 AS id",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "Snowflake rejects INTERSECT BY NAME (recorded errno 1003)",
            sql: "SELECT 1 AS id INTERSECT BY NAME SELECT 2 AS id",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake rejects MINUS BY NAME (recorded errno 1003)",
            sql: "SELECT 1 AS id MINUS BY NAME SELECT 2 AS id",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake rejects an ON column list after BY NAME (recorded errno 1003)",
            sql: "SELECT 1 AS a, 2 AS b UNION ALL BY NAME ON (a) SELECT 3 AS b, 4 AS a",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake rejects a LEFT set operation (recorded errno 1003)",
            sql: "SELECT 1 AS a, 2 AS b LEFT UNION ALL BY NAME SELECT 3 AS b, 4 AS a",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake keeps INTERSECT ALL for analysis to refuse (errno 2)",
            sql: "SELECT 1 INTERSECT ALL SELECT 2",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DuckDB keeps INTERSECT BY NAME",
            sql: "SELECT 1 AS id INTERSECT BY NAME SELECT 2 AS id",
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
fn given_a_reserved_keyword_after_a_select_item_when_parsing_snowflake_then_it_is_no_alias() {
    let test_cases = [
        AcceptanceTestCase {
            description: "Snowflake rejects GROUP as an implicit alias (recorded errno 1003)",
            sql: "SELECT 1 GROUP ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake rejects ORDER as an implicit alias (recorded errno 1003)",
            sql: "SELECT 1 ORDER ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake keeps a non-reserved implicit alias",
            sql: "SELECT 1 v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "Snowflake keeps a quoted reserved word as an alias",
            sql: "SELECT 1 \"GROUP\" ;",
            dialect: Dialect::Snowflake,
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
fn given_a_select_list_when_parsing_snowflake_then_it_needs_an_item_and_top_needs_an_integer() {
    let test_cases = [
        AcceptanceTestCase {
            description: "Snowflake rejects SELECT with no items (recorded errno 1003)",
            sql: "SELECT FROM orders",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake rejects TOP with a string count (recorded errno 1003)",
            sql: "SELECT TOP 'x' 1 FROM t1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake rejects TOP 1 with no select item (recorded errno 1003)",
            sql: "SELECT TOP 1 FROM t1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake reads TOP before a name as a column (recorded semantic error)",
            sql: "SELECT TOP v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "Snowflake keeps TOP with an integer count",
            sql: "SELECT TOP 5 c1 FROM t1 ;",
            dialect: Dialect::Snowflake,
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
fn given_auditor_tree_findings_when_parsing_faithfully_then_the_tree_records_what_was_written() {
    let test_cases = [
        ProjectionTestCase {
            description: "DATEDIFF's date part is a date part, not a column",
            sql: "SELECT DATEDIFF(DAY, 1, 1)",
            pointer: "/function/args/0/var/this",
            expected_value: json!("DAY"),
        },
        ProjectionTestCase {
            description: "EXTRACT's comma form takes a date part",
            sql: "SELECT EXTRACT(DAY, 1)",
            pointer: "/function/args/0/var/this",
            expected_value: json!("DAY"),
        },
        ProjectionTestCase {
            description: "TIMEDIFF's date part is a date part",
            sql: "SELECT TIMEDIFF(DAY, 1, 1)",
            pointer: "/function/args/0/var/this",
            expected_value: json!("DAY"),
        },
        ProjectionTestCase {
            description: "TIMESTAMPDIFF's date part is a date part",
            sql: "SELECT TIMESTAMPDIFF(DAY, 1, 1)",
            pointer: "/function/args/0/var/this",
            expected_value: json!("DAY"),
        },
        ProjectionTestCase {
            description: "TRUNC's date part is a date part",
            sql: "SELECT TRUNC(1, DAY)",
            pointer: "/function/args/1/var/this",
            expected_value: json!("DAY"),
        },
        ProjectionTestCase {
            description: "TRUNC's numeric scale column stays a column",
            sql: "SELECT TRUNC(1, c1)",
            pointer: "/function/args/1/column/name/name",
            expected_value: json!("c1"),
        },
        ProjectionTestCase {
            description: "unary plus keeps its operator",
            sql: "SELECT + 1",
            pointer: "/pos/this/literal/value",
            expected_value: json!("1"),
        },
        ProjectionTestCase {
            description: "CURRENT_ORGANIZATION_USER without parentheses is a call",
            sql: "SELECT CURRENT_ORGANIZATION_USER",
            pointer: "/function/no_parens",
            expected_value: json!(true),
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
fn given_order_by_all_when_parsing_then_it_is_the_keyword_with_its_direction() {
    let test_cases = [
        StatementTestCase {
            description: "ORDER BY ALL DESC keeps its direction",
            sql: "SELECT 1 ORDER BY ALL DESC",
            pointer: "/0/select/order_by/all/desc",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "ORDER BY ALL has no sort keys",
            sql: "SELECT 1 ORDER BY ALL DESC",
            pointer: "/0/select/order_by/expressions",
            expected_value: json!([]),
        },
        StatementTestCase {
            description: "a quoted ALL is a column, not the keyword",
            sql: "SELECT 1 ORDER BY \"ALL\"",
            pointer: "/0/select/order_by/all",
            expected_value: json!(null),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            statement_at(test_case.sql, test_case.pointer),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_snowflake_drop_of_another_object_kind_when_parsing_then_it_takes_that_kinds_options() {
    let test_cases = [
        AcceptanceTestCase {
            description: "Snowflake drops an agent (recorded semantic error, so it parsed)",
            sql: "DROP AGENT v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a two-word kind takes IF EXISTS (recorded accepted)",
            sql: "DROP COMPUTE POOL IF EXISTS v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a service takes FORCE (recorded accepted)",
            sql: "DROP SERVICE IF EXISTS v1 FORCE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an external table takes RESTRICT (recorded semantic error)",
            sql: "DROP EXTERNAL TABLE v1 RESTRICT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an agent takes no FORCE",
            sql: "DROP AGENT v1 FORCE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description:
                "a stage needs a name; the Command fallback accepted this (recorded errno 1003)",
            sql: "DROP STAGE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a string is no name (recorded errno 1003)",
            sql: "DROP AGENT 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "IF without EXISTS (recorded errno 1003)",
            sql: "DROP AGENT IF v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "half of a two-word kind (recorded errno 1003)",
            sql: "DROP COMPUTE v1 ;",
            dialect: Dialect::Snowflake,
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

#[test]
fn given_a_snowflake_drop_of_another_object_kind_when_parsing_then_the_tree_names_the_kind() {
    let test_cases = [
        ProjectionTestCase {
            description: "a two-word kind is named whole",
            sql: "DROP APPLICATION PACKAGE IF EXISTS v1",
            pointer: "/0/drop_object/kind",
            expected_value: json!("APPLICATION PACKAGE"),
        },
        ProjectionTestCase {
            description: "IF EXISTS is kept",
            sql: "DROP APPLICATION PACKAGE IF EXISTS v1",
            pointer: "/0/drop_object/if_exists",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "STREAMLIT is not read as STREAM",
            sql: "DROP STREAMLIT v1",
            pointer: "/0/drop_object/kind",
            expected_value: json!("STREAMLIT"),
        },
        ProjectionTestCase {
            description: "a service keeps FORCE",
            sql: "DROP SERVICE v1 FORCE",
            pointer: "/0/drop_object/force",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "DROP SCHEMA keeps RESTRICT apart from the default",
            sql: "DROP SCHEMA v1 RESTRICT",
            pointer: "/0/drop_schema/restrict",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "plain DROP SCHEMA leaves RESTRICT out of the tree",
            sql: "DROP SCHEMA v1",
            pointer: "/0/drop_schema/restrict",
            expected_value: serde_json::Value::Null,
        },
        ProjectionTestCase {
            description: "DROP SCHEMA keeps CASCADE",
            sql: "DROP SCHEMA v1 CASCADE",
            pointer: "/0/drop_schema/cascade",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "an external table keeps RESTRICT",
            sql: "DROP EXTERNAL TABLE v1 RESTRICT",
            pointer: "/0/drop_object/restrict",
            expected_value: json!(true),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            statements(test_case.sql, Dialect::Snowflake, CallMode::Faithful)
                .pointer(test_case.pointer)
                .cloned()
                .unwrap_or_default(),
            test_case.expected_value,
            "{}",
            test_case.description
        );
    }
}
