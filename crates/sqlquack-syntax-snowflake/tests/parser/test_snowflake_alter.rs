use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, snowflake_kind, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase, SnowflakeVerdictTestCase};

#[test]
fn given_a_snowflake_alter_of_an_object_kind_when_parsing_then_recorded_rejections_are_refused() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a task resumes (recorded semantic error, so it parsed)",
            sql: "ALTER TASK IF EXISTS v1 RESUME ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a task sets a session parameter by any name (recorded semantic error)",
            sql: "ALTER TASK v1 SET LOCK_TIMEOUT = 1 , LOCK_TIMEOUT = 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a task unsets EXECUTE AS USER (recorded semantic error)",
            sql: "ALTER TASK v1 UNSET EXECUTE AS USER ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a task named GRAPH suspends (recorded semantic error)",
            sql: "ALTER TASK GRAPH SUSPEND ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a task graph sets its task defaults (recorded semantic error)",
            sql: "ALTER TASK GRAPH v1 SET TASK_DEFAULTS ( USER_TASK_TIMEOUT_MS = 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an experiment aborts (recorded semantic error)",
            sql: "ALTER EXPERIMENT \"v1 Q\" ABORT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description:
                "a property value is missing; the Raw fallback accepted this (recorded errno 1003)",
            sql: "ALTER TASK v1 SET WAREHOUSE = ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a reserved keyword is no name (recorded errno 1003)",
            sql: "ALTER TASK v1 REMOVE AFTER SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a string is no user (recorded errno 1003)",
            sql: "ALTER TASK v1 SET EXECUTE AS USER 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a task has no task defaults (recorded errno 1003)",
            sql: "ALTER TASK v1 SET TASK_DEFAULTS ( USER_TASK_TIMEOUT_MS = 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description:
                "a task graph reads CONTACT as contacts, not a parameter (recorded errno 1003)",
            sql: "ALTER TASK GRAPH v1 SET CONTACT = v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description:
                "Snowflake refuses the page's UNSET DATA MOVEMENT POLICY (recorded errno 1003)",
            sql: "ALTER TAG v1 UNSET DATA MOVEMENT POLICY ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a snapshot identifier is a string (recorded errno 1003)",
            sql: "ALTER SNAPSHOT SET v1 DELETE SNAPSHOT IDENTIFIER m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a snapshot only sets its comment (recorded errno 1003)",
            sql: "ALTER SNAPSHOT v1 UNSET COMMENT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a word that starts no object kind (recorded errno 1003)",
            sql: "ALTER v1 ADD RUN v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an alert's condition is a parenthesised query (recorded accepted)",
            sql: "ALTER ALERT IF EXISTS v1 MODIFY CONDITION EXISTS ( SELECT 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an alert from a template sets its properties (recorded semantic error)",
            sql: "ALTER ALERT IF EXISTS v1 FROM TEMPLATE v1 SET WAREHOUSE = 'm' SCHEDULE = '1 MINUTE' COMMENT = 'm' RUNBOOK = 'm' SUSPEND_ALERT_AFTER_NUM_FAILURES = 1 TEMPLATE_PARAMS = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an online feature table refreshes (recorded semantic error)",
            sql: "ALTER ONLINE FEATURE TABLE \"SELECT\" REFRESH ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a variable drops its default (recorded semantic error)",
            sql: "ALTER VARIABLE v1 DROP DEFAULT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an alert property value is missing (recorded errno 1003)",
            sql: "ALTER ALERT v1 FROM TEMPLATE v1 SET WAREHOUSE = ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ONLINE without FEATURE TABLE is no ALTER (recorded errno 1003)",
            sql: "ALTER ONLINE TABLE v1 REFRESH ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "FEATURE TABLE without ONLINE is no ALTER (recorded errno 1003)",
            sql: "ALTER FEATURE TABLE v1 REFRESH ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "UNSET TAG needs a tag name (recorded errno 1003)",
            sql: "ALTER ONLINE FEATURE TABLE v1 UNSET TAG ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a string is no variable name (recorded errno 1003)",
            sql: "ALTER VARIABLE 'v1' DROP DEFAULT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a data metric function is named by its TABLE( ... ) signature (recorded semantic error)",
            sql: "ALTER FUNCTION v1 ( TABLE ( INT ) , TABLE ( INT ) ) SET SECURE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a procedure runs as a restricted caller (recorded semantic error)",
            sql: "ALTER PROCEDURE v1 ( ) EXECUTE AS RESTRICTED CALLER ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a stage path ends at whitespace, so the quoted name splits (recorded errno 1003)",
            sql: "ALTER MODEL v1 ADD VERSION v1 FROM @\"s1 Q\" ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a stage path may name a reserved keyword (recorded semantic error)",
            sql: "ALTER MODEL v1 ADD VERSION v1 FROM @SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model named MONITOR unsets its comment (recorded semantic error)",
            sql: "ALTER MODEL MONITOR UNSET COMMENT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model named monitor is renamed",
            sql: "ALTER MODEL monitor RENAME TO m3 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model named monitor adds a version",
            sql: "ALTER MODEL monitor ADD VERSION v2 FROM MODEL m2 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model in a schema named monitor sets its comment",
            sql: "ALTER MODEL monitor.m1 SET COMMENT = 'x' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model monitor still adds segment columns",
            sql: "ALTER MODEL MONITOR mm ADD SEGMENT_COLUMN = 'c1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model in a schema named monitor is dropped",
            sql: "DROP MODEL monitor.m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a model monitor is still dropped by name",
            sql: "DROP MODEL MONITOR mm ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "TAG pairs are a SET of their own (every recorded mix is errno 1003)",
            sql: "ALTER COMPUTE POOL v1 SET MIN_NODES = 1 TAG v1 = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an agent sets one property at a time (recorded errno 1003)",
            sql: "ALTER AGENT v1 SET COMMENT = 'm' PROFILE = 'm' SECURE = TRUE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "COMPUTE without POOL is no kind (recorded errno 1003)",
            sql: "ALTER COMPUTE v1 SUSPEND ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "Snowflake has no ALTER EVENT TABLE (recorded errno 1003)",
            sql: "ALTER EVENT TABLE v1 UNSET RULE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "IF EXISTS comes before an external table's name (recorded errno 1003)",
            sql: "ALTER EXTERNAL TABLE v1 IF EXISTS ADD PARTITION ( v1 = 'm' ) LOCATION 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a partition column is no string (recorded errno 1003)",
            sql: "ALTER EXTERNAL TABLE v1 ADD PARTITION ( 'v1' = 'm' ) LOCATION 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "only external tables add partitions (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD PARTITION ( v1 = 'm' ) LOCATION 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an inline specification takes no USING (recorded errno 1003)",
            sql: "ALTER SERVICE v1 FROM $$m: 1$$ USING ( v1 => 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a restricted session scope adds its body without AS; ADD AS is refused (recorded errno 1003)",
            sql: "ALTER RESTRICTED SESSION SCOPE v1 ADD AS $$m: 1 $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a restricted session scope takes a dollar-quoted definition (recorded semantic error)",
            sql: "ALTER RSS v1 AS $$v1 $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a dataset version comes from a query, not an expression (recorded errno 1003)",
            sql: "ALTER DATASET v1 ADD VERSION 'm' FROM ( 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "VOLUME alone is no kind (recorded errno 1003)",
            sql: "ALTER VOLUME v1 REMOVE STORAGE_LOCATION 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a policy body follows another SET property (recorded semantic error)",
            sql: "ALTER INGRESS POLICY v1 SET COMMENT = 'm' AS $$m: 1 $$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a SET of a policy body alone is refused (recorded errno 1003)",
            sql: "ALTER FEATURE POLICY v1 SET AS $$$$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a masking policy sets its body after an arrow (recorded semantic error)",
            sql: "ALTER MASKING POLICY v1 SET BODY -> 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a network policy adds to a rule list (recorded semantic error)",
            sql: "ALTER NETWORK POLICY v1 ADD ALLOWED_NETWORK_RULE_LIST = ( 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a session policy is no ALTER SESSION (recorded semantic error)",
            sql: "ALTER SESSION POLICY v1 SET ALLOWED_SECONDARY_ROLES = ( 'ALL' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ALTER SESSION still sets a parameter",
            sql: "ALTER SESSION SET TIMEZONE = 'UTC' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a snapshot policy unsets a property (recorded semantic error)",
            sql: "ALTER SNAPSHOT POLICY v1 UNSET EXPIRE_AFTER_DAYS ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a snapshot policy takes no reserved keyword as its name (recorded errno 1003)",
            sql: "ALTER SNAPSHOT POLICY SELECT UNSET COMMENT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "MASKING without POLICY is no kind (recorded errno 1003)",
            sql: "ALTER MASKING v1 RENAME TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "DATA POLICY lacks MOVEMENT (recorded errno 1003)",
            sql: "ALTER DATA POLICY v1 RENAME TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a group sets comma lists, each ended by the next property (recorded semantic error)",
            sql: "ALTER FAILOVER GROUP IF EXISTS v1 SET OBJECT_TYPES = DATABASES , SHARES ALLOWED_DATABASES = v1 , v1 ALLOWED_SHARES = v1 , v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an integration type is a name of two words (recorded runtime error)",
            sql: "ALTER FAILOVER GROUP v1 SET OBJECT_TYPES = INTEGRATIONS ALLOWED_INTEGRATION_TYPES = SECURITY INTEGRATIONS ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a group property value has no name of its own (recorded errno 1003)",
            sql: "ALTER FAILOVER GROUP v1 SET OBJECT_TYPES = INTEGRATIONS = SECURITY INTEGRATIONS ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "allowed accounts are organization-qualified (recorded semantic error)",
            sql: "ALTER FAILOVER GROUP IF EXISTS v1 ADD v1 . v1 , v1 . v1 TO ALLOWED_ACCOUNTS IGNORE EDITION CHECK ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a reserved keyword is no account name (recorded errno 1003)",
            sql: "ALTER CONNECTION v1 ENABLE FAILOVER TO ACCOUNTS v1 . SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a replication group moves objects only to a REPLICATION GROUP (recorded errno 1003)",
            sql: "ALTER REPLICATION GROUP v1 MOVE DATABASES v1 TO GROUP v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "GROUP alone is no kind (recorded errno 1003)",
            sql: "ALTER GROUP v1 RENAME TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "the organization sets its event routing table (recorded semantic error)",
            sql: "ALTER ORGANIZATION SET EVENT ROUTING TABLE v1 FOR ALL APPLICATION LISTINGS ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "region groups are strings (recorded errno 1003)",
            sql: "ALTER ORGANIZATION USER GROUP v1 SET VISIBILITY = REGION GROUPS m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an organization profile adds a live version (recorded semantic error)",
            sql: "ALTER ORGANIZATION PROFILE v1 ADD LIVE VERSION IF NOT EXISTS v1 FROM LAST ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ALL is an integration value (recorded semantic error)",
            sql: "ALTER INTEGRATION v1 SET ALLOWED_AUTHENTICATION_SECRETS = all ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a reserved keyword is no secret name (recorded errno 1003)",
            sql: "ALTER INTEGRATION v1 SET ALLOWED_AUTHENTICATION_SECRETS = ( SELECT ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a webhook header needs a value (recorded errno 1003)",
            sql: "ALTER INTEGRATION v1 SET WEBHOOK_HEADERS = ( 'm' = ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "EXTERNAL ACCESS without INTEGRATION is no kind (recorded errno 1003)",
            sql: "ALTER EXTERNAL ACCESS v1 UNSET ALLOWED_NETWORK_RULES ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an account redirect list keys on DEFAULT (recorded semantic error)",
            sql: "ALTER ACCOUNT SET LOGIN_IDP_REDIRECT = ( DEFAULT = v1 , DEFAULT = v1 ) OBJECT_VISIBILITY = $$m: 1$$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an account policy needs POLICY (recorded errno 1003)",
            sql: "ALTER ACCOUNT SET FEATURE v1 FOR ALL APPLICATIONS ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "other dialects keep their ALTER fallback",
            sql: "ALTER v1 ADD RUN v1",
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
fn given_a_snowflake_alter_of_an_object_kind_when_parsing_then_the_tree_names_kind_and_action() {
    let test_cases = [
        ProjectionTestCase {
            description: "TASK GRAPH is one kind",
            sql: "ALTER TASK GRAPH v1 COMMIT",
            pointer: "/0/alter_object/kind",
            expected_value: json!("TASK GRAPH"),
        },
        ProjectionTestCase {
            description: "a task named GRAPH is a task",
            sql: "ALTER TASK GRAPH RESUME",
            pointer: "/0/alter_object/kind",
            expected_value: json!("TASK"),
        },
        ProjectionTestCase {
            description: "a multi-word action is named whole",
            sql: "ALTER SNAPSHOT SET v1 SUSPEND SNAPSHOT CREATION POLICY",
            pointer: "/0/alter_object/action",
            expected_value: json!("SUSPEND SNAPSHOT CREATION POLICY"),
        },
        ProjectionTestCase {
            description: "a contact keeps its purpose",
            sql: "ALTER TASK v1 SET CONTACT STEWARD = v1",
            pointer: "/0/alter_object/properties/0/name/name",
            expected_value: json!("STEWARD"),
        },
        ProjectionTestCase {
            description: "a session parameter is named by itself",
            sql: "ALTER TASK v1 UNSET LOCK_TIMEOUT",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("LOCK_TIMEOUT"),
        },
        ProjectionTestCase {
            description: "ONLINE FEATURE TABLE is one kind",
            sql: "ALTER ONLINE FEATURE TABLE v1 SUSPEND",
            pointer: "/0/alter_object/kind",
            expected_value: json!("ONLINE FEATURE TABLE"),
        },
        ProjectionTestCase {
            description: "an alert from a template names its action whole",
            sql: "ALTER ALERT v1 FROM TEMPLATE t1 SET COMMENT = 'm'",
            pointer: "/0/alter_object/action",
            expected_value: json!("FROM TEMPLATE SET"),
        },
        ProjectionTestCase {
            description: "a variable's new default is its operand",
            sql: "ALTER VARIABLE v1 SET DEFAULT 42",
            pointer: "/0/alter_object/operands/0/literal/value",
            expected_value: json!("42"),
        },
        ProjectionTestCase {
            description: "a function's argument types are kept, a TABLE( ... ) argument as a tuple",
            sql: "ALTER FUNCTION v1 ( TABLE ( INT , TEXT ) ) UNSET TAG t1",
            pointer: "/0/alter_object/arguments/0/tuple/expressions/1/data_type/data_type",
            expected_value: json!("text"),
        },
        ProjectionTestCase {
            description: "COMPUTE POOL is one kind",
            sql: "ALTER COMPUTE POOL v1 STOP ALL",
            pointer: "/0/alter_object/kind",
            expected_value: json!("COMPUTE POOL"),
        },
        ProjectionTestCase {
            description: "IF EXISTS is kept",
            sql: "ALTER TAG IF EXISTS v1 RENAME TO v2",
            pointer: "/0/alter_object/if_exists",
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
fn given_snowflake_session_settings_when_parsing_then_each_needs_a_value() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "settings separated by spaces (recorded OK)",
            sql: "ALTER SESSION SET QUERY_TAG = 'q1' TIMEZONE = 'UTC' WEEK_START = 1 ;",
            expected_kind: Some("alter_session"),
        },
        SnowflakeVerdictTestCase {
            description: "a setting with no value (recorded errno 1003)",
            sql: "ALTER SESSION SET JSON_INDENT = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET with no setting (recorded errno 1003)",
            sql: "ALTER SESSION SET ;",
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
