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
            description: "a storage integration's location list is one property value",
            sql: "CREATE STORAGE INTEGRATION s1 TYPE = EXTERNAL_STAGE STORAGE_PROVIDER = 'S3' ENABLED = TRUE STORAGE_ALLOWED_LOCATIONS = ( 's3://b1/p1/' , 's3://b2/p2/' )",
            pointer: "/0/create_object/properties/3/kind",
            expected_value: json!("STORAGE_ALLOWED_LOCATIONS"),
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

#[test]
fn given_a_snowflake_create_without_a_complete_kind_when_parsing_then_it_is_refused() {
    let test_cases = [
        AcceptanceTestCase { description: "an account needs its kind (recorded syntax error)", sql: "CREATE v1 ADMIN_NAME = 'm' ADMIN_PASSWORD = 'm' EMAIL = 'm' EDITION = STANDARD ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an application needs its source (recorded syntax error)", sql: "CREATE APPLICATION v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "CREATE OR ALTER of an application needs its source (recorded syntax error)", sql: "CREATE OR ALTER APPLICATION v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a lone name is no statement (recorded syntax error)", sql: "CREATE v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a profile is an organization profile (recorded syntax error)", sql: "CREATE PROFILE v1 AS 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a profile from a stage is an organization profile (recorded syntax error)", sql: "CREATE PROFILE v1 FROM @d1.s1.st1/p1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a bare profile is an organization profile (recorded syntax error)", sql: "CREATE PROFILE v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "PACKAGES needs POLICY (recorded syntax error)", sql: "CREATE PACKAGES v1 LANGUAGE PYTHON ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "POLICY needs its kind (recorded syntax error)", sql: "CREATE POLICY v1 LANGUAGE PYTHON ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "CREATE OR ALTER of a lone name (recorded syntax error)", sql: "CREATE OR ALTER v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "CREATE OR ALTER of a secret needs its kind (recorded syntax error)", sql: "CREATE OR ALTER v1 TYPE = CLOUD_PROVIDER_TOKEN API_AUTHENTICATION = 'm' ENABLED = TRUE ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a cloud provider token secret needs its kind (recorded syntax error)", sql: "CREATE v1 TYPE = CLOUD_PROVIDER_TOKEN API_AUTHENTICATION = 'm' ENABLED = TRUE ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a generic string secret needs its kind (recorded syntax error)", sql: "CREATE v1 TYPE = GENERIC_STRING SECRET_STRING = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an OAuth secret needs its kind (recorded syntax error)", sql: "CREATE v1 TYPE = OAUTH2 API_AUTHENTICATION = v1 OAUTH_SCOPES = ( 'm' ) ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an OAuth refresh token secret needs its kind (recorded syntax error)", sql: "CREATE v1 TYPE = OAUTH2 OAUTH_REFRESH_TOKEN = 'm' OAUTH_REFRESH_TOKEN_EXPIRY_TIME = 'm' API_AUTHENTICATION = v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a password secret needs its kind (recorded syntax error)", sql: "CREATE v1 TYPE = PASSWORD USERNAME = 'm' PASSWORD = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a symmetric key secret needs its kind (recorded syntax error)", sql: "CREATE v1 TYPE = SYMMETRIC_KEY ALGORITHM = GENERIC ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a workload identity secret needs its kind (recorded syntax error)", sql: "CREATE v1 TYPE = WORKLOAD_IDENTITY_FEDERATION ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an application from a package is kept", sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE p1 ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an organization profile is kept", sql: "CREATE ORGANIZATION PROFILE v1 ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a packages policy is kept", sql: "CREATE PACKAGES POLICY v1 LANGUAGE PYTHON ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a secret with its kind is kept", sql: "CREATE SECRET v1 TYPE = PASSWORD USERNAME = 'm' PASSWORD = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: true },
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
