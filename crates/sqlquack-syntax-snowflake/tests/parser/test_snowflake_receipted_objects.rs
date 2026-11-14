use serde_json::json;

use crate::helpers::{snowflake_kind, statement_at};
use crate::test_types::{SnowflakeVerdictTestCase, StatementTestCase};

#[test]
fn given_a_snowflake_external_consumer_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "ADD takes PAT for PROGRAMMATIC ACCESS TOKEN (recorded semantic error)",
            sql: "ALTER EXTERNAL CONSUMER v1 ADD PAT v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "ADD keeps its properties (recorded semantic error)",
            sql: "ALTER EXTERNAL CONSUMER v1 ADD PROGRAMMATIC ACCESS TOKEN v1 COMMENT = m1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "MODIFY sets properties (recorded semantic error)",
            sql: "ALTER EXTERNAL CONSUMER v1 MODIFY PROGRAMMATIC ACCESS TOKEN v1 SET DAYS_TO_EXPIRY = 1 COMMENT = m1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "MODIFY needs SET (recorded errno 1003)",
            sql: "ALTER EXTERNAL CONSUMER v1 MODIFY PAT v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a token action takes no IF EXISTS (recorded errno 1003)",
            sql: "ALTER EXTERNAL CONSUMER v1 REMOVE PROGRAMMATIC ACCESS TOKEN IF EXISTS v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a token is named, not a string (recorded errno 1003)",
            sql: "ALTER EXTERNAL CONSUMER v1 ROTATE PROGRAMMATIC ACCESS TOKEN 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "the token words are all needed (recorded errno 1003)",
            sql: "ALTER EXTERNAL CONSUMER v1 ADD PROGRAMMATIC ACCESS v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a consumer is not renamed (recorded errno 1003)",
            sql: "ALTER EXTERNAL CONSUMER v1 RENAME TO v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "UNSET takes a list (recorded success)",
            sql: "ALTER EXTERNAL CONSUMER IF EXISTS v1 UNSET COMMENT , EMAIL ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "CREATE takes IF NOT EXISTS (recorded semantic error)",
            sql: "CREATE OR REPLACE EXTERNAL CONSUMER IF NOT EXISTS v1 REGION = 'PUBLIC.AWS_US_WEST_2' COMMENT = 'm' EMAIL = 'm' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "CREATE takes no IF EXISTS (recorded errno 1003)",
            sql: "CREATE EXTERNAL CONSUMER IF EXISTS v1 ;",
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
fn given_a_snowflake_external_consumer_when_parsing_then_the_tree_keeps_its_kind() {
    let test_cases = [
        StatementTestCase {
            description: "CREATE keeps EXTERNAL in its kind",
            sql: "CREATE EXTERNAL CONSUMER v1 COMMENT = m1 ;",
            pointer: "/0/create_object/kind",
            expected_value: json!("EXTERNAL CONSUMER"),
        },
        StatementTestCase {
            description: "a token action keeps its words",
            sql: "ALTER EXTERNAL CONSUMER v1 ROTATE PAT v1 ;",
            pointer: "/0/alter_object/action",
            expected_value: json!("ROTATE PROGRAMMATIC ACCESS TOKEN"),
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
fn given_a_snowflake_tag_connection_or_dcm_project_when_parsing_then_the_tree_keeps_its_parts() {
    let test_cases = [
        StatementTestCase {
            description: "a tag's allowed values are one list, not loose text",
            sql: "CREATE TAG t1 ALLOWED_VALUES 'a' , 'b' COMMENT = 'c' ;",
            pointer: "/0/create_object/properties/0/value/tuple/expressions/1/literal/value",
            expected_value: json!("b"),
        },
        StatementTestCase {
            description: "CREATE OR ALTER TAG keeps OR ALTER",
            sql: "CREATE OR ALTER TAG t1 ;",
            pointer: "/0/create_object/or_alter",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "a connection keeps its quoted name",
            sql: "CREATE CONNECTION \"c 1\" ;",
            pointer: "/0/create_object/name/name/quoted",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "a DCM project keeps its log level",
            sql: "CREATE DCM PROJECT p1 LOG_LEVEL = WARN ;",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("LOG_LEVEL"),
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
fn given_a_snowflake_ai_function_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an AI function from an experiment run (recorded semantic error)",
            sql: "CREATE OR REPLACE AI FUNCTION IF NOT EXISTS v1 ( v1 INT , v1 INT ) RETURNS INT FROM EXPERIMENT v1 RUN v1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a plain function from an experiment run (recorded semantic error)",
            sql: "CREATE FUNCTION v1 ( ) RETURNS INT FROM EXPERIMENT v1 RUN v1 ;",
            expected_kind: Some("create_function"),
        },
        SnowflakeVerdictTestCase {
            description: "an AI function with a body (recorded semantic error)",
            sql: "CREATE AI FUNCTION v1 ( ) RETURNS INT COMMENT = m1 AS $$1 $$ ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an AI function needs RETURNS (recorded errno 1003)",
            sql: "CREATE AI FUNCTION v1 ( ) INT AS $$1 $$ ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an argument is named, not a string (recorded errno 1003)",
            sql: "CREATE AI FUNCTION v1 ( 'v1' INT ) RETURNS INT AS $$1 $$ ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a run is named, not a string (recorded errno 1003)",
            sql: "CREATE AI FUNCTION v1 ( ) RETURNS INT FROM EXPERIMENT v1 RUN 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "FROM needs EXPERIMENT (recorded errno 1003)",
            sql: "CREATE AI FUNCTION v1 ( ) RETURNS INT FROM v1 RUN v1 ;",
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
fn given_a_snowflake_execute_immediate_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a bound variable is a source, as Snowflake documents",
            sql: "BEGIN EXECUTE IMMEDIATE :q USING ( v1 ) ; END ;",
            expected_kind: Some("scripting_block"),
        },
        SnowflakeVerdictTestCase {
            description: "a RESULTSET may take what EXECUTE IMMEDIATE returns (recorded semantic error)",
            sql: "BEGIN LET r1 RESULTSET := ( EXECUTE IMMEDIATE :q1 USING ( v1 ) ) ; END ;",
            expected_kind: Some("scripting_block"),
        },
        SnowflakeVerdictTestCase {
            description: "a RESULTSET may take EXECUTE IMMEDIATE without USING (recorded semantic error)",
            sql: "BEGIN LET r1 RESULTSET := ( EXECUTE IMMEDIATE :q1 ) ; END ;",
            expected_kind: Some("scripting_block"),
        },
        SnowflakeVerdictTestCase {
            description: "an assignment without LET may take EXECUTE IMMEDIATE in parentheses",
            sql: "BEGIN r1 := ( EXECUTE IMMEDIATE :q1 USING ( v1 ) ) ; END ;",
            expected_kind: Some("scripting_block"),
        },
        SnowflakeVerdictTestCase {
            description: "an assignment's EXECUTE IMMEDIATE needs its closing parenthesis",
            sql: "BEGIN r1 := ( EXECUTE IMMEDIATE :q1 ; END ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "only an assignment takes EXECUTE IMMEDIATE in parentheses, not RETURN",
            sql: "BEGIN RETURN ( EXECUTE IMMEDIATE :q1 ) ; END ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a source is a string or a variable, not a call",
            sql: "EXECUTE IMMEDIATE v1 ( 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a source is a string or a variable, not a cast",
            sql: "EXECUTE IMMEDIATE 'a' :: VARCHAR ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a source is a variable's name, not a dotted name",
            sql: "EXECUTE IMMEDIATE v1 . v2 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a closing parenthesis with nothing to close is no end",
            sql: "EXECUTE IMMEDIATE :q1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a source may concatenate strings and variables, as Snowflake documents",
            sql: "EXECUTE IMMEDIATE v2 || ',(80)' || ',(84)' ;",
            expected_kind: Some("execute_immediate"),
        },
        SnowflakeVerdictTestCase {
            description: "a concatenation needs its right operand",
            sql: "EXECUTE IMMEDIATE v2 || ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a session variable with bind variables (recorded semantic error)",
            sql: "EXECUTE IMMEDIATE $v1 USING ( v1 , v1 ) ;",
            expected_kind: Some("execute_immediate"),
        },
        SnowflakeVerdictTestCase {
            description: "a dollar sign alone (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE $ ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "nothing to execute (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "USING needs a variable (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE 'SELECT 1' USING ( ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "USING needs parentheses (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE 'SELECT 1' USING v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a file with arguments and a dry run (recorded semantic error)",
            sql: "EXECUTE IMMEDIATE FROM @v1.s1/p1/f1.sql USING ( v1 => 1 , v1 => 1 ) DRY_RUN = TRUE ;",
            expected_kind: Some("execute_immediate"),
        },
        SnowflakeVerdictTestCase {
            description: "a file needs FROM (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE @s1/p1/f1.sql ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a file argument needs a value (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE FROM @s1/p1/f1.sql USING ( v1 => ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a stage path has no string part (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE FROM @s1/p1/'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a stage path ends the source (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE FROM @s1/p1/ SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DRY_RUN needs a value (recorded errno 1003)",
            sql: "EXECUTE IMMEDIATE FROM @s1/p1/f1.sql DRY_RUN = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a block's variable (recorded success)",
            sql: "DECLARE v1 VARCHAR DEFAULT 'SELECT 1' ; BEGIN EXECUTE IMMEDIATE v1 USING ( v1 ) ; END ;",
            expected_kind: Some("scripting_block"),
        },
        SnowflakeVerdictTestCase {
            description: "a reserved word is no variable (recorded errno 1003)",
            sql: "DECLARE v1 VARCHAR DEFAULT 'SELECT 1' ; BEGIN EXECUTE IMMEDIATE SELECT ; END ;",
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
fn given_a_snowflake_define_of_a_tag_pipe_or_code_bundle_when_parsing_then_recorded_verdicts_hold()
{
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a tag with allowed values (recorded success)",
            sql: "DEFINE TAG d1.s1.v1 ALLOWED_VALUES 'm' , 'm' , 'm' COMMENT = 'm' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a tag needs its allowed values (recorded errno 1003)",
            sql: "DEFINE TAG d1.s1.v1 ALLOWED_VALUES ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a pipe with properties (recorded semantic error)",
            sql: "DEFINE PIPE d1.s1.v1 AUTO_INGEST = TRUE AS COPY INTO t1 FROM @s1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a pipe's body is a COPY (recorded errno 1003)",
            sql: "DEFINE PIPE d1.s1.v1 AS 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a pipe's body follows AS (recorded errno 1003)",
            sql: "CREATE PIPE v1 COPY INTO t1 FROM @s1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a code bundle from an asset (recorded semantic error)",
            sql: "DEFINE CODE BUNDLE v1 . v1 . v1 FROM 'asset://a1/' COMMENT = 'm' ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a code bundle's source follows FROM (recorded errno 1003)",
            sql: "CREATE CODE BUNDLE v1 . v1 . v1 'snow://workspace/w1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DEFINE keeps refusing another opaque kind",
            sql: "DEFINE NOTEBOOK PROJECT v1 FROM 'snow://workspace/w1' ;",
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
fn given_a_snowflake_notebook_project_or_organization_profile_when_parsing_then_recorded_verdicts_hold(
) {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a notebook project from a workspace (recorded semantic error)",
            sql: "CREATE NOTEBOOK PROJECT v1 . v1 . v1 FROM 'snow://workspace/w1' COMMENT = m1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a notebook project's name may leave out its database (recorded semantic error)",
            sql: "CREATE NOTEBOOK PROJECT . v1 . v1 FROM '@d1.s1.st1' ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a notebook project's source follows FROM (recorded errno 1003)",
            sql: "CREATE NOTEBOOK PROJECT v1 . v1 . v1 '@d1.s1.st1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a notebook project takes no IF EXISTS (recorded errno 1003)",
            sql: "CREATE NOTEBOOK PROJECT IF EXISTS v1 . v1 . v1 FROM '@d1.s1.st1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a notebook may be named PROJECT (recorded runtime error)",
            sql: "CREATE NOTEBOOK project ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a notebook named PROJECT may come from a stage (recorded runtime error)",
            sql: "CREATE NOTEBOOK project FROM '@d1.s1.st1' ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a notebook named PROJECT may take options",
            sql: "CREATE NOTEBOOK project QUERY_WAREHOUSE = w1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a reserved word is no notebook project's name",
            sql: "CREATE NOTEBOOK PROJECT set FROM 'snow://workspace/w1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "PROJECT alone is no kind (recorded errno 1003)",
            sql: "CREATE PROJECT v1 . v1 . v1 FROM '@d1.s1.st1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a profile from a stage with a version (recorded semantic error)",
            sql: "CREATE ORGANIZATION PROFILE IF NOT EXISTS v1 FROM @d1.s1.st1/p1 VERSION v1 PUBLISH = TRUE ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a profile's manifest is a string (recorded errno 1003)",
            sql: "CREATE ORGANIZATION PROFILE v1 AS m1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "only a staged profile takes a version (recorded errno 1003)",
            sql: "CREATE ORGANIZATION PROFILE v1 AS 'm' VERSION \"v1 Q\" ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a profile version is named, not a string (recorded errno 1003)",
            sql: "CREATE ORGANIZATION PROFILE v1 FROM @d1.s1.st1/p1 VERSION 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "PUBLISH needs a value (recorded errno 1003)",
            sql: "CREATE ORGANIZATION PROFILE v1 AS 'm' PUBLISH = ;",
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
fn given_a_snowflake_failover_or_replication_group_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "allowed objects are named (recorded semantic error)",
            sql: "CREATE FAILOVER GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_DATABASES = \"v1 Q\" ALLOWED_ACCOUNTS = v1 . v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an integration type is two words (recorded semantic error)",
            sql: "CREATE REPLICATION GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_INTEGRATION_TYPES = SECURITY INTEGRATIONS ALLOWED_ACCOUNTS = v1 . v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an allowed database is no string (recorded errno 1003)",
            sql: "CREATE FAILOVER GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_DATABASES = 'v1' ALLOWED_ACCOUNTS = v1 . v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an object type is no string (recorded errno 1003)",
            sql: "CREATE REPLICATION GROUP v1 OBJECT_TYPES = 'TABLE' ALLOWED_ACCOUNTS = v1 . v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an integration type is not quoted (recorded errno 1003)",
            sql: "CREATE FAILOVER GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_INTEGRATION_TYPES = \"SELECT\" ALLOWED_ACCOUNTS = v1 . v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an error integration is no string (recorded errno 1003)",
            sql: "CREATE REPLICATION GROUP v1 OBJECT_TYPES = DATABASES ALLOWED_ACCOUNTS = v1 . v1 ERROR_INTEGRATION = 'v1' ;",
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
fn given_a_snowflake_show_services_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "OF TYPE names a service type (recorded acceptance)",
            sql: "SHOW SERVICES OF TYPE USER ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "IN COMPUTE POOL names a pool (recorded semantic error)",
            sql: "SHOW SERVICES IN COMPUTE POOL v1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "EXCLUDE JOBS keeps the documented clauses after it",
            sql: "SHOW SERVICES EXCLUDE JOBS LIKE 'm' IN COMPUTE POOL v1 STARTS WITH 'm' LIMIT 1 FROM 'm' ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "LIMIT may follow OF TYPE (recorded acceptance)",
            sql: "SHOW SERVICES OF TYPE USER LIMIT 1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "OF TYPE comes straight after SERVICES, not after IN (recorded errno 1003)",
            sql: "SHOW SERVICES IN COMPUTE POOL v1 OF TYPE USER ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "OF TYPE needs a type (recorded errno 1003)",
            sql: "SHOW SERVICES OF TYPE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "EXCLUDE needs JOBS (recorded errno 1003)",
            sql: "SHOW SERVICES EXCLUDE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "COMPUTE POOL needs a name (recorded errno 1003)",
            sql: "SHOW SERVICES IN COMPUTE POOL ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "STARTS WITH takes a string (recorded errno 1003)",
            sql: "SHOW SERVICES STARTS WITH m1 ;",
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
fn given_a_snowflake_service_snapshot_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description:
                "a snapshot names its service, volume and instance (recorded semantic error)",
            sql: "CREATE SNAPSHOT v1 FROM SERVICE v1 VOLUME \"d1\" INSTANCE 0 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a snapshot set is not a service snapshot (recorded semantic error)",
            sql: "CREATE SNAPSHOT SET v1 FOR TABLE v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a snapshot needs SERVICE after FROM (recorded errno 1003)",
            sql: "CREATE SNAPSHOT v1 FROM v1 VOLUME \"d1\" INSTANCE 0 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a snapshot needs VOLUME (recorded errno 1003)",
            sql: "CREATE SNAPSHOT v1 FROM SERVICE v1 \"d1\" INSTANCE 0 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a snapshot's tags need TAG (recorded errno 1003)",
            sql: "CREATE SNAPSHOT v1 FROM SERVICE v1 VOLUME \"d1\" INSTANCE 0 ( v1 = 'm' ) ;",
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
fn given_a_snowflake_alter_user_without_a_name_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "TAG is a property name before = (recorded semantic error)",
            sql: "ALTER USER SET TAG = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "TAG is a property name to unset (recorded semantic error)",
            sql: "ALTER USER UNSET TAG ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a named user sets tags",
            sql: "ALTER USER v1 SET TAG v1 = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "setting a tag needs a user name (recorded errno 1003)",
            sql: "ALTER USER SET TAG v1 = 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "unsetting a tag needs a user name (recorded errno 1003)",
            sql: "ALTER USER UNSET TAG v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "setting a policy needs a user name (recorded errno 1003)",
            sql: "ALTER USER SET AUTHENTICATION POLICY v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "renaming needs a user name (recorded errno 1003)",
            sql: "ALTER USER RENAME TO v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a named user sets a policy",
            sql: "ALTER USER v1 SET AUTHENTICATION POLICY v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "the current user resets its password (recorded acceptance)",
            sql: "ALTER USER RESET PASSWORD ;",
            expected_kind: Some("alter_object"),
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
fn given_a_snowflake_alter_user_without_a_name_when_parsing_then_the_tree_names_no_user() {
    let test_cases = [
        StatementTestCase {
            description: "an omitted user is no name, not an empty one",
            sql: "ALTER USER ADD KEY PAIR v1 PUBLIC_KEY = 'm' ;",
            pointer: "/0/alter_object/name",
            expected_value: json!(null),
        },
        StatementTestCase {
            description: "an omitted user before a token action is no name",
            sql: "ALTER USER ROTATE PROGRAMMATIC ACCESS TOKEN v1 ;",
            pointer: "/0/alter_object/name",
            expected_value: json!(null),
        },
        StatementTestCase {
            description: "a named user keeps its name",
            sql: "ALTER USER u1 ADD KEY PAIR v1 PUBLIC_KEY = 'm' ;",
            pointer: "/0/alter_object/name/name/name",
            expected_value: json!("u1"),
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
fn given_a_snowflake_external_agent_task_list_or_user_group_when_parsing_then_recorded_verdicts_hold(
) {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an external agent takes WITH VERSION (recorded acceptance)",
            sql: "CREATE EXTERNAL AGENT v1 WITH VERSION v1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an external agent takes no COMMENT (recorded errno 1003)",
            sql: "CREATE EXTERNAL AGENT v1 COMMENT = 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an external agent's version is named (recorded errno 1003)",
            sql: "CREATE EXTERNAL AGENT v1 WITH VERSION 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "WITH needs VERSION (recorded errno 1003)",
            sql: "CREATE EXTERNAL AGENT v1 WITH v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ROOT ONLY follows STARTS WITH (recorded acceptance)",
            sql:
                "SHOW TERSE TASKS LIKE 'm' IN ACCOUNT STARTS WITH 'm' ROOT ONLY LIMIT 1 FROM 'm' ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "ROOT needs ONLY (recorded errno 1003)",
            sql: "SHOW TASKS ROOT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an organization user group takes properties (recorded semantic error)",
            sql: "CREATE ORGANIZATION USER GROUP IF NOT EXISTS v1 IS_GRANTABLE = TRUE ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an organization user group needs a name (recorded errno 1003)",
            sql: "CREATE ORGANIZATION USER GROUP ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a connector adds its live version from LAST (recorded semantic error)",
            sql: "ALTER OPENFLOW CONNECTOR v1 ADD LIVE VERSION FROM LAST COMMENT = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a connector adds a version from a location (recorded semantic error)",
            sql: "ALTER OPENFLOW CONNECTOR v1 ADD VERSION FROM 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a live version comes from LAST (recorded errno 1003)",
            sql: "ALTER OPENFLOW CONNECTOR v1 ADD LIVE VERSION FROM v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a version comes from a location, not LAST (recorded errno 1003)",
            sql: "ALTER OPENFLOW CONNECTOR v1 ADD VERSION FROM LAST ;",
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
fn given_a_snowflake_external_agent_or_filtered_show_when_parsing_then_the_tree_keeps_its_words() {
    let test_cases = [
        StatementTestCase {
            description: "an external agent's text keeps EXTERNAL",
            sql: "CREATE EXTERNAL AGENT v1 WITH VERSION v1 ;",
            pointer: "/0/raw/sql",
            expected_value: json!("CREATE EXTERNAL AGENT v1 WITH VERSION v1"),
        },
        StatementTestCase {
            description: "EXECUTE IMMEDIATE keeps a bound variable as its source",
            sql: "EXECUTE IMMEDIATE :q1 USING ( v1 , v2 ) ;",
            pointer: "/0/execute_immediate/source/parameter/name",
            expected_value: json!("q1"),
        },
        StatementTestCase {
            description: "EXECUTE IMMEDIATE keeps each USING variable",
            sql: "EXECUTE IMMEDIATE :q1 USING ( v1 , v2 ) ;",
            pointer: "/0/execute_immediate/using/1/column/name/name",
            expected_value: json!("v2"),
        },
        StatementTestCase {
            description: "EXECUTE IMMEDIATE joins a concatenated source left to right",
            sql: "EXECUTE IMMEDIATE v2 || ',(80)' || $s1 ;",
            pointer: "/0/execute_immediate/source/concat/right/parameter/style",
            expected_value: json!("Dollar"),
        },
        StatementTestCase {
            description: "EXECUTE IMMEDIATE FROM keeps its named arguments and dry run",
            sql: "EXECUTE IMMEDIATE FROM @v1.s1/p1/f1.sql USING ( v1 => 1 ) DRY_RUN = FALSE ;",
            pointer: "/0/execute_immediate/dry_run",
            expected_value: json!(false),
        },
        StatementTestCase {
            description: "a RESULTSET keeps EXECUTE IMMEDIATE as a node",
            sql: "BEGIN LET r1 RESULTSET := ( EXECUTE IMMEDIATE :q1 ) ; END ;",
            pointer: "/0/scripting_block/body/0/let/declaration/resultset/default/statement/execute_immediate/source/parameter/name",
            expected_value: json!("q1"),
        },
        StatementTestCase {
            description: "a DEFINE kept as text says DEFINE",
            sql: "DEFINE CODE BUNDLE v1 . v1 . v1 FROM 'asset://a1/' ;",
            pointer: "/0/raw/sql",
            expected_value: json!("DEFINE CODE BUNDLE v1 . v1 . v1 FROM 'asset://a1/'"),
        },
        StatementTestCase {
            description: "SHOW TASKS keeps ROOT ONLY as a filter, not in its kind",
            sql: "SHOW TASKS STARTS WITH 'm' ROOT ONLY ;",
            pointer: "/0/show/root_only",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "SHOW TASKS with ROOT ONLY keeps TASKS as its kind",
            sql: "SHOW TASKS STARTS WITH 'm' ROOT ONLY ;",
            pointer: "/0/show/this",
            expected_value: json!("TASKS"),
        },
        StatementTestCase {
            description: "SHOW SERVICES keeps its filters in its kind",
            sql: "SHOW SERVICES OF TYPE USER , JOB IN COMPUTE POOL v1 LIMIT 1 ;",
            pointer: "/0/show/this",
            expected_value: json!("SERVICES OF TYPE USER, JOB"),
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
fn given_a_snowflake_show_with_privileges_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a documented privilege (recorded acceptance)",
            sql: "SHOW DATABASES WITH PRIVILEGES USAGE , USAGE ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "a privilege of several words",
            sql: "SHOW SCHEMAS WITH PRIVILEGES USAGE , CREATE TABLE ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "a name that is no privilege (recorded errno 1003)",
            sql: "SHOW WAREHOUSES WITH PRIVILEGES v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a trailing comma (recorded errno 1003)",
            sql: "SHOW SCHEMAS WITH PRIVILEGES USAGE , ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a privilege list may end a RESULTSET's query",
            sql: "BEGIN LET r1 RESULTSET := ( SHOW DATABASES WITH PRIVILEGES USAGE ) ; END ;",
            expected_kind: Some("scripting_block"),
        },
        SnowflakeVerdictTestCase {
            description: "a privilege list may flow into the next statement",
            sql: "SHOW DATABASES WITH PRIVILEGES USAGE ->> SELECT 1 ;",
            expected_kind: Some("operator"),
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
fn given_a_snowflake_variable_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a variable with its type, default and comment (recorded semantic error)",
            sql: "CREATE OR REPLACE VARIABLE IF NOT EXISTS v1 INT DEFAULT 1 COMMENT = 'm' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a variable needs a type (recorded errno 1003)",
            sql: "CREATE VARIABLE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a type is not a string (recorded errno 1003)",
            sql: "CREATE VARIABLE v1 'INT' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DEFAULT needs a value (recorded errno 1003)",
            sql: "CREATE VARIABLE v1 INT DEFAULT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a comment is a string (recorded errno 1003)",
            sql: "CREATE VARIABLE v1 INT COMMENT = m1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a variable kept as text may flow into the next statement",
            sql: "CREATE VARIABLE v1 INT DEFAULT 1 ->> SELECT 1 ;",
            expected_kind: Some("raw"),
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
fn given_a_snowflake_describe_of_a_metric_function_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "TABLE arguments with column types (recorded semantic error)",
            sql: "DESC FUNCTION v1 ( TABLE ( INT ) , TABLE ( INT , INT ) ) ;",
            expected_kind: Some("describe"),
        },
        SnowflakeVerdictTestCase {
            description: "a TABLE argument needs a column type (recorded errno 1003)",
            sql: "DESC FUNCTION v1 ( TABLE ( INT ) , TABLE ( ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an argument does not open with a parenthesis (recorded errno 1003)",
            sql: "DESC FUNCTION v1 ( TABLE ( INT ) , ( INT ) ) ;",
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
fn given_a_snowflake_warehouse_or_connector_action_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a named warehouse adds tables (recorded semantic error)",
            sql: "ALTER WAREHOUSE v1 ADD TABLES ( v1 ) ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "the current warehouse aborts its queries (recorded acceptance)",
            sql: "ALTER WAREHOUSE ABORT ALL QUERIES ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "adding tables needs a warehouse name (recorded errno 1002)",
            sql: "ALTER WAREHOUSE ADD TABLES ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "dropping tables needs TABLES (recorded errno 1003)",
            sql: "ALTER WAREHOUSE v1 DROP ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a table list is not empty (recorded errno 1003)",
            sql: "ALTER WAREHOUSE v1 ADD TABLES ( ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a table is named, not a string (recorded errno 1003)",
            sql: "ALTER WAREHOUSE v1 DROP TABLES ( 'v1' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ABORT needs ALL QUERIES (recorded errno 1003)",
            sql: "ALTER WAREHOUSE ABORT QUERIES ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "renaming needs a warehouse name (recorded errno 1003)",
            sql: "ALTER WAREHOUSE RENAME TO v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "IF EXISTS does not name the warehouse to rename",
            sql: "ALTER WAREHOUSE IF EXISTS RENAME TO v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "IF EXISTS does not name the warehouse to add tables to",
            sql: "ALTER WAREHOUSE IF EXISTS ADD TABLES ( v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "IF EXISTS may still alter the current warehouse",
            sql: "ALTER WAREHOUSE IF EXISTS SUSPEND ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an unset tag is named (recorded errno 1003)",
            sql: "ALTER WAREHOUSE v1 UNSET TAG 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a connector pushes to a location (recorded semantic error)",
            sql: "ALTER OPENFLOW CONNECTOR v1 PUSH TO 'm' USERNAME = 'm' PASSWORD = 'm' NAME = 'm' EMAIL = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a push location is a string (recorded errno 1003)",
            sql: "ALTER OPENFLOW CONNECTOR v1 PUSH TO m1 USERNAME = 'm' PASSWORD = 'm' NAME = 'm' EMAIL = 'm' ;",
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
fn given_a_snowflake_event_table_with_a_contact_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an event table copies grants (recorded acceptance)",
            sql: "CREATE EVENT TABLE v1 COPY = ( GRANTS ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an event table takes no CONTACT (recorded errno 1003)",
            sql: "CREATE EVENT TABLE v1 WITH CONTACT ( STEWARD = v1 ) ;",
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
fn given_a_snowflake_contact_purpose_or_reserved_name_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a quoted contact purpose on ALTER TABLE SET (recorded semantic error)",
            sql: "ALTER TABLE v1 SET CONTACT 'v1' = v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a quoted contact purpose on ALTER DATABASE UNSET (recorded semantic error)",
            sql: "ALTER DATABASE v1 UNSET CONTACT 'v1' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a quoted contact purpose on ALTER ACCOUNT SET (recorded semantic error)",
            sql: "ALTER ACCOUNT SET CONTACT 'v1' = v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a quoted contact purpose on ALTER TASK GRAPH UNSET (recorded semantic error)",
            sql: "ALTER TASK GRAPH v1 UNSET CONTACT 'v1' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a quoted contact purpose in a view's WITH CONTACT (recorded semantic error)",
            sql: "CREATE VIEW v1 WITH CONTACT ( 'v1' = v1 ) AS SELECT 1 ;",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "a quoted contact purpose in a schema's WITH CONTACT (recorded semantic error)",
            sql: "CREATE SCHEMA v1 WITH CONTACT ( 'v1' = v1 ) ;",
            expected_kind: Some("create_schema"),
        },
        SnowflakeVerdictTestCase {
            description: "a quoted contact purpose in an Iceberg table's WITH CONTACT (recorded semantic error)",
            sql: "CREATE ICEBERG TABLE v1 BASE_LOCATION = 'm' WITH CONTACT ( 'v1' = v1 ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "ALTER MATERIALIZED VIEW takes no quoted contact purpose (recorded errno 1003)",
            sql: "ALTER MATERIALIZED VIEW v1 SET CONTACT 'v1' = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ALTER DYNAMIC TABLE takes no quoted contact purpose (recorded errno 1003)",
            sql: "ALTER DYNAMIC TABLE v1 UNSET CONTACT 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a dynamic table clone takes a named TARGET_LAG (recorded semantic error)",
            sql: "CREATE DYNAMIC TABLE v1 CLONE v1 TARGET_LAG = m1 WAREHOUSE = v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a dynamic table clone needs a TARGET_LAG value (recorded errno 1003)",
            sql: "CREATE DYNAMIC TABLE v1 CLONE v1 TARGET_LAG = WAREHOUSE = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "LIKE takes no unquoted reserved word (recorded errno 1003)",
            sql: "CREATE TABLE v1 LIKE SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "LIKE takes a quoted reserved word",
            sql: "CREATE TABLE v1 LIKE \"SELECT\" ;",
            expected_kind: Some("create_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a hybrid table's LIKE takes no unquoted reserved word (recorded errno 1003)",
            sql: "CREATE HYBRID TABLE v1 LIKE SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "INSERT INTO takes no unquoted reserved word (recorded errno 1003)",
            sql: "INSERT INTO SELECT VALUES ( 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "INSERT INTO takes a quoted reserved word",
            sql: "INSERT INTO \"SELECT\" VALUES ( 1 ) ;",
            expected_kind: Some("insert"),
        },
        SnowflakeVerdictTestCase {
            description: "MERGE INTO takes no unquoted reserved word (recorded errno 1003)",
            sql: "MERGE INTO SELECT USING t2 ON v1.c1 = t2.c1 WHEN MATCHED THEN UPDATE ALL BY NAME ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET ( ... ) takes no unquoted reserved word (recorded errno 1003)",
            sql: "SET ( SELECT ) = ( 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET ( ... ) takes no string (recorded errno 1003)",
            sql: "SET ( 'v1' ) = ( 1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "UNSET takes no unquoted reserved word (recorded errno 1003)",
            sql: "UNSET SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "UNSET ( ... ) takes a quoted reserved word",
            sql: "UNSET ( v1 , \"SELECT\" ) ;",
            expected_kind: Some("unset_statement"),
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
fn given_a_snowflake_quoted_contact_purpose_or_set_variable_when_parsing_then_the_tree_keeps_its_quotes(
) {
    let test_cases = [
        StatementTestCase {
            description: "a string contact purpose keeps its text",
            sql: "ALTER TABLE v1 SET CONTACT 'v1' = v1 ;",
            pointer: "/0/alter_object/properties/0/name/name",
            expected_value: json!("v1"),
        },
        StatementTestCase {
            description: "a string contact purpose is marked as a string",
            sql: "ALTER TABLE v1 SET CONTACT 'v1' = v1 ;",
            pointer: "/0/alter_object/properties/0/string_name",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "a string contact purpose keeps its span",
            sql: "ALTER TABLE v1 SET CONTACT 'v1' = v1 ;",
            pointer: "/0/alter_object/properties/0/name/span/start",
            expected_value: json!(27),
        },
        StatementTestCase {
            description: "an identifier contact purpose is not marked as a string",
            sql: "ALTER TABLE v1 SET CONTACT v1 = v1 ;",
            pointer: "/0/alter_object/properties/0/string_name",
            expected_value: json!(null),
        },
        StatementTestCase {
            description: "a quoted UNSET parameter keeps its quoting",
            sql: "ALTER WAREHOUSE v1 UNSET \"SELECT\" ;",
            pointer: "/0/alter_object/properties/0/quoted",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "an unquoted UNSET parameter is not quoted",
            sql: "ALTER WAREHOUSE v1 UNSET COMMENT ;",
            pointer: "/0/alter_object/properties/0/quoted",
            expected_value: json!(null),
        },
        StatementTestCase {
            description: "an optional WITH before TAG stays in the property kind",
            sql: "CREATE TABLE t1 ( c1 INT ) WITH TAG ( v1 = 'm' ) ;",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("WITH TAG"),
        },
        StatementTestCase {
            description: "a TAG without WITH has the bare kind",
            sql: "CREATE TABLE t1 ( c1 INT ) TAG ( v1 = 'm' ) ;",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("TAG"),
        },
        StatementTestCase {
            description: "FORCE is a property, not part of the action",
            sql: "ALTER OPENFLOW RUNTIME v1 UPGRADE FORCE ;",
            pointer: "/0/alter_object/action",
            expected_value: json!("UPGRADE"),
        },
        StatementTestCase {
            description: "a quoted SET variable stays quoted",
            sql: "SET ( v1 , \"v1 Q\" ) = ( 1 , 2 ) ;",
            pointer: "/0/set_statement/items/0/name/tuple/expressions/1/column/name/quoted",
            expected_value: json!(true),
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
fn given_a_snowflake_lone_kind_word_or_omitted_name_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a lone BACKUP SET container has no name (recorded semantic error)",
            sql: "SHOW BACKUPS IN BACKUP SET ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "a lone GRAPH is a name (recorded semantic error)",
            sql: "SHOW COMMITTED TASKS IN GRAPH ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "VERSIONS takes a bare name (recorded unsupported feature)",
            sql: "SHOW VERSIONS IN v1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "a DCM PROJECT container still needs its name (recorded errno 1003)",
            sql: "SHOW DEPLOYMENTS IN DCM PROJECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "VOLUMES alone is a kind (recorded semantic error)",
            sql: "SHOW VOLUMES ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "BY takes a user name without USER (recorded runtime error)",
            sql: "SHOW DELEGATED AUTHORIZATIONS BY v1 ;",
            expected_kind: Some("show"),
        },
        SnowflakeVerdictTestCase {
            description: "an unquoted reserved keyword is no SHOW scope (recorded errno 1003)",
            sql: "SHOW PRIMARY KEYS IN SCHEMA SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ARTIFACT alone is a DESCRIBE kind (recorded semantic error)",
            sql: "DESC ARTIFACT v1 ;",
            expected_kind: Some("describe"),
        },
        SnowflakeVerdictTestCase {
            description: "DESC ARTIFACT REPOSITORY still needs its name (recorded errno 1003)",
            sql: "DESC ARTIFACT REPOSITORY ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty TYPE = is refused (recorded errno 1003)",
            sql: "DESCRIBE TABLE v1 TYPE = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "EVENT TABLE is a DROP kind (recorded semantic error)",
            sql: "DROP EVENT TABLE v1 ;",
            expected_kind: Some("drop_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an online feature table's name may be left out before SET (recorded semantic error)",
            sql: "ALTER ONLINE FEATURE TABLE SET COMMENT = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a role's comment is set (recorded semantic error)",
            sql: "ALTER ROLE v1 SET COMMENT = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a deployment without OPENFLOW is not renamed (recorded errno 1003)",
            sql: "ALTER DEPLOYMENT v1 RENAME TO v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty ROOTS list (recorded OK)",
            sql: "CREATE TASK GRAPH v1 ROOTS = ( ) ;",
            expected_kind: Some("create_task"),
        },
        SnowflakeVerdictTestCase {
            description: "a task with no properties (recorded semantic error)",
            sql: "CREATE TASK v1 ;",
            expected_kind: Some("create_task"),
        },
        SnowflakeVerdictTestCase {
            description: "FORMAT alone is no CREATE kind (recorded errno 1003)",
            sql: "CREATE FORMAT v1 CLONE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "GRAPH alone is no EXECUTE kind (recorded errno 1003)",
            sql: "EXECUTE GRAPH v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "PRECISION alone is no type (recorded errno 1003)",
            sql: "SELECT CAST ( NULL AS PRECISION ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DOUBLE PRECISION is a type (documented)",
            sql: "SELECT CAST ( NULL AS DOUBLE PRECISION ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "a session variable's name is not quoted (recorded errno 1003)",
            sql: "SELECT $\"v1 Q\" ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET needs a variable name (recorded errno 1003)",
            sql: "SET CURRENT = 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an index is not on a reserved keyword (recorded errno 1003)",
            sql: "CREATE INDEX v1 ON SELECT ( v1 ) ;",
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
fn given_a_snowflake_create_or_alter_task_when_parsing_then_or_alter_is_kept() {
    let test_cases = [
        StatementTestCase {
            description: "each task property is its own property",
            sql: "CREATE TASK v1 WAREHOUSE = w1 SCHEDULE = '1 MINUTE' AS SELECT 1 ;",
            pointer: "/0/create_task/task_properties/1/kind",
            expected_value: json!("SCHEDULE"),
        },
        StatementTestCase {
            description: "a property value keeps its literal",
            sql: "CREATE TASK v1 WAREHOUSE = w1 SCHEDULE = '1 MINUTE' AS SELECT 1 ;",
            pointer: "/0/create_task/task_properties/1/value/literal/value",
            expected_value: json!("1 MINUTE"),
        },
        StatementTestCase {
            description: "a contact pair keeps its purpose",
            sql: "CREATE TASK GRAPH v1 WITH CONTACT ( steward = c1 ) ;",
            pointer: "/0/create_task/task_properties/0/name/name",
            expected_value: json!("steward"),
        },
        StatementTestCase {
            description: "AFTER lists its predecessors",
            sql: "CREATE TASK v1 AFTER t1 , t2 AS SELECT 1 ;",
            pointer: "/0/create_task/task_properties/0/value/tuple/expressions/1/table/name/name",
            expected_value: json!("t2"),
        },
        StatementTestCase {
            description: "CREATE OR ALTER TASK keeps OR ALTER",
            sql: "CREATE OR ALTER TASK v1 WAREHOUSE = w1 AS SELECT 1 ;",
            pointer: "/0/create_task/or_alter",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "CREATE OR ALTER TASK GRAPH keeps OR ALTER",
            sql: "CREATE OR ALTER TASK GRAPH v1 ROOTS = ( ) ;",
            pointer: "/0/create_task/or_alter",
            expected_value: json!(true),
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
fn given_a_snowflake_put_to_a_table_stage_after_a_dot_when_parsing_then_the_target_is_whole() {
    let test_cases = [
        StatementTestCase {
            description:
                "the current schema's table stage keeps its table name (recorded acceptance)",
            sql: "PUT file:///tmp/f1 @.%v1 OVERWRITE = TRUE ;",
            pointer: "/0/put/target/literal/value",
            expected_value: json!("@.%v1"),
        },
        StatementTestCase {
            description: "the table name is not read as a parameter",
            sql: "PUT file:///tmp/f1 @.%v1 OVERWRITE = TRUE ;",
            pointer: "/0/put/params/0/name",
            expected_value: json!("OVERWRITE"),
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
fn given_a_snowflake_lone_contact_or_unnamed_snapshot_policy_when_parsing_then_the_tree_is_structured(
) {
    let test_cases = [
        StatementTestCase {
            description: "a single table contact keeps its purpose",
            sql: "ALTER TABLE v1 SET CONTACT STEWARD = \"v1 Q\" ;",
            pointer: "/0/alter_object/properties/0/name/name",
            expected_value: json!("STEWARD"),
        },
        StatementTestCase {
            description: "a single table contact keeps its quoted name",
            sql: "ALTER TABLE v1 SET CONTACT STEWARD = \"v1 Q\" ;",
            pointer: "/0/alter_object/properties/0/value/table/name/quoted",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "SNAPSHOT POLICY before SET names a snapshot (recorded errno 2003)",
            sql: "ALTER SNAPSHOT POLICY SET COMMENT = 'm' ;",
            pointer: "/0/alter_object/name/name/name",
            expected_value: json!("POLICY"),
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
    assert_eq!(
        snowflake_kind("ALTER SNAPSHOT POLICY UNSET COMMENT ;"),
        None,
        "a snapshot only sets its comment (recorded errno 1003)"
    );
}

#[test]
fn given_snowflake_listing_filters_routine_maps_and_iceberg_versions_when_parsing_then_recorded_verdicts_hold(
) {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an Iceberg table reads at a version tag (recorded errno 2003)",
            sql: "SELECT 1 FROM t1 AT ( VERSION_TAG => 'm' ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "external function headers may use bare words (recorded errno 2169)",
            sql: "CREATE EXTERNAL FUNCTION v1 ( ) RETURNS INT API_INTEGRATION = v1 HEADERS = ( m1 = 'm' ) AS 'm' ;",
            expected_kind: Some("create_function"),
        },
        SnowflakeVerdictTestCase {
            description: "a request translator may be a string (recorded semantic error)",
            sql: "CREATE EXTERNAL FUNCTION v1 ( ) RETURNS INT API_INTEGRATION = v1 REQUEST_TRANSLATOR = 'v1' AS 'm' ;",
            expected_kind: Some("create_function"),
        },
        SnowflakeVerdictTestCase {
            description: "a named argument does not take an unquoted stage (recorded errno 1003)",
            sql: "SELECT * FROM TABLE ( INFER_SCHEMA ( LOCATION => @s1 , FILE_FORMAT => 'm' ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "METRIC alone is not a kind (recorded errno 1003)",
            sql: "CREATE METRIC FUNCTION v1 ( v1 TABLE ( v1 INT ) ) RETURNS NUMBER AS 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "IF after a kind starts IF NOT EXISTS (recorded errno 1003)",
            sql: "CREATE OR ALTER v1 IF ( EXISTS ( SELECT 1 ) ) THEN SELECT 1 ;",
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
fn given_snowflake_listing_filters_or_a_volatile_file_format_when_parsing_then_they_are_kept() {
    let test_cases = [
        StatementTestCase {
            description: "listing filters are kept in order",
            sql: "SHOW TERSE AVAILABLE LISTINGS LIMIT 1 IS_IMPORTED = TRUE IS_SHARED_WITH_ME = TRUE ;",
            pointer: "/0/show/listing_filters",
            expected_value: json!(["IS_IMPORTED", "IS_SHARED_WITH_ME"]),
        },
        StatementTestCase {
            description: "a listing filter needs no LIMIT before it",
            sql: "SHOW AVAILABLE LISTINGS IS_ORGANIZATION = TRUE ;",
            pointer: "/0/show/listing_filters",
            expected_value: json!(["IS_ORGANIZATION"]),
        },
        StatementTestCase {
            description: "a volatile file format keeps VOLATILE",
            sql: "CREATE OR ALTER VOLATILE FILE FORMAT v1 ;",
            pointer: "/0/create_object/modifier",
            expected_value: json!("VOLATILE"),
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
fn given_snowflake_numeric_operands_given_a_string_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "SEQUENCE_NUMBER takes a number (recorded semantic error)",
            sql: "ALTER APPLICATION v1 APPROVE SPECIFICATION v1 SEQUENCE_NUMBER = 1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "SEQUENCE_NUMBER refuses a string (recorded errno 1003)",
            sql: "ALTER APPLICATION v1 APPROVE SPECIFICATION v1 SEQUENCE_NUMBER = 'x' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "GROUP LIMIT refuses a string (recorded errno 1003)",
            sql: "ALTER TABLE v1 ADD DATA METRIC FUNCTION v1 ON ( v1 ) WITHIN GROUP ( v1 ) GROUP LIMIT 'x' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SHOW AVAILABLE LISTINGS LIMIT refuses a string (recorded errno 1003)",
            sql: "SHOW AVAILABLE LISTINGS LIMIT 'x' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "MAX_BATCH_ROWS reads a string (recorded semantic error)",
            sql: "CREATE EXTERNAL FUNCTION v1 ( ) RETURNS INT API_INTEGRATION = v1 MAX_BATCH_ROWS = 'x' AS 'm' ;",
            expected_kind: Some("create_function"),
        },
        SnowflakeVerdictTestCase {
            description: "BATCH_TIMEOUT_SECS reads a string (recorded semantic error)",
            sql: "CREATE FUNCTION v1 ( ) RETURNS INT SERVICE = v1 ENDPOINT = e1 BATCH_TIMEOUT_SECS = 'x' AS 'm' ;",
            expected_kind: Some("create_function"),
        },
        SnowflakeVerdictTestCase {
            description: "a semantic view reads MAX_STALENESS as a string (recorded semantic error)",
            sql: "CREATE SEMANTIC VIEW v1 TABLES ( v1 ) MAX_STALENESS = 'x' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a semantic view refuses a number as MAX_STALENESS (recorded errno 1003)",
            sql: "CREATE SEMANTIC VIEW v1 TABLES ( v1 ) MAX_STALENESS = 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an interactive warehouse needs a name (recorded errno 1003)",
            sql: "CREATE INTERACTIVE WAREHOUSE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an interactive warehouse takes IF NOT EXISTS, not IF EXISTS (recorded errno 1003)",
            sql: "CREATE INTERACTIVE WAREHOUSE IF EXISTS v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an interactive warehouse takes no bare list after its name (recorded errno 1003)",
            sql: "CREATE INTERACTIVE WAREHOUSE v1 ( v1 = 'm' ) ;",
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
fn given_snowflake_routine_modifiers_staleness_or_a_snapshot_named_policy_when_parsing_then_they_are_kept(
) {
    let test_cases = [
        StatementTestCase {
            description: "a semantic view keeps MAX_STALENESS as a property",
            sql: "CREATE SEMANTIC VIEW v1 TABLES ( v1 ) MAX_STALENESS = '1 hour' COMMENT = 'm' ;",
            pointer: "/0/create_object/properties/1/kind",
            expected_value: json!("MAX_STALENESS"),
        },
        StatementTestCase {
            description: "a function keeps SECURE",
            sql: "CREATE SECURE EXTERNAL FUNCTION v1 ( ) RETURNS INT API_INTEGRATION = v1 AS 'm' ;",
            pointer: "/0/create_function/secure",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "a function property keeps its name's quoting",
            sql: "CREATE EXTERNAL FUNCTION v1 ( ) RETURNS INT API_INTEGRATION = \"i 1\" AS 'm' ;",
            pointer: "/0/create_function/snowflake_properties/0/value/table/name/quoted",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "a map property keeps each key with its value",
            sql: "CREATE FUNCTION v1 ( ) RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = '3.12' HANDLER = 'h' SECRETS = ( 'k' = s1 ) AS 'm' ;",
            pointer: "/0/create_function/snowflake_properties/0/value/tuple/expressions/0/eq/right/table/name/name",
            expected_value: json!("s1"),
        },
        StatementTestCase {
            description: "a procedure option phrase is its own property",
            sql: "CREATE PROCEDURE v1 ( ) COPY GRANTS RETURNS INT NOT NULL LANGUAGE SQL AS 'm' ;",
            pointer: "/0/create_procedure/snowflake_properties/1/kind",
            expected_value: json!("NOT NULL"),
        },
        StatementTestCase {
            description: "a function keeps EXTERNAL",
            sql: "CREATE SECURE EXTERNAL FUNCTION v1 ( ) RETURNS INT API_INTEGRATION = v1 AS 'm' ;",
            pointer: "/0/create_function/external",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "a procedure keeps SECURE",
            sql: "CREATE TEMPORARY SECURE PROCEDURE v1 ( ) RETURNS INT LANGUAGE SQL AS 'm' ;",
            pointer: "/0/create_procedure/secure",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "a procedure keeps TEMPORARY",
            sql: "CREATE TEMPORARY SECURE PROCEDURE v1 ( ) RETURNS INT LANGUAGE SQL AS 'm' ;",
            pointer: "/0/create_procedure/temporary",
            expected_value: json!(true),
        },
        StatementTestCase {
            description: "SET COMMENT after SNAPSHOT POLICY alters a snapshot named POLICY (recorded errno 2003)",
            sql: "ALTER SNAPSHOT POLICY SET COMMENT = 'm' ;",
            pointer: "/0/alter_object/kind",
            expected_value: json!("SNAPSHOT"),
        },
        StatementTestCase {
            description: "another SET after SNAPSHOT POLICY stays a snapshot policy",
            sql: "ALTER SNAPSHOT POLICY SET EXPIRE_AFTER_DAYS = 7 ;",
            pointer: "/0/alter_object/kind",
            expected_value: json!("SNAPSHOT POLICY"),
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
