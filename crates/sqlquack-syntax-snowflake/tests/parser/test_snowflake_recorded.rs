use serde_json::json;

use sqlquack_syntax::Dialect;

use crate::helpers::{accepted, snowflake_kind, statement_at};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase, SnowflakeVerdictTestCase};

#[test]
fn given_recorded_snowflake_cases_when_parsing_then_acceptance_matches_snowflake() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a space between the dollar sign and a variable name (recorded errno 1003)",
            sql: "SELECT $ s",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a session variable that follows its dollar sign",
            sql: "SELECT $s",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
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
        AcceptanceTestCase {
            description: "** spreads an array into arguments (recorded success)",
            sql: "SELECT GREATEST ( ** [ 1 , 2 ] ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "** needs an array after it (recorded errno 1003)",
            sql: "SELECT GREATEST ( ** ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "IPADDRESS types a string literal (recorded success)",
            sql: "SELECT IPADDRESS '192.0.2.1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an interval's value is a string, not a name (recorded errno 1003)",
            sql: "SELECT INTERVAL m1 YEAR ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a path colon takes no string (recorded errno 1003)",
            sql: "SELECT v1 :'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "CAST takes nothing after its type (recorded errno 1003)",
            sql: "SELECT CAST ( NULL AS INTERVAL SECOND ( 9 ) , ( 3 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a structured ARRAY's element may be NOT NULL (recorded success)",
            sql: "SELECT CAST ( NULL AS ARRAY ( INT NOT NULL ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a structured MAP's value may be NOT NULL (recorded success)",
            sql: "SELECT CAST ( NULL AS MAP ( VARCHAR , INT NOT NULL ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "CALL needs its argument list closed (recorded errno 1003)",
            sql: "CALL SET_SYS_CONTEXT ( 'm' , 'm' , 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "CALL names a class method with the schema left out (recorded semantic error)",
            sql: "CALL SNOWFLAKE . . ANOMALY_INSIGHTS!DROP_MONITOR ( 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "CALL takes no empty argument (recorded errno 1003)",
            sql: "CALL SET_SYS_CONTEXT ( 'm' , , 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a frame bound is not a string (recorded errno 1003)",
            sql: "SELECT SUM ( ) OVER ( ROWS 'x' PRECEDING ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "EXECUTE USING POLICY_CONTEXT runs a query (recorded runtime error)",
            sql: "EXECUTE USING POLICY_CONTEXT ( v1 => 'm' ) AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "POLICY_CONTEXT ( ... ) AS needs a query (recorded errno 1003)",
            sql: "EXECUTE USING POLICY_CONTEXT ( v1 => 'm' ) AS 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "MODIFY DATA METRIC FUNCTION adds expectations (recorded semantic error)",
            sql: "ALTER TABLE v1 MODIFY DATA METRIC FUNCTION v1 ON ( v1 ) ADD EXPECTATION v1 ( 1 ) , v1 ( 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a data metric function's column list may be empty before TABLE (recorded semantic error)",
            sql: "ALTER TABLE v1 ADD DATA METRIC FUNCTION SNOWFLAKE . CORE . ROW_COUNT_DIFFERENCE ON ( , TABLE ( v1 ( v1 ) ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a data metric function's TABLE source names columns (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD DATA METRIC FUNCTION SNOWFLAKE . CORE . ROW_COUNT_DIFFERENCE ON ( v1 , TABLE ( v1 ( ) ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "MODIFY DATA METRIC FUNCTION may clear its filter (recorded semantic error)",
            sql: "ALTER TABLE v1 MODIFY DATA METRIC FUNCTION v1 ON ( v1 ) FILTER ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ADD DATA METRIC FUNCTION needs a filter condition (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD DATA METRIC FUNCTION v1 ON ( v1 ) FILTER ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ALTER VIEW adds data metric functions (recorded semantic error)",
            sql: "ALTER VIEW v1 ADD DATA METRIC FUNCTION v1 ON ( v1 ) , \"SELECT\" ON ( v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an interval SECOND qualifier takes one precision inside CAST (recorded errno 1003)",
            sql: "SELECT CAST ( INTERVAL '1' SECOND ( 9 ) , ( 3 ) AS VARCHAR ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "SECOND ( 9 ) , ( 3 ) is two select items (recorded success)",
            sql: "SELECT INTERVAL '1' SECOND ( 9 ) , ( 3 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a class's instances are shown with an empty schema (recorded semantic error)",
            sql: "SHOW SNOWFLAKE . . FORECAST ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a class's instances are shown in a container (recorded semantic error)",
            sql: "SHOW SNOWFLAKE . ML . FORECAST LIKE 'm' IN SCHEMA v1 LIMIT 1 FROM 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a class show takes no unnamed scope (recorded errno 1003)",
            sql: "SHOW SNOWFLAKE . ML . FORECAST IN ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a class name has a database (recorded errno 1003)",
            sql: "SHOW . ML . FORECAST ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "USE ROLE alone uses a database named ROLE (recorded semantic error)",
            sql: "USE ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "USE SCHEMA . v1 uses database SCHEMA's schema v1 (recorded semantic error)",
            sql: "USE SCHEMA . v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "USE needs a name (recorded errno 1003)",
            sql: "USE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "DESC RESULT alone describes an object named RESULT (recorded semantic error)",
            sql: "DESC RESULT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DESC RESULT keeps refusing an unclosed LAST_QUERY_ID (recorded errno 1003)",
            sql: "DESC RESULT LAST_QUERY_ID ( ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "EXECUTE TASK GRAPH alone runs a task named GRAPH (recorded semantic error)",
            sql: "EXECUTE TASK GRAPH ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a task named GRAPH is retried (recorded semantic error)",
            sql: "EXECUTE TASK GRAPH RETRY LAST ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a data metric function takes table arguments (recorded success)",
            sql: "CREATE DATA METRIC FUNCTION v1 ( v1 TABLE ( v1 INT ) , v1 TABLE ( v1 INT ) ) RETURNS NUMBER AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a secure data metric function takes every clause (recorded success)",
            sql: "CREATE OR REPLACE SECURE DATA METRIC FUNCTION IF NOT EXISTS v1 ( v1 TABLE ( v1 INT , v1 INT ) ) RETURNS NUMBER NOT NULL LANGUAGE SQL COMMENT = 'm' AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DEFINE defines a data metric function (recorded semantic error)",
            sql: "DEFINE DATA METRIC FUNCTION d1.s1.v1 ( v1 TABLE ( v1 INT ) ) RETURNS NUMBER COMMENT = m1 AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a data metric function's body is a string (recorded errno 1003)",
            sql: "CREATE DATA METRIC FUNCTION v1 ( v1 TABLE ( v1 INT ) ) RETURNS NUMBER AS m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a data metric function's argument is a table (recorded errno 1003)",
            sql: "CREATE DATA METRIC FUNCTION v1 ( v1 ( v1 INT ) ) RETURNS NUMBER AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a table argument's column has a type (recorded errno 1003)",
            sql: "CREATE DATA METRIC FUNCTION v1 ( v1 TABLE ( v1 ) ) RETURNS NUMBER AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a table argument's column type is no string (recorded errno 1003)",
            sql: "DEFINE DATA METRIC FUNCTION d1.s1.v1 ( v1 TABLE ( v1 'INT' ) ) RETURNS NUMBER AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a data metric function needs RETURNS (recorded errno 1003)",
            sql: "DEFINE DATA METRIC FUNCTION d1.s1.v1 ( v1 TABLE ( v1 INT ) ) NUMBER AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a data metric function's name part is no reserved word (recorded errno 1003)",
            sql: "DEFINE DATA METRIC FUNCTION d1.s1.SELECT ( v1 TABLE ( v1 INT ) ) RETURNS NUMBER AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a name before a string is a typed literal (recorded errno 2121)",
            sql: "SELECT v1 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a typed literal's type takes parameters (recorded errno 2121)",
            sql: "SELECT PERIOD ( 'INT' ) '[2026-01-01, 2026-01-02)' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP_NTZ before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP_NTZ '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP_LTZ before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP_LTZ '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP_TZ before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP_TZ '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMPNTZ before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMPNTZ '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMPLTZ before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMPLTZ '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMPTZ before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMPTZ '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "DATETIME before a string is no literal (recorded errno 1003)",
            sql: "SELECT DATETIME '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP_NTZ(3) before a string parses, as Snowflake rejects it later (recorded errno 2121)",
            sql: "SELECT TIMESTAMP_NTZ(3) '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP_TZ(3) before a string parses, as Snowflake rejects it later (recorded errno 2121)",
            sql: "SELECT TIMESTAMP_TZ(3) '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DATETIME(3) before a string parses, as Snowflake rejects it later (recorded errno 2121)",
            sql: "SELECT DATETIME(3) '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP(3) before a string parses, as Snowflake rejects it later (recorded errno 2121)",
            sql: "SELECT TIMESTAMP(3) '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "TIME(3) before a string parses, as Snowflake rejects it later (recorded errno 2121)",
            sql: "SELECT TIME(3) '03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP WITH TIME ZONE before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP WITH TIME ZONE '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP(3) WITH TIME ZONE before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP(3) WITH TIME ZONE '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP WITH LOCAL TIME ZONE before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP WITH LOCAL TIME ZONE '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP(3) WITH LOCAL TIME ZONE before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP(3) WITH LOCAL TIME ZONE '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP WITHOUT TIME ZONE before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP WITHOUT TIME ZONE '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP(3) WITHOUT TIME ZONE before a string is no literal (recorded errno 1003)",
            sql: "SELECT TIMESTAMP(3) WITHOUT TIME ZONE '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TIMESTAMP before a string is a literal (recorded)",
            sql: "SELECT TIMESTAMP '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DATE before a string is a literal (recorded)",
            sql: "SELECT DATE '2026-01-02 03:04:05' AS v FROM input",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a typed literal's parameters are not empty (recorded errno 1003)",
            sql: "SELECT PERIOD ( ) '[2026-01-01, 2026-01-02)' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a reserved word is no type (recorded errno 1003)",
            sql: "SELECT ILIKE 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an UNPIVOT IN item is a column, not a typed literal (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 UNPIVOT ( c1 FOR v1 IN ( v1 'v1' ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ALTER TABLE drops no partition LOCATION (recorded errno 1003)",
            sql: "ALTER TABLE v1 DROP PARTITION LOCATION 'm' ;",
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
            description: "a call without a schema keeps its database as the first name part",
            sql: "SELECT d1 . . f1 ( 1 )",
            pointer: "/0/select/expressions/0/call/name/0/name",
            expected_value: json!("d1"),
        },
        ProjectionTestCase {
            description: "a call without a schema keeps its quoted database quoted",
            sql: "SELECT \"d 1\"..\"f 1\"(1)",
            pointer: "/0/select/expressions/0/call/name/0/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a call without a schema keeps its quoted function quoted",
            sql: "SELECT \"d 1\"..\"f 1\"(1)",
            pointer: "/0/select/expressions/0/call/name/2/quoted",
            expected_value: json!(true),
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
        ProjectionTestCase {
            description: "** keeps the array it spreads",
            sql: "SELECT GREATEST ( ** [ 1 , 2 ] )",
            pointer: "/0/select/expressions/0/function/args/0/variadic/this/array_func/expressions/1/literal/value",
            expected_value: json!("2"),
        },
        ProjectionTestCase {
            description: "a class show keeps the class as its target",
            sql: "SHOW SNOWFLAKE . ML . FORECAST INSTANCES",
            pointer: "/0/show/target/table/catalog/name",
            expected_value: json!("SNOWFLAKE"),
        },
        ProjectionTestCase {
            description: "a class show records INSTANCES",
            sql: "SHOW SNOWFLAKE . ML . FORECAST INSTANCES",
            pointer: "/0/show/this",
            expected_value: json!("CLASS INSTANCES"),
        },
        ProjectionTestCase {
            description: "USE ROLE alone names a database ROLE",
            sql: "USE ROLE",
            pointer: "/0/use/this/name",
            expected_value: json!("ROLE"),
        },
        ProjectionTestCase {
            description: "USE ROLE alone has no kind",
            sql: "USE ROLE",
            pointer: "/0/use/kind",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "DESC of a lone name describes it",
            sql: "DESC RESULT",
            pointer: "/0/describe/target/table/name/name",
            expected_value: json!("RESULT"),
        },
        ProjectionTestCase {
            description: "EXECUTE TASK GRAPH alone runs a task named GRAPH",
            sql: "EXECUTE TASK GRAPH",
            pointer: "/0/execute_object/name/name/name",
            expected_value: json!("GRAPH"),
        },
        ProjectionTestCase {
            description: "a data metric function keeps its table argument's columns",
            sql: "CREATE DATA METRIC FUNCTION v1 ( v1 TABLE ( \"v1 Q\" INT ) ) RETURNS NUMBER AS 'm'",
            pointer: "/0/create_data_metric_function/arguments/0/columns/0/name/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "DEFINE keeps its verb",
            sql: "DEFINE SECURE DATA METRIC FUNCTION d1.s1.v1 ( v1 TABLE ( v1 INT ) ) RETURNS NUMBER AS 'm'",
            pointer: "/0/create_data_metric_function/verb",
            expected_value: json!("DEFINE"),
        },
        ProjectionTestCase {
            description: "a data metric function keeps NOT NULL",
            sql: "CREATE DATA METRIC FUNCTION v1 ( v1 TABLE ( v1 INT ) ) RETURNS NUMBER NOT NULL AS 'm'",
            pointer: "/0/create_data_metric_function/nullable",
            expected_value: json!(false),
        },
        ProjectionTestCase {
            description: "a typed literal keeps its type's name and parameters",
            sql: "SELECT PERIOD ( INT ) 'x'",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_value: json!("PERIOD ( INT )"),
        },
        ProjectionTestCase {
            description: "USE SCHEMA keeps the database part's quoting",
            sql: "USE SCHEMA \"v1 Q\" . v1",
            pointer: "/0/use/parts/0/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "USE SCHEMA keeps the schema part apart from the database",
            sql: "USE SCHEMA \"v1 Q\" . v1",
            pointer: "/0/use/parts/1/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "a schema-qualified window call keeps its schema",
            sql: "SELECT s1.f1 ( 1 ) OVER ( PARTITION BY 1 )",
            pointer: "/0/select/expressions/0/window_function/this/method_call/this/column/name/name",
            expected_value: json!("s1"),
        },
        ProjectionTestCase {
            description: "a path after parentheses keeps the parenthesized base",
            sql: "SELECT ( PARSE_JSON ( '{}' ) ) : a . b",
            pointer: "/0/select/expressions/0/j_s_o_n_extract/expression/literal/value",
            expected_value: json!("a.b"),
        },
        ProjectionTestCase {
            description: ":: PERIOD keeps its element type like CAST",
            sql: "SELECT NULL :: PERIOD ( DATE )",
            pointer: "/0/select/expressions/0/cast/to/name",
            expected_value: json!("PERIOD(DATE)"),
        },
        ProjectionTestCase {
            description: "USE SCHEMA with a quoted dotted name has one part",
            sql: "USE SCHEMA \"v1 Q.v1\"",
            pointer: "/0/use/parts",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "DEFAULT in INSERT VALUES is the keyword, not a variable",
            sql: "INSERT INTO v1 VALUES ( DEFAULT , \"DEFAULT\" ) ;",
            pointer: "/0/insert/values/0/0",
            expected_value: json!({"default_keyword": null}),
        },
        ProjectionTestCase {
            description: "a quoted \"DEFAULT\" in INSERT VALUES stays a column",
            sql: "INSERT INTO v1 VALUES ( DEFAULT , \"DEFAULT\" ) ;",
            pointer: "/0/insert/values/0/1/column/name/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "DEFAULT in a multi-table intoClause is the keyword, not a column",
            sql: "INSERT ALL INTO v1 VALUES ( DEFAULT , v1 ) SELECT 1 ;",
            pointer: "/0/multitable_inserts/expressions/0/conditional_insert/this/insert/values/0/0",
            expected_value: json!({"default_keyword": null}),
        },
        ProjectionTestCase {
            description: "a source column in a multi-table intoClause stays a column",
            sql: "INSERT ALL INTO v1 VALUES ( DEFAULT , v1 ) SELECT 1 ;",
            pointer: "/0/multitable_inserts/expressions/0/conditional_insert/this/insert/values/0/1/column/name/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "DEFAULT in a conditional multi-table intoClause is the keyword",
            sql: "INSERT FIRST WHEN TRUE THEN INTO v1 VALUES ( DEFAULT ) SELECT 1 ;",
            pointer: "/0/multitable_inserts/expressions/0/conditional_insert/this/insert/values/0/0",
            expected_value: json!({"default_keyword": null}),
        },
        ProjectionTestCase {
            description: "ALTER DYNAMIC TABLE MODIFY <col> COMMENT keeps the comment as the column's comment",
            sql: "ALTER DYNAMIC TABLE v1 MODIFY ( COLUMN v1 UNSET COMMENT , v2 COMMENT 'x' ) ;",
            pointer: "/0/alter_object/operands/1/alter_column/comment/literal/value",
            expected_value: json!("x"),
        },
        ProjectionTestCase {
            description: "ALTER DYNAMIC TABLE MODIFY <col> UNSET COMMENT drops the column's comment",
            sql: "ALTER DYNAMIC TABLE v1 MODIFY ( COLUMN v1 UNSET COMMENT , v2 COMMENT 'x' ) ;",
            pointer: "/0/alter_object/operands/0/alter_column/drop",
            expected_value: json!({"var": {"this": "COMMENT"}}),
        },
        ProjectionTestCase {
            description: "a named column FOREIGN KEY keeps its constraint name",
            sql: "ALTER TABLE v1 ADD COLUMN v1 INT CONSTRAINT c9 FOREIGN KEY REFERENCES v2 ( v1 ) RELY ;",
            pointer: "/0/alter_table/actions/0/AddColumn/column/constraints/0/References/constraint_name",
            expected_value: json!("c9"),
        },
        ProjectionTestCase {
            description: "RELY stays with the column constraint it follows",
            sql: "ALTER TABLE v1 ADD COLUMN v1 INT NOT NULL CONSTRAINT c9 UNIQUE NORELY ;",
            pointer: "/0/alter_table/actions/0/AddColumn/column/constraint_properties",
            expected_value: json!([[1, "NORELY"]]),
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
fn given_receipts_contradicted_by_pr_68_when_parsing_then_acceptance_matches_snowflake() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a function argument may omit its name (recorded errno 90201)",
            sql: "CREATE FUNCTION v1 ( INT ) RETURNS INT AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a function argument may omit its type (recorded errno 90201)",
            sql: "CREATE FUNCTION v1 ( v1 ) RETURNS INT LANGUAGE JAVASCRIPT AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a procedure argument may omit its type (recorded errno 90201)",
            sql: "CREATE PROCEDURE v1 ( v1 ) RETURNS INT LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a procedure's RETURNS TABLE column may omit its type (recorded errno 90214)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS TABLE ( v1 ) LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a procedure's RETURNS TABLE column may omit its name (recorded success)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS TABLE ( INT ) LANGUAGE SQL AS $$ BEGIN RETURN 1 ; END $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SHOW GRANTS OF reads a kind word as the name (recorded errno 2003)",
            sql: "SHOW GRANTS OF ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SHOW GRANTS ON takes a bare name (recorded errno 2003)",
            sql: "SHOW GRANTS ON v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SHOW GRANTS TO takes an empty kind (recorded errno 2003)",
            sql: "SHOW GRANTS TO APPLICATION ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SHOW GRANTS OF APPLICATION ROLE needs a name (recorded errno 1003)",
            sql: "SHOW GRANTS OF APPLICATION ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a task tag value may be a name (recorded errno 391805)",
            sql: "CREATE TASK v1 WITH TAG ( v1 = m1 ) AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a task property takes a dollar-quoted value and commas (recorded success)",
            sql: "CREATE TASK v1 CONFIG = $$x$$ TIMEZONE = 'UTC', AUTOCOMMIT = TRUE AS SELECT 1",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a task's AFTER comes before EXECUTE AS USER (recorded errno 1003)",
            sql: "CREATE TASK t2 WAREHOUSE = wh EXECUTE AS USER u1 AFTER t1 AS SELECT 1",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a task's EXECUTE AS USER follows AFTER (recorded success)",
            sql: "CREATE TASK t2 AFTER t1 EXECUTE AS USER u1 AS SELECT 1",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DEFINE SEQUENCE refuses a reserved name part (recorded errno 1003)",
            sql: "DEFINE SEQUENCE d1.s1.SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an experiment may omit its TYPE (recorded errno 400022)",
            sql: "CREATE EXPERIMENT v1 FROM SPECIFICATION $$$$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "stage options may be comma-separated (recorded success)",
            sql: "CREATE STAGE s1 COMMENT = 'x', FILE_FORMAT = ( TYPE = CSV )",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "file format options may be comma-separated (recorded success)",
            sql: "CREATE FILE FORMAT f1 TYPE = 'CSV' FIELD_DELIMITER = '|', SKIP_HEADER = 2",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "Snowflake has no backtick identifiers (recorded errno 1003)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = '3.10' ARTIFACT_REPOSITORY = ` v1 ` HANDLER = 'm1.run' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ALTER DYNAMIC TABLE may omit the name before SET (recorded errno 926)",
            sql: "ALTER DYNAMIC TABLE SET TARGET_LAG = '1 minute' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ALTER DYNAMIC TABLE refuses a missing name before UNSET (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE UNSET INITIALIZATION_WAREHOUSE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a dynamic table's UNSET TAG may be empty (recorded errno 1420)",
            sql: "ALTER DYNAMIC TABLE v1 UNSET TAG ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a dynamic table's SET TAG may omit the tag name (recorded errno 1420)",
            sql: "ALTER DYNAMIC TABLE v1 SET TAG = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a dynamic table column's UNSET TAG needs a tag (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 ALTER v1 UNSET TAG ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a row access policy may name no columns (recorded errno 2003)",
            sql: "ALTER DYNAMIC TABLE v1 ADD ROW ACCESS POLICY v1 ON ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a storage lifecycle policy needs columns (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 ADD STORAGE LIFECYCLE POLICY v1 ON ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "DROP SEARCH OPTIMIZATION takes a string target (recorded errno 904)",
            sql: "ALTER DYNAMIC TABLE v1 DROP SEARCH OPTIMIZATION ON 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "RESUME SEARCH OPTIMIZATION refuses a string target (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 RESUME SEARCH OPTIMIZATION ON 'v1' ;",
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
fn given_expression_gap_receipts_when_parsing_then_acceptance_matches_snowflake() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a schema-qualified call takes OVER (recorded errno 2060, import 159)",
            sql: "SELECT s1.f1 ( 1 ) OVER ( PARTITION BY 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a database-qualified call takes OVER (recorded errno 2060, import 159)",
            sql: "SELECT d1.s1.f1 ( 1 ) OVER ( ORDER BY 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "OVER needs its window (recorded errno 1003, import 159)",
            sql: "SELECT s1.f1 ( 1 ) OVER ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a path follows a parenthesized window call (recorded success, import 159)",
            sql: "SELECT ( FIRST_VALUE ( PARSE_JSON ( '{\"a\":{\"b\":1}}' ) ) OVER ( ORDER BY 1 ) ) : a . b ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a colon after parentheses needs a path (recorded errno 1003, import 159)",
            sql: "SELECT ( PARSE_JSON ( '{\"a\":1}' ) ) : ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: ":: takes PERIOD with its element type (recorded success, import 159)",
            sql: "SELECT '[2026-01-01, 2026-01-02)' :: PERIOD ( DATE ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: ":: PERIOD needs an element type (recorded errno 1003, import 159)",
            sql: "SELECT NULL :: PERIOD ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "RECURSIVE cannot follow another CTE (recorded errno 1003, import 159)",
            sql: "WITH v1 AS ( SELECT 1 AS n ) , RECURSIVE r ( n ) AS ( SELECT 1 UNION ALL SELECT n + 1 FROM r WHERE n < 3 ) SELECT n FROM r ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a later CTE may recurse without RECURSIVE (recorded success, import 159)",
            sql: "WITH v1 AS ( SELECT 1 AS n ) , r ( n ) AS ( SELECT 1 UNION ALL SELECT n + 1 FROM r WHERE n < 3 ) SELECT n FROM r ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a lambda takes at least one parameter (recorded errno 1003)",
            sql: "SELECT ACCUMULATE ( 1 , ( ) -> 0 , ( s , x ) -> s + x , ( s1 , s2 ) -> s1 + s2 , s -> s ) ;",
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
fn given_snowflake_timestamp_conversions_without_arguments_when_parsing_then_they_parse() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description:
                "TO_TIMESTAMP with no arguments parses; its arity is checked later (recorded OK)",
            sql: "SELECT TO_TIMESTAMP ( ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "TRY_TO_TIMESTAMP with no arguments parses (recorded OK)",
            sql: "SELECT TRY_TO_TIMESTAMP ( ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "a trailing comma (recorded errno 1003)",
            sql: "SELECT TO_TIMESTAMP ( 1 , ) ;",
            expected_kind: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            snowflake_kind(test_case.sql),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
