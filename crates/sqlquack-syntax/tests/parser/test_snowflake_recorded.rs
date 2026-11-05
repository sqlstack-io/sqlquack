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
        AcceptanceTestCase {
            description: "a data metric function takes a bare query (recorded semantic error)",
            sql: "SELECT SNOWFLAKE . CORE . NULL_COUNT ( SELECT c1 FROM t1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an unqualified call takes no bare query",
            sql: "SELECT COUNT ( SELECT 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a model method needs a name before ! (recorded errno 1003)",
            sql: "SELECT ( m1 )!predict ( * ) FROM t1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "MODEL( ) takes names, not strings (recorded errno 1003)",
            sql: "SELECT MODEL ( 'v1' )!predict ( 1 ) FROM t1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a method call needs a method after ! (recorded errno 1003)",
            sql: "SELECT svc1! ( v1 , v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "MODEL( )!method parses (recorded success)",
            sql: "SELECT MODEL ( m1 , v1 )!predict ( 1 ) FROM t1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a stage reference is no string (recorded errno 1003)",
            sql: "SELECT GET_STAGE_LOCATION ( @'v1' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a quoted stage name has no blanks (recorded errno 1003)",
            sql: "SELECT GET_STAGE_LOCATION ( @\"v1 Q\" ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a stage name is no reserved word (recorded errno 1003)",
            sql: "SELECT GET_STAGE_LOCATION ( @SELECT ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a quoted reserved stage name parses (recorded semantic error)",
            sql: "SELECT GET_STAGE_LOCATION ( @\"SELECT\" ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an interval type takes a precision (recorded success)",
            sql: "SELECT CAST ( NULL AS INTERVAL DAY TO SECOND ( 3 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an interval leading precision is 1 to 9 (recorded errno 1003)",
            sql: "SELECT CAST ( NULL AS INTERVAL YEAR ( 0 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "seconds after TO take precision 0 (recorded success)",
            sql: "SELECT CAST ( NULL AS INTERVAL DAY TO SECOND ( 0 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an interval type needs a field (recorded errno 1003)",
            sql: "SELECT CAST ( NULL AS INTERVAL ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TO needs a field after it (recorded errno 1003)",
            sql: "SELECT CAST ( NULL AS INTERVAL DAY TO ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a block declares a nested procedure (recorded success)",
            sql: "DECLARE p1 PROCEDURE ( v1 INT ) RETURNS TABLE ( c1 INT ) AS BEGIN RETURN 1 ; END ; BEGIN RETURN 1 ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a reserved word is no later name part (recorded errno 1003)",
            sql: "SELECT SYSTEM$GET_SERVICE_STATUS ( v1 . SELECT . svc1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a function call may leave out the schema (recorded semantic error)",
            sql: "SELECT SNOWFLAKE . . AGENT_RUN ( v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a name without a call keeps its schema (recorded errno 1003)",
            sql: "SELECT SYSTEM$GET_SERVICE_STATUS ( v1 . . svc1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "BEGIN WORK NAME names the transaction (recorded success)",
            sql: "BEGIN WORK NAME v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a trailing ESCAPE is an alias (recorded semantic error)",
            sql: "SELECT v1 LIKE ANY ( 'm' ) ESCAPE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ESCAPE takes a string (recorded success)",
            sql: "SELECT v1 ILIKE 'm' ESCAPE '!' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a lambda needs a parameter (recorded errno 1003)",
            sql: "SELECT ACCUMULATE ( 1 , ( ) -> 0 , ( s , x ) -> s + x , ( s1 , s2 ) -> s1 + s2 , s -> s ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a bind variable is no quoted name (recorded errno 1003)",
            sql: "SELECT :\"v1 Q\" ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a bind variable may be a keyword (recorded semantic error)",
            sql: "SELECT :SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a bare @ is a stage argument (recorded semantic error)",
            sql: "SELECT GET_STAGE_LOCATION ( @) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ALL takes a list of values (recorded semantic error)",
            sql: "SELECT 1 = ALL ( 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "STRING takes empty parentheses (recorded success)",
            sql: "SELECT CAST ( NULL AS STRING ( ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "statements chain with the flow operator (recorded success)",
            sql: "SELECT 1 ->> SELECT 2 ->> SELECT 3 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a block chains with the flow operator (recorded success)",
            sql: "BEGIN BEGIN NULL ; END ; END ->> SELECT 2 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "the flow operator needs a statement after it (recorded errno 1003)",
            sql: "SELECT 1 ->> 1 ;",
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
        ProjectionTestCase {
            description: "BEGIN WORK NAME keeps the name",
            sql: "BEGIN WORK NAME \"v1 Q\"",
            pointer: "/0/transaction/this/identifier/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "an interval type keeps its precision",
            sql: "SELECT CAST ( NULL AS INTERVAL DAY TO SECOND ( 3 ) )",
            pointer: "/0/select/expressions/0/cast/to/to",
            expected_value: json!("SECOND(3)"),
        },
        ProjectionTestCase {
            description: "a nested procedure keeps its return columns",
            sql: "DECLARE p1 PROCEDURE ( ) RETURNS TABLE ( c1 INT ) AS BEGIN RETURN 1 ; END ; BEGIN RETURN 1 ; END ;",
            pointer: "/0/scripting_block/declarations/0/procedure/returns_table/0/name",
            expected_value: json!("c1"),
        },
        ProjectionTestCase {
            description: "a call without a schema keeps the empty part",
            sql: "SELECT d1 . . f1 ( 1 )",
            pointer: "/0/select/expressions/0/method_call/this/column/name/name",
            expected_value: json!(""),
        },
        ProjectionTestCase {
            description: "a data metric function keeps its query argument",
            sql: "SELECT SNOWFLAKE . CORE . NULL_COUNT ( SELECT c1 FROM t1 )",
            pointer: "/0/select/expressions/0/method_call/args/0/subquery/this/select/from/expressions/0/table/name/name",
            expected_value: json!("t1"),
        },
        ProjectionTestCase {
            description: "the flow operator keeps the statement after it",
            sql: "SELECT 1 ->> SELECT 2",
            pointer: "/0/operator/expression/select/expressions/0/literal/value",
            expected_value: json!("2"),
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
