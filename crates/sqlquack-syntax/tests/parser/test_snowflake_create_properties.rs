use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_property_or_policy_create_when_parsing_then_only_what_snowflake_parses_is_accepted(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a user is created by name (recorded success)",
            sql: "CREATE USER v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a user name is no reserved word (recorded errno 1003)",
            sql: "CREATE USER SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "WITH needs a property (recorded errno 1003)",
            sql: "CREATE RESOURCE MONITOR v1 WITH ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a backup policy takes WITH RETENTION LOCK (recorded success)",
            sql: "CREATE OR ALTER BACKUP POLICY v1 WITH RETENTION LOCK EXPIRE_AFTER_DAYS = 1 COMMENT = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "RETENTION alone is refused (recorded errno 1003)",
            sql: "CREATE BACKUP POLICY v1 WITH RETENTION ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "OR ALTER creates a database role (recorded success)",
            sql: "CREATE OR ALTER DATABASE ROLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "OR ALTER DATABASE ROLE ; still names a database (recorded success)",
            sql: "CREATE OR ALTER DATABASE ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a partial integration kind is refused (recorded errno 1003)",
            sql: "CREATE API i1 API_PROVIDER = aws_api_gateway API_AWS_ROLE_ARN = 'm' API_ALLOWED_PREFIXES = ( 'https://example.com/' ) ENABLED = TRUE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a list takes no trailing comma (recorded errno 1003)",
            sql: "CREATE CATALOG INTEGRATION v1 CATALOG_SOURCE = ICEBERG_REST TABLE_FORMAT = ICEBERG REST_CONFIG = ( CATALOG_URI = 'm' ) REST_AUTHENTICATION = ( TYPE = OAUTH OAUTH_CLIENT_ID = 'm' OAUTH_CLIENT_SECRET = 'm' OAUTH_ALLOWED_SCOPES = ( 'm' , ) ) ENABLED = TRUE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "string keys pair inside a list (recorded success)",
            sql: "CREATE NOTIFICATION INTEGRATION v1 TYPE = WEBHOOK ENABLED = TRUE WEBHOOK_URL = 'm' WEBHOOK_HEADERS = ( 'm' = 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a packages policy names its language (recorded success)",
            sql: "CREATE PACKAGES POLICY \"v1 Q\" LANGUAGE PYTHON ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a masking policy argument may be a bare type (recorded success)",
            sql: "CREATE MASKING POLICY v1 AS ( VARCHAR ) RETURNS VARCHAR -> v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a policy body is required (recorded errno 1003)",
            sql: "CREATE MASKING POLICY v1 AS ( v1 VARCHAR ) RETURNS VARCHAR -> ;",
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
fn given_a_snowflake_property_or_policy_create_when_parsing_then_the_tree_keeps_its_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "a property-list kind keeps its kind",
            sql: "CREATE OR REPLACE SECURITY INTEGRATION i1 TYPE = SAML2 ENABLED = TRUE",
            pointer: "/0/create_object/kind",
            expected_value: json!("SECURITY INTEGRATION"),
        },
        ProjectionTestCase {
            description: "a property keeps its name",
            sql: "CREATE USER u1 DEFAULT_ROLE = r1",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("DEFAULT_ROLE"),
        },
        ProjectionTestCase {
            description: "a multi-word value stays one value",
            sql: "CREATE FAILOVER GROUP g1 OBJECT_TYPES = INTEGRATIONS ALLOWED_INTEGRATION_TYPES = SECURITY INTEGRATIONS ALLOWED_ACCOUNTS = o1.a1",
            pointer: "/0/create_object/properties/1/value/table/name/name",
            expected_value: json!("SECURITY INTEGRATIONS"),
        },
        ProjectionTestCase {
            description: "a body policy keeps its body",
            sql: "CREATE ROW ACCESS POLICY p1 AS ( c1 INT ) RETURNS BOOLEAN -> c1 > 0",
            pointer: "/0/create_object/operands/2/gt/right/literal/value",
            expected_value: json!("0"),
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
