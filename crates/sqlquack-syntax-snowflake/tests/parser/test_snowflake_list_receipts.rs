use sqlquack_syntax::Dialect;

use crate::helpers::accepted;
use crate::test_types::AcceptanceTestCase;

#[test]
fn given_a_trailing_comma_in_a_snowflake_list_when_parsing_then_acceptance_matches_snowflake() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a property value list ends in a comma (recorded errno 1003)",
            sql: "ALTER NETWORK RULE v1 SET VALUE_LIST = ( 'm' , ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description:
                "a property value list with an item after each comma (recorded errno 2003)",
            sql: "ALTER NETWORK RULE v1 SET VALUE_LIST = ( 'm' , 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a file format's NULL_IF list ends in a comma (recorded errno 1003)",
            sql: "ALTER STAGE v1 SET FILE_FORMAT = ( TYPE = CSV NULL_IF = ( 'm' , ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an integration's list ends in a comma (recorded errno 1003)",
            sql: "ALTER INTEGRATION v1 SET ALLOWED_AUTHENTICATION_SECRETS = ( v1 , ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an integration's list with an item after its comma (recorded errno 2003)",
            sql: "ALTER INTEGRATION v1 SET ALLOWED_AUTHENTICATION_SECRETS = ( v1 , v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an organization's list ends in a comma (recorded errno 1003)",
            sql: "ALTER ORGANIZATION SET DOMAIN_NAMES = ( 'm' , ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an organization's list with a name after its comma (recorded errno 2003)",
            sql: "ALTER ORGANIZATION SET DOMAIN_NAMES = ( 'm' , m1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a tuple ends in a comma (recorded errno 1003)",
            sql: "SELECT ( 1 ) IN ( ( 1 ) , ( 1 , ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "tuples without a trailing comma (recorded OK)",
            sql: "SELECT ( 1 ) IN ( ( 1 ) , ( 1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a SET value tuple ends in a comma (recorded errno 1003)",
            sql: "SET ( v1 ) = ( 1 , ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a LIKE ANY pattern list ends in a comma (recorded errno 1003)",
            sql: "SELECT v1 LIKE ANY ( 'm' , ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an UNSET list ends in a comma (recorded errno 1003)",
            sql: "ALTER TASK GRAPH v1 UNSET SUSPEND_TASK_AFTER_NUM_FAILURES , ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an UNSET list with a property after its comma (recorded errno 2003)",
            sql: "ALTER TASK v1 UNSET LOCK_TIMEOUT , STATEMENT_TIMEOUT_IN_SECONDS ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a secondary role list ends in a comma (recorded errno 1003)",
            sql: "USE SECONDARY ROLES v1 , ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a secondary role list with a role after its comma (recorded errno 2003)",
            sql: "USE SECONDARY ROLES v1 , v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ORDER BY ends in a comma (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 ORDER BY v1 , ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ORDER BY with a key after its comma (recorded errno 2003)",
            sql: "SELECT 1 FROM t1 ORDER BY v1 , v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "DESCRIBE's argument types end in a comma (recorded errno 1003)",
            sql: "DESC PROCEDURE v1 ( INT , ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "DESCRIBE with a type after its comma (recorded errno 2003)",
            sql: "DESC PROCEDURE v1 ( INT , INT ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a procedure's SET ends in a comma (recorded errno 1003)",
            sql: "ALTER PROCEDURE v1 ( ) SET EXTERNAL_ACCESS_INTEGRATIONS = 'm' , ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a procedure's SET without a trailing comma (recorded errno 2003)",
            sql: "ALTER PROCEDURE v1 ( ) SET EXTERNAL_ACCESS_INTEGRATIONS = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an external table's NULL_IF list ends in a comma (recorded errno 1003)",
            sql: "CREATE EXTERNAL TABLE v1 ( v1 INT AS 1 ) LOCATION = @s1 FILE_FORMAT = ( TYPE = CSV NULL_IF = ( 'm' , ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an external table's NULL_IF list with an item after its comma (recorded errno 2003)",
            sql: "CREATE EXTERNAL TABLE v1 ( v1 INT AS 1 ) LOCATION = @s1 FILE_FORMAT = ( TYPE = CSV NULL_IF = ( 'm' , 'm' ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a view's row access policy columns end in a comma (recorded errno 1003)",
            sql: "CREATE VIEW v1 ROW ACCESS POLICY v1 ON ( v1 , ) AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a view's row access policy with a column after its comma (recorded errno 2003)",
            sql: "CREATE VIEW v1 ROW ACCESS POLICY v1 ON ( v1 , v1 ) AS SELECT 1 ;",
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
fn given_an_empty_statement_in_a_snowflake_block_when_parsing_then_acceptance_matches_snowflake() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a block member followed by an extra semicolon (recorded OK)",
            sql: "BEGIN NULL ; ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a loop body member followed by an extra semicolon (recorded OK)",
            sql: "BEGIN WHILE ( FALSE ) DO NULL ; ; END WHILE ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an exception handler member followed by an extra semicolon (recorded OK)",
            sql: "BEGIN NULL ; EXCEPTION WHEN STATEMENT_ERROR THEN NULL ; ; NULL ; END ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a block that opens with a semicolon (recorded errno 1003)",
            sql: "BEGIN ; NULL ; END ;",
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
fn given_a_string_after_a_comma_in_a_group_name_list_when_parsing_then_acceptance_matches_snowflake(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a string after a comma in a list of shares (recorded errno 1003)",
            sql: "ALTER FAILOVER GROUP v1 SET ALLOWED_SHARES = v1 , 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a string after a comma in the object types (recorded errno 1003)",
            sql: "ALTER REPLICATION GROUP v1 SET OBJECT_TYPES = DATABASES , 'TABLE' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a single string for a list of shares (recorded errno 2003)",
            sql: "ALTER FAILOVER GROUP v1 SET ALLOWED_SHARES = 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "names after each comma in a list of shares (recorded errno 2003)",
            sql: "ALTER FAILOVER GROUP v1 SET ALLOWED_SHARES = v1 , v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a string after a comma in a created group's shares (recorded errno 1003)",
            sql: "CREATE FAILOVER GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_SHARES = v1 , 'v1' ALLOWED_ACCOUNTS = v1 . v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a comma before the next property of a created group (recorded errno 1003)",
            sql: "CREATE REPLICATION GROUP v1 OBJECT_TYPES = DATABASES , ALLOWED_ACCOUNTS = v1 . v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "names after each comma in a created group's lists (recorded errno 2003)",
            sql: "CREATE REPLICATION GROUP v1 OBJECT_TYPES = DATABASES , SHARES ALLOWED_ACCOUNTS = v1 . v1 ;",
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
fn given_a_word_key_after_a_comma_in_a_header_or_secret_map_when_parsing_then_acceptance_matches_snowflake(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a word key after a comma in ALTER FUNCTION's HEADERS (recorded errno 1003)",
            sql: "ALTER FUNCTION v1 ( ) SET HEADERS = ( 'm' = 'm' , m1 = 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a word key after a comma in ALTER FUNCTION's SECRETS (recorded errno 1003)",
            sql: "ALTER FUNCTION v1 ( ) SET SECRETS = ( 'm' = v1 , m1 = v1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a word key after a comma in an external function's HEADERS (recorded errno 1003)",
            sql: "CREATE EXTERNAL FUNCTION v1 ( ) RETURNS INT API_INTEGRATION = v1 HEADERS = ( 'm' = 'm' , m1 = 'm' ) AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a word key after a comma in WEBHOOK_HEADERS (recorded errno 1003)",
            sql: "CREATE NOTIFICATION INTEGRATION v1 TYPE = WEBHOOK ENABLED = TRUE WEBHOOK_URL = 'm' WEBHOOK_HEADERS = ( 'm' = 'm' , m1 = 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a word key after a comma in ALTER INTEGRATION's WEBHOOK_HEADERS (recorded errno 1003)",
            sql: "ALTER INTEGRATION v1 SET WEBHOOK_HEADERS = ( 'm' = 'm' , m1 = 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a lone word key in HEADERS (recorded errno 2003)",
            sql: "ALTER FUNCTION v1 ( ) SET HEADERS = ( m1 = 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a word value after a comma in HEADERS (recorded errno 2003)",
            sql: "ALTER FUNCTION v1 ( ) SET HEADERS = ( 'm' = 'm' , 'm' = m1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a lone word key in WEBHOOK_HEADERS (recorded errno 2003)",
            sql: "CREATE NOTIFICATION INTEGRATION v1 TYPE = WEBHOOK ENABLED = TRUE WEBHOOK_URL = 'm' WEBHOOK_HEADERS = ( m1 = 'm' ) ;",
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
fn given_a_string_tag_name_in_alter_warehouse_unset_when_parsing_then_acceptance_matches_snowflake()
{
    let test_cases = [
        AcceptanceTestCase {
            description: "a string tag name after a comma (recorded errno 1003)",
            sql: "ALTER WAREHOUSE v1 UNSET TAG v1 , 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "tag names after each comma (recorded errno 2003)",
            sql: "ALTER WAREHOUSE v1 UNSET TAG v1 , v1 ;",
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
