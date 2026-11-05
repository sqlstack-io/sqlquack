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
        ProjectionTestCase {
            description: "COUNT takes a star with a quoted EXCLUDE name",
            sql: "SELECT COUNT(* EXCLUDE \"v1 Q\") FROM t1",
            pointer: "/aggregate_function/args/0/star/except/0/name",
            expected_value: json!("v1 Q"),
        },
        ProjectionTestCase {
            description: "COUNT takes a star with an EXCLUDE list",
            sql: "SELECT COUNT(* EXCLUDE (v1)) FROM t1",
            pointer: "/aggregate_function/args/0/star/except/0/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "a bare COUNT star is unchanged",
            sql: "SELECT COUNT(*) FROM t1",
            pointer: "/aggregate_function/args/0/star/except",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "HASH_AGG takes DISTINCT over several arguments",
            sql: "SELECT HASH_AGG(DISTINCT 1, 2)",
            pointer: "/aggregate_function/distinct",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "OBJECT_AGG takes DISTINCT over its key and value",
            sql: "SELECT OBJECT_AGG(DISTINCT v1, 1) FROM t1",
            pointer: "/aggregate_function/args/1/literal/value",
            expected_value: json!("1"),
        },
        ProjectionTestCase {
            description: "OBJECT_AGG without DISTINCT stays not distinct",
            sql: "SELECT OBJECT_AGG(v1, 1) FROM t1",
            pointer: "/aggregate_function/distinct",
            expected_value: json!(false),
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
            description: "an unquoted reserved keyword is no name (recorded errno 1003)",
            sql: "DROP PIPE SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a quoted reserved keyword is a name (recorded semantic error)",
            sql: "DROP PIPE \"SELECT\" ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
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
            description: "a join keeps LAST apart from FIRST",
            sql: "SELECT * FROM t1 JOIN t2 ON TRUE LAST ( a1 )",
            pointer: "/0/select/joins/0/first_last",
            expected_value: json!("LAST"),
        },
        ProjectionTestCase {
            description: "a join without FIRST or LAST leaves both out of the tree",
            sql: "SELECT * FROM t1 JOIN t2 ON TRUE",
            pointer: "/0/select/joins/0/first_last",
            expected_value: serde_json::Value::Null,
        },
        ProjectionTestCase {
            description: "a failover account keeps its organization as its own part",
            sql: "ALTER FAILOVER GROUP g1 ADD \"o.1\" . a1 TO ALLOWED_ACCOUNTS",
            pointer: "/0/alter_object/operands/0/table/schema/name",
            expected_value: json!("o.1"),
        },
        ProjectionTestCase {
            description: "a failover account keeps its quoting apart from its organization's",
            sql: "ALTER FAILOVER GROUP g1 ADD o1 . \"a.1\" TO ALLOWED_ACCOUNTS",
            pointer: "/0/alter_object/operands/0/table/name/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a connection's failover account keeps its organization",
            sql: "ALTER CONNECTION c1 DISABLE FAILOVER TO ACCOUNTS o1 . a1",
            pointer: "/0/alter_object/operands/0/table/schema/name",
            expected_value: json!("o1"),
        },
        ProjectionTestCase {
            description: "TRUNCATE ERROR_TABLE marks the error table",
            sql: "TRUNCATE ERROR_TABLE ( v1 )",
            pointer: "/0/truncate/error_table",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a plain TRUNCATE leaves error_table out of the tree",
            sql: "TRUNCATE TABLE v1",
            pointer: "/0/truncate/error_table",
            expected_value: serde_json::Value::Null,
        },
        ProjectionTestCase {
            description: "DROP SEQUENCE keeps RESTRICT apart from the default",
            sql: "DROP SEQUENCE v1 RESTRICT",
            pointer: "/0/drop_sequence/restrict",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "plain DROP SEQUENCE leaves RESTRICT out of the tree",
            sql: "DROP SEQUENCE v1",
            pointer: "/0/drop_sequence/restrict",
            expected_value: serde_json::Value::Null,
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

#[test]
fn given_statements_from_the_parser_only_disagreements_when_parsing_snowflake_then_the_verdict_matches_snowflake(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a bare expression is no statement",
            sql: "BREAK ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "BREAK inside a loop is a statement",
            sql: "BEGIN LOOP BREAK ; END LOOP ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "NULL alone is no statement",
            sql: "NULL ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a $$ procedure body is still read",
            sql: "CREATE PROCEDURE p1 ( ) RETURNS INT LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "UNSET needs a name",
            sql: "UNSET ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "UNSET ( ) needs a name",
            sql: "UNSET ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "UNSET takes a parenthesized list",
            sql: "UNSET ( v1 , v2 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ALTER SESSION UNSET needs a parameter",
            sql: "ALTER SESSION UNSET ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ALTER SESSION UNSET takes no string",
            sql: "ALTER SESSION UNSET 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ALTER SESSION UNSET takes a parameter name",
            sql: "ALTER SESSION UNSET TIMEZONE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "EXECUTE needs an object kind",
            sql: "EXECUTE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "EXECUTE ALERT needs a name",
            sql: "EXECUTE ALERT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "EXECUTE ALERT takes a quoted name",
            sql: r#"EXECUTE ALERT "v1 Q" ;"#,
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "EXECUTE ALERT refuses a reserved name",
            sql: "EXECUTE ALERT SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "EXECUTE TASK takes RETRY LAST",
            sql: "EXECUTE TASK v1 RETRY LAST ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "EXECUTE TASK takes RETRY GRAPH RUN GROUP",
            sql: "EXECUTE TASK v1 RETRY GRAPH RUN GROUP 'g' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "EXECUTE DCM PROJECT takes PURGE AS",
            sql: r#"EXECUTE DCM PROJECT v1 PURGE AS "d1" ;"#,
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "EXECUTE DCM PROJECT takes TEST ALL",
            sql: "EXECUTE DCM PROJECT v1 TEST ALL ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "EXPLAIN takes USING JSON",
            sql: "EXPLAIN USING JSON SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SHOW needs an object kind",
            sql: "SHOW ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "SHOW LIKE takes a string",
            sql: "SHOW VARIABLES LIKE 'v%' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SHOW LIKE refuses a name",
            sql: "SHOW VARIABLES LIKE m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "START TRANSACTION NAME needs a name",
            sql: "START TRANSACTION NAME ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "START TRANSACTION NAME takes a name",
            sql: "START TRANSACTION NAME t1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "USE SECONDARY ROLES needs a role",
            sql: "USE SECONDARY ROLES ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "USE SECONDARY ROLES takes a quoted role",
            sql: r#"USE SECONDARY ROLES "v1 Q" ;"#,
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an out-of-range exception number is a semantic error",
            sql: "DECLARE e1 EXCEPTION ( -19999 , 'm' ) ; BEGIN RETURN 1 ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DECLARE without a type is a semantic error",
            sql: "DECLARE x ; BEGIN RETURN 1 ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "LET still needs a type or a value",
            sql: "BEGIN LET x ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a CASE operand continues after its parentheses",
            sql: "BEGIN CASE ( 1 ) || '' WHEN '1' THEN RETURN 1 ; END CASE ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "FETCH INTO refuses a reserved word",
            sql: "DECLARE c1 CURSOR FOR SELECT 1 ; BEGIN OPEN c1 ; FETCH c1 INTO SELECT ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "OPEN outside a block is refused",
            sql: "OPEN c1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "( ) is no expression",
            sql: "SELECT ( ) IN ( ( 1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "IN needs a value",
            sql: "SELECT 1 IN ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "UNION BY needs NAME",
            sql: "SELECT 1 UNION BY SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "UNION BY NAME stays",
            sql: "SELECT 1 UNION BY NAME SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "LIMIT refuses a non-empty string",
            sql: "SELECT 1 FROM t1 LIMIT 'x' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "LIMIT takes an empty string as no limit",
            sql: "SELECT 1 FROM t1 LIMIT '' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a literal has no fields",
            sql: "SELECT 1.2.3 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a string has no star",
            sql: "SELECT 'v1' . * ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a subscript needs an index",
            sql: "SELECT ARRAY_CONSTRUCT ( 1 , 2 ) [ ] ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an unterminated $$ string is refused",
            sql: "SELECT $$m ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "POSITION refuses a trailing comma",
            sql: "SELECT POSITION ( 1 , 1 , ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP WITH LOCAL needs TIME ZONE",
            sql: "SELECT CAST ( NULL AS TIMESTAMP WITH LOCAL ZONE ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP WITH LOCAL TIME ZONE stays",
            sql: "SELECT CAST ( NULL AS TIMESTAMP WITH LOCAL TIME ZONE ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an OBJECT field needs NULL after NOT",
            sql: "SELECT CAST ( NULL AS OBJECT ( k1 INT NOT ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "VECTOR refuses a VARCHAR element",
            sql: "SELECT CAST ( NULL AS VECTOR ( VARCHAR , 3 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "VECTOR takes a FLOAT element",
            sql: "SELECT CAST ( NULL AS VECTOR ( FLOAT , 3 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "adjacent strings are refused",
            sql: "SELECT 'm' 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "COLLATE needs a string",
            sql: "SELECT 'm' COLLATE m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "COLLATE takes a string",
            sql: "SELECT 'm' COLLATE 'en' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an empty VALUES row is refused",
            sql: "INSERT INTO t1 VALUES ( 1 ) , ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "EXTRACT refuses a string part before FROM",
            sql: "SELECT EXTRACT ( 'm' FROM 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "DATE_PART refuses a string part before FROM",
            sql: "SELECT DATE_PART ( 'm' FROM 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description:
                "a PIVOT aggregate is a call, not a parenthesised column (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 PIVOT ( ( v1 ) FOR c1 IN ( 1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a PIVOT aggregate reads a column, not a string (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 PIVOT ( SUM ( 'v1' ) FOR c1 IN ( 1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a PIVOT aggregate alias is no string",
            sql: "SELECT 1 FROM t1 PIVOT ( SUM ( v1 ) 'v1' FOR c1 IN ( 1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a PIVOT IN list is not empty",
            sql: "SELECT 1 FROM t1 PIVOT ( SUM ( v1 ) FOR c1 IN ( ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ANY ORDER needs BY (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 PIVOT ( SUM ( v1 ) FOR c1 IN ( ANY ORDER ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ANY ORDER BY parses",
            sql: "SELECT 1 FROM t1 PIVOT ( SUM ( v1 ) FOR c1 IN ( ANY ORDER BY c1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a PIVOT IN value takes an alias without AS (recorded semantic error)",
            sql: "SELECT 1 FROM t1 PIVOT ( SUM ( v1 ) FOR c1 IN ( 1 v1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an UNPIVOT IN column takes a quoted alias without AS",
            sql: "SELECT 1 FROM t1 UNPIVOT ( c1 FOR v1 IN ( v1 \"SELECT\" ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an UNPIVOT IN list holds columns, not strings (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 UNPIVOT ( c1 FOR v1 IN ( 'v1' ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an UNPIVOT name column is no reserved keyword",
            sql: "SELECT 1 FROM t1 UNPIVOT ( c1 FOR SELECT IN ( v1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a join takes FIRST ( alias ) (recorded semantic error)",
            sql: "SELECT * FROM t1 JOIN t2 FIRST ( v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a directed join takes LAST ( alias ) after USING",
            sql: "SELECT * FROM t1 INNER DIRECTED JOIN t2 USING ( v1 ) LAST ( v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "FIRST ( alias ) refuses a reserved keyword (recorded errno 1003)",
            sql: "SELECT * FROM t1 JOIN t2 FIRST ( SELECT ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TRUNCATE takes ERROR_TABLE ( table ) (recorded semantic error)",
            sql: "TRUNCATE TABLE IF EXISTS ERROR_TABLE ( \"v1 Q\" ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "TRUNCATE ( name ) is no statement (recorded errno 1003)",
            sql: "TRUNCATE ( v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TABLE( ) needs an argument",
            sql: "SELECT * FROM TABLE ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TABLE( ) refuses a bare name",
            sql: "SELECT * FROM TABLE ( m1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TABLE( ) takes a string",
            sql: "SELECT * FROM TABLE ( 't1' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a named argument takes TABLE( ) around a bare name",
            sql: "SELECT * FROM TABLE ( m1!DETECT_ANOMALIES ( INPUT_DATA => TABLE ( v1 ) ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a named argument still refuses an empty TABLE( )",
            sql: "SELECT * FROM TABLE ( m1!DETECT_ANOMALIES ( INPUT_DATA => TABLE ( ) ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "IDENTIFIER( ) refuses a bare name",
            sql: "SELECT * FROM IDENTIFIER ( m1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "IDENTIFIER( ) takes a session variable",
            sql: "SELECT * FROM IDENTIFIER ( $v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SELECT INTO takes colon variables",
            sql: "SELECT 1 , 2 INTO :v1 , :v2 FROM t1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "CREATE TASK clones without an AS body (recorded success)",
            sql: "CREATE OR REPLACE TASK v1 CLONE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a clone needs a kind and a name (recorded errno 1003)",
            sql: "CREATE v1 CLONE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "CREATE OR needs REPLACE or ALTER (recorded errno 1003)",
            sql: "CREATE OR TASK v1 CLONE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a backup set restore names its identifier as a string (recorded errno 1003 without)",
            sql: "CREATE TABLE v1 FROM BACKUP SET v1 IDENTIFIER m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a share is named by its account (recorded errno 1003 without)",
            sql: "CREATE DATABASE v1 FROM SHARE v1 . ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an archive alias is no reserved keyword (recorded errno 1003)",
            sql: "CREATE TABLE v1 FROM ARCHIVE OF v1 SELECT WHERE 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a CREATE from a source needs its kind (recorded errno 1003)",
            sql: "CREATE v1 FROM SHARE v1 . v1 ;",
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
fn given_snowflake_statements_with_their_own_nodes_when_parsing_then_the_tree_names_their_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "UNSET keeps its names",
            sql: "UNSET ( v1 , v2 )",
            pointer: "/0/unset_statement/names/1/name",
            expected_value: json!("v2"),
        },
        ProjectionTestCase {
            description: "UNSET records the parentheses",
            sql: "UNSET ( v1 , v2 )",
            pointer: "/0/unset_statement/parenthesized",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "EXECUTE TASK GRAPH names the kind",
            sql: "EXECUTE TASK GRAPH v1 RETRY LAST",
            pointer: "/0/execute_object/kind",
            expected_value: json!("TASK GRAPH"),
        },
        ProjectionTestCase {
            description: "EXECUTE TASK names the action",
            sql: "EXECUTE TASK v1 RETRY GRAPH RUN GROUP 'g'",
            pointer: "/0/execute_object/action",
            expected_value: json!("RETRY GRAPH RUN"),
        },
        ProjectionTestCase {
            description: "EXECUTE DCM PROJECT keeps the PURGE alias",
            sql: "EXECUTE DCM PROJECT v1 PURGE AS \"d1\"",
            pointer: "/0/execute_object/alias/name",
            expected_value: json!("d1"),
        },
        ProjectionTestCase {
            description: "EXPLAIN keeps its USING format",
            sql: "EXPLAIN USING TABULAR SELECT 1",
            pointer: "/0/describe/style",
            expected_value: json!("USING TABULAR"),
        },
        ProjectionTestCase {
            description: "SELECT INTO keeps a colon variable",
            sql: "SELECT 1 INTO :v1",
            pointer: "/0/select/into/this/parameter/style",
            expected_value: json!("Colon"),
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

#[test]
fn given_a_snowflake_alter_of_an_application_database_or_service_kind_when_parsing_then_it_follows_the_grammar(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a package adds a version from a stage (recorded semantic error)",
            sql: "ALTER APPLICATION PACKAGE v1 ADD VERSION v1 USING '@d1.s1.st1/p1' LABEL = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a version needs USING (recorded errno 1003)",
            sql: "ALTER APPLICATION PACKAGE v1 ADD VERSION '@d1.s1.st1/p1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a release directive names its accounts, version and patch",
            sql: "ALTER APPLICATION PACKAGE v1 SET RELEASE DIRECTIVE v1 ACCOUNTS = ( o1.a1 ) VERSION = v1 PATCH = 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an unquoted reserved account name is rejected (recorded errno 1003)",
            sql: "ALTER APPLICATION PACKAGE v1 MODIFY RELEASE CHANNEL v1 ADD ACCOUNTS = ( v1 . SELECT ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an application upgrades from a stage (recorded semantic error)",
            sql: "ALTER APPLICATION v1 UPGRADE USING '@d1.s1.st1/p1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "UPGRADE USING needs a path (recorded errno 1003)",
            sql: "ALTER APPLICATION v1 UPGRADE USING ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a database replicates to accounts and may skip the edition check",
            sql: "ALTER DATABASE v1 ENABLE REPLICATION TO ACCOUNTS o1.a1 , o1.a2 IGNORE EDITION CHECK ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a linked catalog writes NONE or ALL",
            sql: "ALTER DATABASE v1 UPDATE LINKED_CATALOG SET ALLOWED_WRITE_OPERATIONS = ALL ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a linked catalog adds namespaces to a list",
            sql: "ALTER DATABASE IF EXISTS v1 UPDATE LINKED_CATALOG ADD ( 'n1' , 'n2' ) TO BLOCKED_NAMESPACES ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "Cortex Search unsets its primary key (recorded accepted)",
            sql: "ALTER CORTEX SEARCH SERVICE IF EXISTS v1 UNSET PRIMARY KEY ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "PRIMARY needs KEY (recorded errno 1003)",
            sql: "ALTER CORTEX SEARCH SERVICE v1 UNSET PRIMARY ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "half of CORTEX SEARCH SERVICE is no kind (recorded errno 1003)",
            sql: "ALTER CORTEX SEARCH v1 REFRESH ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "BACKUP needs SET (recorded errno 1003)",
            sql: "ALTER BACKUP v1 ADD BACKUP ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a backup set takes a legal hold",
            sql: "ALTER BACKUP SET v1 MODIFY BACKUP IDENTIFIER 'm' ADD LEGAL HOLD ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a semantic view adds a materialization",
            sql: "ALTER SEMANTIC VIEW v1 ADD MATERIALIZATION v1 WAREHOUSE = v1 AS DIMENSIONS v1 METRICS v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SEMANTIC needs VIEW (recorded errno 1003)",
            sql: "ALTER SEMANTIC v1 RENAME TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a Streamlit unsets several properties",
            sql: "ALTER STREAMLIT v1 UNSET TITLE , COMMENT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a Streamlit name is no string (recorded errno 1003)",
            sql: "ALTER STREAMLIT 'v1' COMMIT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a database role renames (recorded errno 2003, after parsing)",
            sql: "ALTER DATABASE ROLE v1 RENAME TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a database role unsets a tag (recorded errno 2003, after parsing)",
            sql: "ALTER DATABASE ROLE \"SELECT\" UNSET TAG v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an application role sets a comment (recorded accepted)",
            sql: "ALTER APPLICATION ROLE IF EXISTS v1 SET COMMENT = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "with no role name, ROLE names the database (recorded errno 2003)",
            sql: "ALTER DATABASE ROLE RENAME TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an application role needs a property to unset (recorded errno 1003)",
            sql: "ALTER APPLICATION ROLE v1 UNSET ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a database role name is no string (recorded errno 1003)",
            sql: "ALTER DATABASE ROLE 'v1' RENAME TO v1 ;",
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
fn given_a_snowflake_alter_of_an_application_database_or_service_kind_when_parsing_then_the_tree_keeps_the_action(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "the kind is named whole",
            sql: "ALTER APPLICATION PACKAGE v1 DROP VERSION v1",
            pointer: "/0/alter_object/kind",
            expected_value: json!("APPLICATION PACKAGE"),
        },
        ProjectionTestCase {
            description: "a release channel change keeps both levels",
            sql: "ALTER APPLICATION PACKAGE v1 MODIFY RELEASE CHANNEL c1 DROP VERSION v1",
            pointer: "/0/alter_object/action",
            expected_value: json!("MODIFY RELEASE CHANNEL DROP VERSION"),
        },
        ProjectionTestCase {
            description: "the edition check option is kept",
            sql: "ALTER DATABASE v1 ENABLE REPLICATION TO ACCOUNTS o1.a1 IGNORE EDITION CHECK",
            pointer: "/0/alter_object/action",
            expected_value: json!("ENABLE REPLICATION IGNORE EDITION CHECK"),
        },
        ProjectionTestCase {
            description: "an account keeps its unquoted organization as a separate part",
            sql: "ALTER APPLICATION PACKAGE v1 MODIFY RELEASE CHANNEL c1 ADD ACCOUNTS = ( o1 . \"a Q\" )",
            pointer: "/0/alter_object/operands/1/tuple/expressions/0/table/schema/quoted",
            expected_value: json!(false),
        },
        ProjectionTestCase {
            description: "an account keeps its quoted name as written",
            sql: "ALTER APPLICATION PACKAGE v1 MODIFY RELEASE CHANNEL c1 ADD ACCOUNTS = ( o1 . \"a Q\" )",
            pointer: "/0/alter_object/operands/1/tuple/expressions/0/table/name/name",
            expected_value: json!("a Q"),
        },
        ProjectionTestCase {
            description: "a quoted property name is not case-folded",
            sql: "ALTER ORGANIZATION ACCOUNT UNSET \"v1 Q\", foo",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("v1 Q"),
        },
        ProjectionTestCase {
            description: "a linked catalog change names its list",
            sql: "ALTER DATABASE v1 UPDATE LINKED_CATALOG REMOVE ( 'n1' ) FROM ALLOWED_NAMESPACES",
            pointer: "/0/alter_object/action",
            expected_value: json!("UPDATE LINKED_CATALOG REMOVE FROM ALLOWED_NAMESPACES"),
        },
        ProjectionTestCase {
            description: "IF EXISTS is kept",
            sql: "ALTER STREAMLIT IF EXISTS v1 RENAME TO v2",
            pointer: "/0/alter_object/if_exists",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a suspended backup policy names which",
            sql: "ALTER BACKUP SET v1 SUSPEND BACKUP EXPIRATION POLICY",
            pointer: "/0/alter_object/action",
            expected_value: json!("SUSPEND BACKUP EXPIRATION POLICY"),
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

#[test]
fn given_a_snowflake_describe_when_parsing_then_it_needs_a_kind_and_a_name() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a two-word kind is read whole (recorded semantic error)",
            sql: "DESC COMPUTE POOL v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "half a kind is no kind (recorded errno 1003)",
            sql: "DESC POOL v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a kind needs its name (recorded errno 1003)",
            sql: "DESC FILE FORMAT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a bare name is described as an object (recorded errno 2003)",
            sql: "DESC v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a word after the name is refused, not dropped",
            sql: "DESC TABLE v1 v2 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an argument type is no string (recorded errno 1003)",
            sql: "DESC FUNCTION v1 ( 'INT' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "AS RESOURCE comes before AGENT only",
            sql: "DESC AS RESOURCE STAGE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a transaction is described by its number (recorded errno 1003 without)",
            sql: "DESC TRANSACTION v1 ;",
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
fn given_a_snowflake_describe_when_parsing_then_the_tree_keeps_its_kind_name_and_options() {
    let test_cases = [
        ProjectionTestCase {
            description: "a one-word kind keeps the name after it",
            sql: "DESC WAREHOUSE v1",
            pointer: "/0/describe/target/table/name/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "DATABASE is the kind, not the name",
            sql: "DESCRIBE DATABASE v1",
            pointer: "/0/describe/kind",
            expected_value: json!("DATABASE"),
        },
        ProjectionTestCase {
            description: "a three-word kind is named whole",
            sql: "DESC CORTEX SEARCH SERVICE v1",
            pointer: "/0/describe/kind",
            expected_value: json!("CORTEX SEARCH SERVICE"),
        },
        ProjectionTestCase {
            description: "AS RESOURCE is kept",
            sql: "DESC AS RESOURCE AGENT v1",
            pointer: "/0/describe/style",
            expected_value: json!("AS RESOURCE"),
        },
        ProjectionTestCase {
            description: "the application keeps its quoting",
            sql: "DESC CONFIGURATION v1 IN APPLICATION \"a b\".v2",
            pointer: "/0/describe/properties/0",
            expected_value: json!(["in_application", "\"a b\".v2"]),
        },
        ProjectionTestCase {
            description: "a listing revision is kept",
            sql: "DESC LISTING v1 REVISION = DRAFT",
            pointer: "/0/describe/properties/0",
            expected_value: json!(["revision", "DRAFT"]),
        },
        ProjectionTestCase {
            description: "a transaction number is the target",
            sql: "DESC TRANSACTION 12",
            pointer: "/0/describe/target/literal/value",
            expected_value: json!("12"),
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

#[test]
fn given_a_snowflake_show_when_parsing_then_its_kind_and_clauses_follow_the_grammar() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a shown kind in its container (recorded semantic error)",
            sql: "SHOW OFFERS IN LISTING v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a kind that needs a container refuses a bare name (recorded errno 1003)",
            sql: "SHOW OFFERS IN v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "half a kind is no kind (recorded errno 1003)",
            sql: "SHOW POOLS ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "STARTS WITH takes a string (recorded errno 1003)",
            sql: "SHOW DATABASES STARTS WITH m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "WITH PRIVILEGES needs a privilege (recorded errno 1003)",
            sql: "SHOW DATABASES WITH PRIVILEGES ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a container another kind owns is refused (recorded errno 1003)",
            sql: "SHOW CHANNELS IN LISTING v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a reserved word is no part of a dotted name (recorded errno 1003)",
            sql: "SHOW REPLICATION DATABASES WITH PRIMARY v1 . SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TABLE stands alone as a container (recorded semantic error)",
            sql: "SHOW COLUMNS IN TABLE ;",
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
fn given_a_snowflake_show_when_parsing_then_the_tree_keeps_kind_container_and_operands() {
    let test_cases = [
        ProjectionTestCase {
            description: "a multi-word kind is named whole",
            sql: "SHOW COMPUTE POOL INSTANCE FAMILIES",
            pointer: "/0/show/this",
            expected_value: json!("COMPUTE POOL INSTANCE FAMILIES"),
        },
        ProjectionTestCase {
            description: "the container kind is kept",
            sql: "SHOW ENTITIES IN DCM PROJECT v1",
            pointer: "/0/show/scope_kind",
            expected_value: json!("DCM PROJECT"),
        },
        ProjectionTestCase {
            description: "the package after an artifact repository is kept",
            sql: "SHOW VERSIONS IN ARTIFACT REPOSITORY v1 FOR PACKAGE p1",
            pointer: "/0/show/for_target/table/name/name",
            expected_value: json!("p1"),
        },
        ProjectionTestCase {
            description: "the run of an experiment is kept",
            sql: "SHOW RUN METRICS IN EXPERIMENT v1 RUN r1",
            pointer: "/0/show/target/table/name/name",
            expected_value: json!("r1"),
        },
        ProjectionTestCase {
            description: "privileges are a list",
            sql: "SHOW DATABASES WITH PRIVILEGES USAGE, MODIFY",
            pointer: "/0/show/privileges",
            expected_value: json!(["USAGE", "MODIFY"]),
        },
        ProjectionTestCase {
            description: "the primary keeps its account and database apart",
            sql: "SHOW REPLICATION DATABASES WITH PRIMARY a1.\"d 1\"",
            pointer: "/0/show/target/table/schema/name",
            expected_value: json!("a1"),
        },
        ProjectionTestCase {
            description: "the primary database keeps its quoting",
            sql: "SHOW REPLICATION DATABASES WITH PRIMARY a1.\"d 1\"",
            pointer: "/0/show/target/table/name/quoted",
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

#[test]
fn given_a_documented_snowflake_element_when_parsing_then_the_tree_keeps_it() {
    let test_cases = [
        ProjectionTestCase {
            description: "SELECT ALL is kept, where it was dropped",
            sql: "SELECT ALL * FROM t1",
            pointer: "/0/select/all",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "DISTINCT is kept on an aggregate of two arguments",
            sql: "SELECT APPROX_COUNT_DISTINCT ( DISTINCT 1 , 1 )",
            pointer: "/0/select/expressions/0/function/distinct",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "TIMESTAMP WITHOUT TIME ZONE is TIMESTAMP_NTZ, not bare TIMESTAMP",
            sql: "SELECT CAST(NULL AS TIMESTAMP(3) WITHOUT TIME ZONE)",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_value: json!("TIMESTAMPNTZ(3)"),
        },
        ProjectionTestCase {
            description: "a transaction name is the name, quoting kept, NAME apart",
            sql: "START TRANSACTION NAME \"v1 Q\"",
            pointer: "/0/transaction/this/identifier/name",
            expected_value: json!("v1 Q"),
        },
        ProjectionTestCase {
            description: "a quoted transaction name stays quoted",
            sql: "START TRANSACTION NAME \"v1 Q\"",
            pointer: "/0/transaction/this/identifier/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "secondary roles are listed one by one, quoting kept",
            sql: "USE SECONDARY ROLES v1 , \"v2\"",
            pointer: "/0/use/roles/1/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a quoted RENAME target stays quoted",
            sql: "SELECT * RENAME v1 AS \"v1 Q\" FROM t1",
            pointer: "/0/select/expressions/0/star/rename/0/1/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a quoted PIVOT aggregate alias stays quoted",
            sql: "SELECT * FROM t1 PIVOT ( SUM ( v1 ) \"v1 Q\" FOR v2 IN ( 1 , 2 ) )",
            pointer: "/0/select/from/expressions/0/pivot/expressions/0/alias/alias/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a quoted UNPIVOT name column stays quoted",
            sql: "SELECT * FROM t1 UNPIVOT ( v1 FOR \"v1 Q\" IN ( a , b ) )",
            pointer: "/0/select/from/expressions/0/unpivot/name_column/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a quoted COMMENT ON name stays quoted",
            sql: "COMMENT ON TABLE \"v1\" IS 'm'",
            pointer: "/0/comment/this/identifier/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a quoted COMMENT ON COLUMN table stays quoted",
            sql: "COMMENT ON COLUMN \"v1 Q\" . v1 IS 'm'",
            pointer: "/0/comment/this/column/table/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a quoted part of a longer COMMENT ON COLUMN table keeps its quotes",
            sql: "COMMENT ON COLUMN \"a.b\" . c . d IS 'm'",
            pointer: "/0/comment/this/column/table/name",
            expected_value: json!("\"a.b\".c"),
        },
        ProjectionTestCase {
            description: "a cloned task keeps its source",
            sql: "CREATE TASK \"v1 Q\" CLONE db.v2",
            pointer: "/0/create_task/clone_source/schema/name",
            expected_value: json!("db"),
        },
        ProjectionTestCase {
            description: "a cloned task keeps its name's quoting",
            sql: "CREATE TASK \"v1 Q\" CLONE db.v2",
            pointer: "/0/create_task/name",
            expected_value: json!("\"v1 Q\""),
        },
        ProjectionTestCase {
            description: "a database replica keeps its source and properties",
            sql: "CREATE DATABASE v1 AS REPLICA OF a1 . \"d Q\" DATA_RETENTION_TIME_IN_DAYS = 1",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("DATA_RETENTION_TIME_IN_DAYS"),
        },
        ProjectionTestCase {
            description: "an archive keeps its alias apart from the condition",
            sql: "CREATE TRANSIENT TABLE v1 FROM ARCHIVE OF v2 AS a1 WHERE 1",
            pointer: "/0/create_object/operands/0/table/alias/name",
            expected_value: json!("a1"),
        },
        ProjectionTestCase {
            description: "a backup set restore keeps the set and its identifier",
            sql: "CREATE DYNAMIC TABLE v1 FROM BACKUP SET s1 IDENTIFIER 'm'",
            pointer: "/0/create_object/operands/1/literal/value",
            expected_value: json!("m"),
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
