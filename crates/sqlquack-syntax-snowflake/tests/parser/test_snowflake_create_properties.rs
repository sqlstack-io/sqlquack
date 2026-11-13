use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, snowflake_kind, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase, SnowflakeVerdictTestCase};

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
            description: "only a compute pool names an application",
            sql: "CREATE USER u1 FOR APPLICATION a1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a compute pool names its application right after its name",
            sql: "CREATE COMPUTE POOL p1 MIN_NODES = 1 MAX_NODES = 1 INSTANCE_FAMILY = v1 FOR APPLICATION a1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a model may be called monitor",
            sql: "CREATE MODEL monitor FROM MODEL m2 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model called monitor takes a version",
            sql: "CREATE OR REPLACE MODEL monitor WITH VERSION v1 FROM MODEL m2 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model may sit in a schema called monitor",
            sql: "CREATE OR REPLACE MODEL monitor.m1 WITH VERSION v1 FROM MODEL monitor.m2 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model monitor still needs its name",
            sql: "CREATE MODEL MONITOR WITH MODEL = v1 ;",
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
            description: "a DCM warehouse definition takes its properties (recorded success)",
            sql: "DEFINE WAREHOUSE v1 GENERATION = '1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a property value is a value, not a list of pairs (recorded errno 1003)",
            sql: "CREATE ARTIFACT REPOSITORY v1 TYPE = APPLICATION ( v1 = 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a compute pool's tag list ends the statement (recorded errno 1003)",
            sql: "CREATE COMPUTE POOL IF NOT EXISTS v1 FOR APPLICATION v1 MIN_NODES = 1 MAX_NODES = 1 INSTANCE_FAMILY = v1 AUTO_RESUME = TRUE INITIALLY_SUSPENDED = TRUE AUTO_SUSPEND_SECS = 1 WITH TAG ( v1 = 'm' , v1 = 'm' ) COMMENT = 'm' PLACEMENT_GROUP = 'm' BACKUP_INSTANCE_FAMILIES = ( 'm' , 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an alert takes properties after its tag list (recorded errno 1004, after parsing)",
            sql: "CREATE OR REPLACE ALERT IF NOT EXISTS v1 FROM TEMPLATE v1 WITH TAG ( v1 = 'm' , v1 = 'm' ) WAREHOUSE = v1 SCHEDULE = '1 MINUTE' COMMENT = 'm' RUNBOOK = 'm' SUSPEND_ALERT_AFTER_NUM_FAILURES = 1 TEMPLATE_PARAMS = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "MODEL alone is a kind, not part of MODEL MONITOR (recorded errno 1021, after parsing)",
            sql: "CREATE MODEL v1 WITH VERSION v1 FROM @s1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "VOLUME alone is a kind, not part of EXTERNAL VOLUME (recorded errno 1422, after parsing)",
            sql: "CREATE VOLUME v1 STORAGE_LOCATIONS = ( ( NAME = 'm' STORAGE_PROVIDER = 'S3' STORAGE_AWS_ROLE_ARN = 'm' STORAGE_BASE_URL = 's3://b1/p1/' ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
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
            description: "a parenthesized list holds its values side by side, not a list inside a list",
            sql: "CREATE CONTACT v1 USERS = ( 'u1' , 'u2' )",
            pointer: "/0/create_object/properties/0/value/tuple/expressions/1/literal/value",
            expected_value: json!("u2"),
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
            description: "a warehouse keeps OR ALTER",
            sql: "CREATE OR ALTER WAREHOUSE v1 WAREHOUSE_SIZE = MEDIUM",
            pointer: "/0/create_object/or_alter",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a compute pool's application is a property",
            sql: "CREATE COMPUTE POOL v1 FOR APPLICATION \"v1 Q\" MIN_NODES = 1 MAX_NODES = 1 INSTANCE_FAMILY = v1",
            pointer: "/0/create_object/properties/0/value/identifier/name",
            expected_value: json!("v1 Q"),
        },
        ProjectionTestCase {
            description: "an external volume's locations are pairs inside a list",
            sql: "CREATE EXTERNAL VOLUME v1 STORAGE_LOCATIONS = ( ( NAME = 'm' STORAGE_PROVIDER = 'S3' ) )",
            pointer: "/0/create_object/properties/0/value/tuple/expressions/0/tuple/expressions/1/property_e_q/left/identifier/name",
            expected_value: json!("STORAGE_PROVIDER"),
        },
        ProjectionTestCase {
            description: "RSS is the restricted session scope kind",
            sql: "CREATE RSS v1 AS $$v1 $$",
            pointer: "/0/create_object/kind",
            expected_value: json!("RSS"),
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

#[test]
fn given_snowflake_class_instance_commands_when_parsing_then_their_grammar_is_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a forecast with named arguments, tags and a comment (recorded OK)",
            sql: "CREATE OR REPLACE SNOWFLAKE . ML . FORECAST IF NOT EXISTS v1 ( INPUT_DATA => SYSTEM$REFERENCE ( 'VIEW' , 'v1' ) , TIMESTAMP_COLNAME => 'm' , TARGET_COLNAME => 'm' ) WITH TAG ( v1 = 'm' , v1 = 'm' ) COMMENT = 'm' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a forecast with positional arguments (recorded OK)",
            sql: "CREATE SNOWFLAKE . ML . FORECAST v1 ( 'm' , 'm' , 'm' , 'm' ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a budget in the default schema with no arguments and a bare comment (recorded OK)",
            sql: "CREATE SNOWFLAKE . . BUDGET v1 ( ) WITH COMMENT = m1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a class that is not a class (recorded errno 1003)",
            sql: "CREATE SNOWFLAKE . ML . v1 ( 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a class with no SNOWFLAKE database (recorded errno 1003)",
            sql: "CREATE . ML . FORECAST v1 ( 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a named argument with no value (recorded errno 1003)",
            sql: "CREATE SNOWFLAKE . ML . CLASSIFICATION v1 ( INPUT_DATA => , TARGET_COLNAME => 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty positional argument (recorded errno 1003)",
            sql: "CREATE SNOWFLAKE . ML . FORECAST v1 ( 'm' , 'm' , 'm' , ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "IF EXISTS on CREATE (recorded errno 1003)",
            sql: "CREATE SNOWFLAKE . CORE . BUDGET IF EXISTS v1 ( ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a tag list without TAG (recorded errno 1003)",
            sql: "CREATE SNOWFLAKE . CORE . BUDGET v1 ( ) ( v1 = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a property set without TAG (recorded OK)",
            sql: "ALTER SNOWFLAKE . CORE . BUDGET IF EXISTS v1 SET v1 = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "UNSET TAG with no tag names unsets a property named TAG (recorded OK)",
            sql: "ALTER SNOWFLAKE . ML . CLASSIFICATION v1 UNSET TAG ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a rename to a quoted reserved word (recorded OK)",
            sql: "ALTER SNOWFLAKE . CORE . BUDGET v1 RENAME TO \"SELECT\" ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an ALTER missing the instance's name (recorded errno 1003)",
            sql: "ALTER SNOWFLAKE . CORE . BUDGET SET COMMENT = 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET with no property (recorded errno 1003)",
            sql: "ALTER SNOWFLAKE . CORE . BUDGET v1 SET = 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a drop in the default schema (recorded OK)",
            sql: "DROP SNOWFLAKE . . BUDGET IF EXISTS v1 ;",
            expected_kind: Some("drop_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a drop missing the instance's name (recorded errno 1003)",
            sql: "DROP SNOWFLAKE . ML . FORECAST ;",
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

#[test]
fn given_a_snowflake_create_or_alter_kept_as_text_when_parsing_then_its_head_and_option_lists_are_checked(
) {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a warehouse named IF before its action (recorded OK)",
            sql: "ALTER WAREHOUSE IF SUSPEND ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "RESUME IF SUSPENDED after IF EXISTS (recorded OK)",
            sql: "ALTER WAREHOUSE IF EXISTS v1 RESUME IF SUSPENDED ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "the current warehouse with no name (recorded OK)",
            sql: "ALTER WAREHOUSE ABORT ALL QUERIES ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "START, a reserved keyword, as the action (recorded OK)",
            sql: "ALTER OPENFLOW CONNECTOR v1 START ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a reserved keyword written against a stage path (recorded OK)",
            sql: "CREATE MODEL v1 FROM @%SELECT ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a property before the body (recorded OK)",
            sql: "CREATE PIPE v1 INTEGRATION = m1 AS COPY INTO t1 FROM @s1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an unreserved operand word as the whole name",
            sql: "CREATE TAG version ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an unreserved operand word as the last part of a name",
            sql: "CREATE DATASET s1.policy ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an unreserved operand word as a value",
            sql: "ALTER WAREHOUSE v1 SET RESOURCE_MONITOR = token ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an unreserved operand word as a new name",
            sql: "ALTER ROLE v1 RENAME TO template ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an alert condition comparing with a quantified subquery",
            sql: "CREATE ALERT v1 WAREHOUSE = v1 SCHEDULE = '1 minute' IF ( EXISTS ( SELECT 1 FROM t1 WHERE c1 = ANY ( SELECT c1 FROM t2 ) ) ) THEN CALL p1 ( ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a property with no value (recorded errno 1003)",
            sql: "CREATE WAREHOUSE v1 COMMENT = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a property with no value on CREATE TAG (recorded errno 1003)",
            sql: "CREATE TAG v1 COMMENT = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unclosed tag list (recorded errno 1003)",
            sql: "CREATE COMPUTE POOL v1 MIN_NODES = 1 MAX_NODES = 1 INSTANCE_FAMILY = v1 TAG ( v1 = 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string as a tag name (recorded errno 1003)",
            sql: "CREATE ALERT v1 FROM TEMPLATE v1 TAG ( 'v1' = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "IF without EXISTS before a name (recorded errno 1003)",
            sql: "ALTER OPENFLOW RUNTIME IF v1 RESUME ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "IF EXISTS on CREATE (recorded errno 1003)",
            sql: "CREATE COMPUTE POOL IF EXISTS v1 MIN_NODES = 1 MAX_NODES = 1 INSTANCE_FAMILY = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an action word taken as the name, with no action after it (recorded errno 1003)",
            sql: "ALTER POSTGRES INSTANCE SUSPEND ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a word that is not an action after the name (recorded errno 1003)",
            sql: "ALTER WAREHOUSE v1 TAG v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "RENAME without TO (recorded errno 1003)",
            sql: "ALTER WAREHOUSE v1 RENAME v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "AS with no body (recorded errno 1003)",
            sql: "CREATE OR ALTER PIPE v1 AS ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a body where a value belongs (recorded errno 1003)",
            sql: "CREATE ALERT v1 COMMENT = IF ( EXISTS ( SELECT 1 ) ) THEN SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved keyword as a value (recorded errno 1003)",
            sql: "CREATE OR ALTER WAREHOUSE v1 RESOURCE_MONITOR = SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved keyword as a list item (recorded errno 1003)",
            sql: "ALTER WAREHOUSE v1 ADD TABLES ( SELECT ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved keyword as the last name (recorded errno 1003)",
            sql: "CREATE SNAPSHOT SET v1 FOR SCHEMA SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved keyword as a property name (recorded errno 1003)",
            sql: "ALTER OPENFLOW DATA PLANE INTEGRATION v1 SET = 'd1.s1.t1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a value assigned a value (recorded errno 1003)",
            sql: "CREATE EXTERNAL VOLUME v1 STORAGE_LOCATIONS = ( ( NAME = 'm' = 'GCS' STORAGE_BASE_URL = 'gcs://b1/p1/' ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a kind missing its second word (recorded errno 1003)",
            sql: "ALTER OPENFLOW v1 SUSPEND ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a kind word with no operand at the end (recorded errno 1003)",
            sql: "CREATE BACKUP SET v1 FOR TABLE ;",
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
