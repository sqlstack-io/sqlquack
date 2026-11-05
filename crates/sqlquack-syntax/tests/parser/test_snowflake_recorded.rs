use serde_json::json;

use sqlquack_syntax::Dialect;

use crate::helpers::{accepted, statement_at};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_recorded_snowflake_cases_when_parsing_then_acceptance_matches_snowflake() {
    let test_cases = [
        AcceptanceTestCase {
            description: "UNDROP takes a stage (recorded success)",
            sql: "UNDROP STAGE \"v1 Q\" ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "UNDROP takes a variable (recorded success)",
            sql: "UNDROP VARIABLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "CREATE TYPE names a data type (recorded success)",
            sql: "CREATE OR REPLACE TYPE IF NOT EXISTS v1 AS INT COMMENT = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a type's COMMENT is a string (recorded errno 1003)",
            sql: "CREATE TYPE v1 AS INT COMMENT = m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a table name is no reserved keyword (recorded errno 1003)",
            sql: "CREATE HYBRID TABLE SELECT AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a hybrid table needs columns, AS or LIKE (recorded errno 1003)",
            sql: "CREATE HYBRID TABLE v1 v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "USING TEMPLATE takes a query, not a value (recorded errno 1003)",
            sql: "CREATE TABLE v1 USING TEMPLATE ( 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "USING TEMPLATE takes a string (recorded success)",
            sql: "CREATE TABLE v1 USING TEMPLATE 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a function body is a string (recorded errno 1003)",
            sql: "CREATE FUNCTION v1 ( ) RETURNS INT AS m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a function name is no reserved keyword (recorded errno 1003)",
            sql: "CREATE FUNCTION SELECT ( ) RETURNS INT AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "BEGIN NAME needs a name (recorded errno 1003)",
            sql: "BEGIN NAME 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "BEGIN NAME names the transaction (recorded success)",
            sql: "BEGIN NAME v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a lone word before ( is no object kind (recorded errno 1003)",
            sql: "CREATE v1 ( ) RETURNS INT AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a window aggregate parses without arguments (recorded semantic error)",
            sql: "SELECT SUM ( ) OVER ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a PIVOT aggregate needs an argument (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 PIVOT ( SUM ( ) FOR c1 IN ( 1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "FLATTEN parses without arguments (recorded semantic error)",
            sql: "SELECT * FROM TABLE ( FLATTEN ( ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a table function needs TABLE( ), not parentheses (recorded errno 1003)",
            sql: "SELECT * FROM ( FLATTEN ( ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "AT needs its closing parenthesis (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 AT ( TIMESTAMP => TO_TIMESTAMP_LTZ ( '2026-01-01' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "CHANGES needs an INFORMATION value (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 CHANGES ( INFORMATION => ) AT ( TIMESTAMP => TO_TIMESTAMP_LTZ ( '2026-01-01' ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "CHANGES takes an INFORMATION value (recorded success)",
            sql: "SELECT 1 FROM t1 CHANGES ( INFORMATION => DEFAULT ) AT ( TIMESTAMP => TO_TIMESTAMP_LTZ ( '2026-01-01' ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "IN is no function (recorded errno 1003)",
            sql: "SELECT IN ( 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a type parameter is no string (recorded errno 1003)",
            sql: "SELECT CAST ( NULL AS PERIOD ( 'INT' ) ) ;",
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
fn given_snowflake_create_and_begin_name_when_parsing_then_the_tree_keeps_the_element() {
    let test_cases = [
        ProjectionTestCase {
            description: "BEGIN NAME keeps the name's quoting",
            sql: "BEGIN NAME \"v1 Q\"",
            pointer: "/0/transaction/this/identifier/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a Snowflake type keeps its data type",
            sql: "CREATE TYPE v1 AS INT",
            pointer: "/0/create_type/definition/Alias/data_type/data_type",
            expected_value: json!("int"),
        },
        ProjectionTestCase {
            description: "an aggregate without arguments stays a call with no arguments",
            sql: "SELECT SUM ( ) OVER ( )",
            pointer: "/0/select/expressions/0/window_function/this/function/args",
            expected_value: json!([]),
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
