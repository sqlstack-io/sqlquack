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
            description: "a session variable with bind variables (recorded semantic error)",
            sql: "EXECUTE IMMEDIATE $v1 USING ( v1 , v1 ) ;",
            expected_kind: Some("command"),
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
            expected_kind: Some("command"),
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
            expected_kind: Some("raw"),
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
