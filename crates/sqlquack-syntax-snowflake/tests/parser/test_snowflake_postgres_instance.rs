use crate::helpers::snowflake_kind;
use crate::test_types::SnowflakeVerdictTestCase;

#[test]
fn given_a_snowflake_create_postgres_instance_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "keeps the required properties",
            sql: "CREATE POSTGRES INSTANCE v1 COMPUTE_FAMILY = 'm' STORAGE_SIZE_GB = 10 AUTHENTICATION_AUTHORITY = POSTGRES ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps a fork with time travel, properties and WITH TAG",
            sql: "CREATE POSTGRES INSTANCE IF NOT EXISTS v1 FORK v1 AT ( OFFSET => -60 ) COMPUTE_FAMILY = 'm' STORAGE_SIZE_GB = 10 HIGH_AVAILABILITY = TRUE POSTGRES_SETTINGS = 'm' COMMENT = 'm' WITH TAG ( v1 = 'm' , v1 = 'm' ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps an empty AUTHENTICATION_PROPERTIES list",
            sql: "CREATE POSTGRES INSTANCE v1 COMPUTE_FAMILY = 'm' STORAGE_SIZE_GB = 10 AUTHENTICATION_AUTHORITY = POSTGRES AUTHENTICATION_PROPERTIES = ( ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a property value is one token (recorded errno 1003)",
            sql: "CREATE POSTGRES INSTANCE v1 COMPUTE_FAMILY = 'm' STORAGE_SIZE_GB = 10 AUTHENTICATION_AUTHORITY = POSTGRES POSTGRES_VERSION = 1 6 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a tag list needs TAG (recorded errno 1003)",
            sql: "CREATE POSTGRES INSTANCE v1 COMPUTE_FAMILY = 'm' STORAGE_SIZE_GB = 10 AUTHENTICATION_AUTHORITY = POSTGRES ( v1 = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "the name is required (recorded errno 1003)",
            sql: "CREATE POSTGRES INSTANCE FORK v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "FORK needs its source (recorded errno 1003)",
            sql: "CREATE POSTGRES INSTANCE v1 FORK ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a word after the name is not a property (recorded errno 1003)",
            sql: "CREATE POSTGRES INSTANCE v1 v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "time travel needs its kind (recorded errno 1003)",
            sql: "CREATE POSTGRES INSTANCE v1 FORK v1 AT ( => -60 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "time travel needs its value (recorded errno 1003)",
            sql: "CREATE POSTGRES INSTANCE v1 FORK v1 AT ( TIMESTAMP => ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "CREATE needs POSTGRES before INSTANCE (recorded errno 1003)",
            sql: "CREATE INSTANCE v1 FORK v1 ;",
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
fn given_a_snowflake_alter_postgres_instance_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "keeps RENAME with a name and no TO",
            sql: "ALTER POSTGRES INSTANCE v1 RENAME v1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "RENAME takes no TO (recorded errno 1003)",
            sql: "ALTER POSTGRES INSTANCE IF EXISTS v1 RENAME TO v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps RESET ACCESS FOR a role string",
            sql: "ALTER POSTGRES INSTANCE IF EXISTS v1 RESET ACCESS FOR 'snowflake_admin' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "RESET needs ACCESS FOR (recorded errno 1003)",
            sql: "ALTER POSTGRES INSTANCE v1 RESET FOR 'snowflake_admin' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps SUSPEND",
            sql: "ALTER POSTGRES INSTANCE IF EXISTS v1 SUSPEND ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SET of a named and an empty list value",
            sql: "ALTER POSTGRES INSTANCE v1 SET COMMENT = m1 AUTHENTICATION_PROPERTIES = ( ) ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps SET TAG = as a property",
            sql: "ALTER POSTGRES INSTANCE v1 SET TAG = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a SET value is one token (recorded errno 1003)",
            sql: "ALTER POSTGRES INSTANCE v1 SET POSTGRES_VERSION = 1 7 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET has no APPLY clause (recorded errno 1003)",
            sql: "ALTER POSTGRES INSTANCE v1 SET APPLY ON 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps UNSET of properties and a bare TAG",
            sql: "ALTER POSTGRES INSTANCE IF EXISTS v1 UNSET COMMENT , TAG ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "UNSET TAG takes names (recorded errno 1003)",
            sql: "ALTER POSTGRES INSTANCE v1 UNSET TAG 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "ALTER needs POSTGRES before INSTANCE (recorded errno 1003)",
            sql: "ALTER INSTANCE v1 RESUME ;",
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
fn given_a_snowflake_drop_or_describe_of_an_instance_when_parsing_then_postgres_may_be_left_out() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "keeps DROP INSTANCE (recorded semantic error)",
            sql: "DROP INSTANCE v1 ;",
            expected_kind: Some("drop_object"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps DESC INSTANCE (recorded unsupported feature)",
            sql: "DESC INSTANCE v1 ;",
            expected_kind: Some("describe"),
        },
        SnowflakeVerdictTestCase {
            description: "keeps DROP POSTGRES INSTANCE",
            sql: "DROP POSTGRES INSTANCE IF EXISTS v1 ;",
            expected_kind: Some("drop_object"),
        },
        SnowflakeVerdictTestCase {
            description: "DROP INSTANCE needs a name",
            sql: "DROP INSTANCE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DESC INSTANCE takes no string name",
            sql: "DESC INSTANCE 'v1' ;",
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
