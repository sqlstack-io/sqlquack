use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_procedure_create_when_parsing_then_only_what_snowflake_parses_is_accepted() {
    let test_cases = [
        AcceptanceTestCase {
            description: "an argument's mode follows its name (recorded OK)",
            sql: "CREATE PROCEDURE v1 ( v1 OUTPUT INT ) RETURNS INT LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "IN after a name is a mode too (recorded OK)",
            sql: "DEFINE PROCEDURE d1.s1.v1 ( v1 IN INT ) RETURNS INT LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a procedure named for a mode word still parses",
            sql: "CREATE PROCEDURE p1 ( input INT , output INT ) RETURNS INT LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a runtime version may be a bare word (recorded parse, then semantic error)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = m1 HANDLER = 'm1.run' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "packages may be bare words (recorded parse, then semantic error)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = '3.10' PACKAGES = ( m1 ) HANDLER = 'm1.run' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an artifact repository may be a string (recorded parse, then semantic error)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = '3.10' ARTIFACT_REPOSITORY = 'v1' HANDLER = 'm1.run' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a runtime version is never the next property's key (recorded syntax error for functions)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = HANDLER = 'm1.run' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a scoped temporary table (recorded OK)",
            sql: "CREATE PROCEDURE SCOPED TEMPORARY TABLE \"v1 Q\" ( \"SELECT\" INT ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a scoped table is TEMP or TEMPORARY (recorded syntax error)",
            sql: "CREATE PROCEDURE SCOPED TABLE v1 ( v1 INT ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a scoped temporary table needs TABLE (recorded syntax error)",
            sql: "CREATE PROCEDURE SCOPED TEMP v1 ( v1 INT ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "each scoped column has a type (recorded syntax error)",
            sql: "CREATE PROCEDURE SCOPED TEMP TABLE v1 ( v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a column type is no string (recorded syntax error)",
            sql: "CREATE PROCEDURE SCOPED TEMP TABLE v1 ( v1 'INT' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a procedure named scoped is still a procedure",
            sql: "CREATE PROCEDURE scoped ( ) RETURNS INT LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
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
fn given_a_snowflake_procedure_create_when_parsing_then_the_tree_keeps_its_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "a mode after the name keeps the name",
            sql: "CREATE PROCEDURE p1 ( a1 OUT INT ) RETURNS INT LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$",
            pointer: "/0/create_procedure/parameters/0/name/name",
            expected_value: json!("a1"),
        },
        ProjectionTestCase {
            description: "a mode after the name is recorded as the mode",
            sql: "CREATE PROCEDURE p1 ( a1 OUTPUT INT ) RETURNS INT LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$",
            pointer: "/0/create_procedure/parameters/0/mode_text",
            expected_value: json!("OUTPUT"),
        },
        ProjectionTestCase {
            description: "a scoped temporary table keeps its second column",
            sql: "CREATE PROCEDURE SCOPED TEMP TABLE t1 ( c1 INT , c2 VARCHAR )",
            pointer: "/0/create_object/operands/0/tuple/expressions/1/column_def/name/name",
            expected_value: json!("c2"),
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
