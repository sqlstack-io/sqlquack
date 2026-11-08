use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_snowflake_table_sequence_and_share_actions_when_parsing_then_only_documented_forms_are_accepted(
) {
    let test_cases = [
        AcceptanceTestCase { description: "a sequence is renamed (recorded semantic error)", sql: "ALTER SEQUENCE v1 RENAME TO v2 ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a sequence's comment is unset", sql: "ALTER SEQUENCE IF EXISTS v1 UNSET COMMENT ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "UNSET on a sequence names COMMENT", sql: "ALTER SEQUENCE v1 UNSET ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a version tag is created with its point and retention", sql: "ALTER ICEBERG TABLE v1 CREATE OR REPLACE VERSION_TAG IF NOT EXISTS t1 AS OF VERSION 5 RETAIN 3 DAYS ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "RETAIN counts DAYS, HOURS or MINUTES", sql: "ALTER ICEBERG TABLE v1 CREATE VERSION_TAG t1 AS OF VERSION 5 RETAIN 3 WEEKS ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "REPLACE VERSION_TAG needs AS OF", sql: "ALTER ICEBERG TABLE v1 REPLACE VERSION_TAG t1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a version tag is dropped", sql: "ALTER ICEBERG TABLE v1 DROP VERSION_TAG IF EXISTS t1 ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a partition spec is replaced", sql: "ALTER ICEBERG TABLE v1 REPLACE PARTITION BY ( a ) WITH ( b ) ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "the Iceberg version is a number", sql: "ALTER ICEBERG TABLE v1 UPGRADE TO ICEBERG VERSION v3 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an Iceberg table drops its position deletes", sql: "ALTER ICEBERG TABLE v1 OPTIMIZE ( REMOVE_POSITION_DELETES ) ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "OPTIMIZE belongs to ICEBERG TABLE", sql: "ALTER TABLE v1 OPTIMIZE ( REMOVE_POSITION_DELETES ) ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an Iceberg table keeps its other actions", sql: "ALTER ICEBERG TABLE v1 SET COMMENT = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a storage lifecycle policy is added on columns", sql: "ALTER TABLE v1 ADD STORAGE LIFECYCLE POLICY p1 ON ( a , b ) ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "the policy's column list is not empty", sql: "ALTER TABLE v1 ADD STORAGE LIFECYCLE POLICY p1 ON ( ) ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a storage lifecycle policy is dropped", sql: "ALTER TABLE v1 DROP STORAGE LIFECYCLE POLICY ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "RENAME needs TO (recorded syntax error)", sql: "ALTER TABLE v1 RENAME v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a reserved word is no new name (recorded syntax error)", sql: "ALTER TABLE v1 RENAME TO SELECT ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a quoted reserved word is a new name", sql: "ALTER TABLE v1 RENAME TO \"SELECT\" ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a column is still renamed", sql: "ALTER TABLE v1 RENAME COLUMN a TO b ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a reserved word is no swap target (recorded syntax error)", sql: "ALTER TABLE v1 SWAP WITH SELECT ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a reserved word is no table name (recorded syntax error)", sql: "ALTER TABLE SELECT SWAP WITH v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a view is renamed when it exists", sql: "ALTER VIEW IF EXISTS v1 RENAME TO v2 ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a reserved word is no view name (recorded syntax error)", sql: "ALTER VIEW SELECT RENAME TO v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "files are listed as strings", sql: "ALTER EXTERNAL TABLE v1 ADD FILES ( 'a/b.csv' , 'c.csv' ) ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an empty file list (recorded syntax error)", sql: "ALTER EXTERNAL TABLE v1 ADD FILES ( ) ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a file is no name (recorded syntax error)", sql: "ALTER EXTERNAL TABLE v1 REMOVE FILES ( m1 ) ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a share unsets its comment (recorded semantic error)", sql: "ALTER SHARE v1 UNSET COMMENT ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a share keeps its ADD action", sql: "ALTER SHARE v1 ADD ACCOUNTS = a1 ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a share unsets any named property (recorded semantic error 1420)", sql: "ALTER SHARE v1 UNSET v1 ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "UNSET names a property (catalogued reject)", sql: "ALTER SHARE v1 UNSET ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a user group is an organization user group (recorded syntax error)", sql: "ALTER USER GROUP v1 ADD ORGANIZATION USERS v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an organization user group adds users", sql: "ALTER ORGANIZATION USER GROUP IF EXISTS v1 ADD ORGANIZATION USERS v1 , v1 ;", dialect: Dialect::Snowflake, expected_accepted: true },
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
fn given_snowflake_comment_drop_and_call_statements_when_parsing_then_only_documented_forms_are_accepted(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a comment is a string (recorded syntax error)",
            sql: "COMMENT ON TABLE v1 IS m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a reserved word is no column (recorded syntax error)",
            sql: "COMMENT ON COLUMN v1 . SELECT IS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a national string is a string (sqlglot's Snowflake identity fixture)",
            sql: "COMMENT ON TABLE my_schema.my_table IS N'National String'",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a quoted reserved word is a column",
            sql: "COMMENT ON COLUMN v1 . \"SELECT\" IS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "other dialects keep their COMMENT ON values",
            sql: "COMMENT ON TABLE t1 IS NULL",
            dialect: Dialect::PostgreSQL,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a masking policy is dropped",
            sql: "DROP MASKING POLICY IF EXISTS p1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a reserved word is no role name",
            sql: "DROP ROLE SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a database is dropped with RESTRICT (recorded semantic error)",
            sql: "DROP DATABASE v1 RESTRICT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a table argument type is not empty",
            sql: "DROP FUNCTION f1 ( TABLE ( ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a table argument names its types",
            sql: "DROP FUNCTION f1 ( TABLE ( INT ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an instance method is called",
            sql: "CALL d1.s1.b1!ADD_TAG ( 'm' , ( SELECT 1 ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a quoted reserved word is an instance (recorded semantic error)",
            sql: "CALL \"SELECT\"!GET_CONFIG ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an unquoted reserved word is no instance (recorded syntax error)",
            sql: "CALL SELECT!GET_CONFIG ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a method needs its instance (recorded syntax error)",
            sql: "CALL !GET_CONFIG ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a method needs its name (recorded syntax error)",
            sql: "CALL v1! ( ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a method call needs its parentheses (recorded syntax error)",
            sql: "CALL v1!GET_CONFIG ( ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a trailing argument is missing (recorded syntax error)",
            sql: "CALL v1!ADD_TAG ( 'm' , ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a procedure call is unchanged",
            sql: "CALL p1 ( 1 ) ;",
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
fn given_snowflake_table_actions_and_method_calls_when_parsing_then_the_tree_keeps_their_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "ALTER USER without a name acts on the current user",
            sql: "ALTER USER ADD PAT t1 DAYS_TO_EXPIRY = 1",
            pointer: "/0/alter_object/action",
            expected_value: json!("ADD PROGRAMMATIC ACCESS TOKEN"),
        },
        ProjectionTestCase {
            description: "a modified credential keeps its change",
            sql: "ALTER USER u1 MODIFY KEY PAIR k1 UNSET DISABLED , COMMENT",
            pointer: "/0/alter_object/properties/1/kind",
            expected_value: json!("COMMENT"),
        },
        ProjectionTestCase {
            description: "a share's accounts keep their organization part",
            sql: "ALTER SHARE s1 ADD ACCOUNTS = o1.a1 SHARE_RESTRICTIONS = TRUE",
            pointer: "/0/alter_object/operands/0/table/schema/name",
            expected_value: json!("o1"),
        },
        ProjectionTestCase {
            description: "an Iceberg action names its kind",
            sql: "ALTER ICEBERG TABLE v1 UPGRADE TO ICEBERG VERSION 3",
            pointer: "/0/alter_object/kind",
            expected_value: json!("ICEBERG TABLE"),
        },
        ProjectionTestCase {
            description: "IF NOT EXISTS is part of the version tag action",
            sql: "ALTER ICEBERG TABLE v1 CREATE VERSION_TAG IF NOT EXISTS t1",
            pointer: "/0/alter_object/action",
            expected_value: json!("CREATE VERSION_TAG IF NOT EXISTS"),
        },
        ProjectionTestCase {
            description: "RETAIN keeps its unit",
            sql: "ALTER ICEBERG TABLE v1 ALTER VERSION_TAG t1 AS OF TIMESTAMP 1 RETAIN 2 HOURS",
            pointer: "/0/alter_object/properties/1/name/name",
            expected_value: json!("HOURS"),
        },
        ProjectionTestCase {
            description: "the policy's columns are its second operand",
            sql: "ALTER TABLE v1 ADD STORAGE LIFECYCLE POLICY p1 ON ( a , b )",
            pointer: "/0/alter_object/operands/1/tuple/expressions/1/table/name/name",
            expected_value: json!("b"),
        },
        ProjectionTestCase {
            description: "a sequence's new name is its operand",
            sql: "ALTER SEQUENCE v1 RENAME TO v2",
            pointer: "/0/alter_object/operands/0/table/name/name",
            expected_value: json!("v2"),
        },
        ProjectionTestCase {
            description: "CASCADE is kept on DROP DATABASE",
            sql: "DROP DATABASE v1 CASCADE",
            pointer: "/0/drop_database/cascade",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "the method is the call's member",
            sql: "CALL d1.s1.b1!GET_CONFIG ( )",
            pointer: "/0/call/member/name",
            expected_value: json!("GET_CONFIG"),
        },
        ProjectionTestCase {
            description: "the instance keeps its qualified name",
            sql: "CALL d1.s1.b1!GET_CONFIG ( )",
            pointer: "/0/call/name/2/name",
            expected_value: json!("b1"),
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
fn given_snowflake_policy_and_integration_alters_without_a_name_when_parsing_then_only_receipted_omissions_are_accepted(
) {
    let test_cases = [
        AcceptanceTestCase { description: "a masking policy's comment is set without a name (recorded semantic error)", sql: "ALTER MASKING POLICY SET COMMENT = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a password policy's property is unset without a name (recorded semantic error)", sql: "ALTER PASSWORD POLICY UNSET PASSWORD_MIN_LENGTH ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a storage lifecycle policy's property is unset without a name (recorded semantic error)", sql: "ALTER STORAGE LIFECYCLE POLICY UNSET ARCHIVE_FOR_DAYS ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an integration is enabled without a name (recorded semantic error)", sql: "ALTER INTEGRATION SET ENABLED = TRUE ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an integration's secret is refreshed without a name (recorded semantic error)", sql: "ALTER INTEGRATION REFRESH OAUTH_CLIENT_SECRET ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a catalog integration's authentication is set without a name (recorded semantic error)", sql: "ALTER CATALOG INTEGRATION SET REST_AUTHENTICATION = ( BEARER_TOKEN = 'm' ) ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an external access integration's rules are unset without a name (recorded semantic error)", sql: "ALTER EXTERNAL ACCESS INTEGRATION UNSET ALLOWED_NETWORK_RULES ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "the account's tags are unset by the word TAG (recorded semantic error)", sql: "ALTER ACCOUNT UNSET TAG ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "the account's contacts are unset by the word CONTACT (recorded semantic error)", sql: "ALTER ACCOUNT UNSET CONTACT ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "the account still unsets a named tag", sql: "ALTER ACCOUNT UNSET TAG v1 ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an organization user is visible to all (recorded semantic error)", sql: "ALTER ORGANIZATION USER v1 SET VISIBILITY = ALL ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "a nameless SET TAG (recorded syntax error)", sql: "ALTER MASKING POLICY SET TAG v1 = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a nameless UNSET TAG (recorded syntax error)", sql: "ALTER MASKING POLICY UNSET TAG v1 ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a nameless SET BODY (recorded syntax error)", sql: "ALTER MASKING POLICY SET BODY -> 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a nameless SET with nothing to set (recorded syntax error)", sql: "ALTER INTEGRATION SET ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a nameless REFRESH with no secret (recorded syntax error)", sql: "ALTER INTEGRATION REFRESH ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a backup policy needs its name (recorded syntax error)", sql: "ALTER BACKUP POLICY SET COMMENT = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a secret needs its name (recorded syntax error)", sql: "ALTER SECRET SET COMMENT = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a snapshot policy's UNSET needs its name (recorded syntax error)", sql: "ALTER SNAPSHOT POLICY UNSET COMMENT ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a string is no policy name (recorded syntax error)", sql: "ALTER MASKING POLICY 'v1' SET COMMENT = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "a named policy still needs SET (recorded syntax error)", sql: "ALTER MASKING POLICY v1 COMMENT = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an organization user's SET names a property (recorded syntax error)", sql: "ALTER ORGANIZATION USER SET ;", dialect: Dialect::Snowflake, expected_accepted: false },
        AcceptanceTestCase { description: "an integration named refresh keeps its name before UNSET", sql: "ALTER INTEGRATION refresh UNSET COMMENT ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an integration named refresh refreshes its secret", sql: "ALTER INTEGRATION refresh REFRESH OAUTH_CLIENT_SECRET ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "an organization user's visibility may be another value", sql: "ALTER ORGANIZATION USER v1 SET VISIBILITY = 'x' ;", dialect: Dialect::Snowflake, expected_accepted: true },
        AcceptanceTestCase { description: "VISIBILITY = ALL is followed by other properties", sql: "ALTER ORGANIZATION USER v1 SET VISIBILITY = ALL , COMMENT = 'm' ;", dialect: Dialect::Snowflake, expected_accepted: true },
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
fn given_snowflake_policy_and_integration_alters_without_a_name_when_parsing_then_the_tree_keeps_the_action(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "an omitted name is empty",
            sql: "ALTER MASKING POLICY SET COMMENT = 'm'",
            pointer: "/0/alter_object/name/name/name",
            expected_value: json!(""),
        },
        ProjectionTestCase {
            description: "the omitted name's SET keeps its property",
            sql: "ALTER MASKING POLICY SET COMMENT = 'm'",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("COMMENT"),
        },
        ProjectionTestCase {
            description: "an omitted name's REFRESH names its secret",
            sql: "ALTER INTEGRATION REFRESH OAUTH_CLIENT_SECRET",
            pointer: "/0/alter_object/action",
            expected_value: json!("REFRESH OAUTH_CLIENT_SECRET"),
        },
        ProjectionTestCase {
            description: "an integration named refresh keeps its name",
            sql: "ALTER INTEGRATION refresh UNSET COMMENT",
            pointer: "/0/alter_object/name/name/name",
            expected_value: json!("refresh"),
        },
        ProjectionTestCase {
            description: "a named policy keeps its name",
            sql: "ALTER MASKING POLICY v1 SET COMMENT = 'm'",
            pointer: "/0/alter_object/name/name/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "the bare word TAG is an unset property",
            sql: "ALTER ACCOUNT UNSET TAG",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("TAG"),
        },
        ProjectionTestCase {
            description: "an organization user's visibility is ALL",
            sql: "ALTER ORGANIZATION USER v1 SET VISIBILITY = ALL",
            pointer: "/0/alter_object/properties/0/kind",
            expected_value: json!("VISIBILITY"),
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
