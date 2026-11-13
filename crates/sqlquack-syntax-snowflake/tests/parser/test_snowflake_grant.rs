use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_grant_or_revoke_when_parsing_then_only_what_snowflake_parses_is_accepted() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a schema privilege grants to a database role (recorded success)",
            sql: "GRANT CREATE SCHEMA ON DATABASE v1 TO DATABASE ROLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a SNOWFLAKE class privilege may leave its schema empty (recorded success)",
            sql: "GRANT CREATE SNOWFLAKE . . BUDGET ON SCHEMA v1 TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an AI function privilege names its function (recorded success)",
            sql: "GRANT USE AI FUNCTION \"SELECT\" ON ACCOUNT TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a kind before TO is the object's name (recorded success)",
            sql: "GRANT ADD SEARCH OPTIMIZATION ON SCHEMA TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a reserved kind before TO is no name (recorded errno 1003)",
            sql: "GRANT MODIFY ON TABLE TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a privilege grantee may be a role named APPLICATION (recorded success)",
            sql: "GRANT CREATE COMPUTE POOL ON ACCOUNT TO APPLICATION ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a role grant needs a grantee name (recorded errno 1003)",
            sql: "GRANT DATABASE ROLE v1 TO APPLICATION ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a database role grant reads DATABASE ROLE ; as a database named ROLE (recorded success)",
            sql: "GRANT DATABASE ROLE v1 TO DATABASE ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a role grant takes no bare grantee keyword (recorded errno 1003)",
            sql: "GRANT ROLE v1 TO ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ownership takes any object kind word (recorded success)",
            sql: "GRANT OWNERSHIP ON v1 v1 TO ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an unfinished multi-word kind is refused (recorded errno 1003)",
            sql: "GRANT MODIFY ON EXTERNAL v1 TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an undocumented privilege is refused (recorded errno 1003)",
            sql: "GRANT MODIFY LOG ON ACCOUNT TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "IN ACCOUNT takes an inherited privilege (recorded success)",
            sql: "GRANT INHERITED CALLER USAGE , USAGE ON ALL TABLES IN ACCOUNT TO ROLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "IN ACCOUNT refuses a privilege that is not inherited (recorded errno 1003)",
            sql: "GRANT CALLER USAGE ON ALL TABLES IN ACCOUNT TO ROLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "APPLICATION PACKAGE before TO is an application named PACKAGE (recorded success)",
            sql: "GRANT ALL INHERITED CALLER PRIVILEGES ON ALL TABLES IN APPLICATION PACKAGE TO ROLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ownership transfer names CURRENT GRANTS in full (recorded errno 1003)",
            sql: "GRANT OWNERSHIP ON TABLE v1 TO ROLE v1 REVOKE CURRENT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a service role revoke from DATABASE ROLE ; (recorded success)",
            sql: "REVOKE SERVICE ROLE v1!v1 FROM DATABASE ROLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a service role is no string (recorded errno 1003)",
            sql: "REVOKE SERVICE ROLE v1!'v1' FROM ROLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a role grant option revoke restricts (recorded success)",
            sql: "REVOKE GRANT OPTION FOR ROLE v1 FROM ROLE v1 RESTRICT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "the single-word create and import privileges Snowflake reads (recorded semantic errors)",
            sql: "GRANT CREATE VOLUME , CREATE POLICY , CREATE REPLICATION , IMPORT LISTING , IMPORT ORGANIZATION ON ACCOUNT TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a word of a longer create privilege is no privilege (recorded errno 1003)",
            sql: "GRANT CREATE POOL ON ACCOUNT TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a data exchange is a kind (recorded semantic error)",
            sql: "REVOKE ATTACH LISTING ON DATA EXCHANGE v1 FROM v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a one-word kind that only ends a longer kind is refused (recorded errno 1003)",
            sql: "GRANT MODIFY ON POOL v1 TO APPLICATION v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an unlisted one-word kind stays accepted (recorded semantic error)",
            sql: "GRANT MODIFY ON VOLUME v1 TO v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "ALL needs a plural (recorded errno 1003)",
            sql: "GRANT REFERENCE_USAGE ON ALL ICEBERG IN SCHEMA v1 TO SHARE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a database role grants to a database (recorded semantic error)",
            sql: "GRANT DATABASE ROLE v1 TO DATABASE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a service role grants to an application (recorded semantic error)",
            sql: "GRANT SERVICE ROLE v1!v1 TO APPLICATION v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SHOW GRANTS OF a class instance role (recorded semantic error)",
            sql: "SHOW GRANTS OF ROLE v1!v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "SHOW GRANTS TO a bare name (recorded semantic error)",
            sql: "SHOW GRANTS TO v1 ;",
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
fn given_a_snowflake_grant_or_revoke_when_parsing_then_the_tree_keeps_its_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "a role grant names the role's kind",
            sql: "GRANT DATABASE ROLE d1.r1 TO ROLE r2",
            pointer: "/0/grant/kind",
            expected_value: json!("DATABASE ROLE"),
        },
        ProjectionTestCase {
            description: "a privilege grantee keeps its kind",
            sql: "GRANT USAGE ON SCHEMA s1 TO APPLICATION ROLE r1",
            pointer: "/0/grant/principals/0/kind",
            expected_value: json!("APPLICATION ROLE"),
        },
        ProjectionTestCase {
            description: "a bare grantee before RESTRICT is the name, not the USER kind",
            sql: "REVOKE INSERT ON TABLE t1 FROM user RESTRICT",
            pointer: "/0/revoke/principals/0/name/name",
            expected_value: json!("user"),
        },
        ProjectionTestCase {
            description: "a future grant keeps its plural and container",
            sql: "GRANT SELECT ON FUTURE TABLES IN SCHEMA s1 TO ROLE r1",
            pointer: "/0/grant/kind",
            expected_value: json!("FUTURE TABLES IN SCHEMA"),
        },
        ProjectionTestCase {
            description: "a multi-word privilege is one privilege",
            sql: "REVOKE ADD SEARCH OPTIMIZATION ON SCHEMA s1 FROM ROLE r1",
            pointer: "/0/revoke/privileges/0/name",
            expected_value: json!("ADD SEARCH OPTIMIZATION"),
        },
        ProjectionTestCase {
            description: "a SNOWFLAKE class privilege keeps its class",
            sql: "GRANT CREATE SNOWFLAKE.ML.FORECAST ON SCHEMA s1 TO ROLE r1",
            pointer: "/0/grant/privileges/0/name",
            expected_value: json!("CREATE SNOWFLAKE.ML.FORECAST"),
        },
        ProjectionTestCase {
            description: "CALLER qualifies every privilege of the list",
            sql: "GRANT CALLER USAGE , SELECT ON TABLE t1 TO ROLE r1",
            pointer: "/0/grant/privileges/1/name",
            expected_value: json!("CALLER SELECT"),
        },
        ProjectionTestCase {
            description: "INHERITED CALLER qualifies every privilege of a revoked list",
            sql: "REVOKE INHERITED CALLER USAGE , SELECT ON ALL TABLES IN ACCOUNT FROM ROLE r1",
            pointer: "/0/revoke/privileges/1/name",
            expected_value: json!("INHERITED CALLER SELECT"),
        },
        ProjectionTestCase {
            description: "a service role keeps the quoting of each part",
            sql: "GRANT SERVICE ROLE \"s 1\"!r1 TO ROLE r2",
            pointer: "/0/grant/securable/name",
            expected_value: json!("\"s 1\"!r1"),
        },
        ProjectionTestCase {
            description: "a quoted class name keeps its quotes and case",
            sql: "GRANT OWNERSHIP ON \"c 1\" i1 TO ROLE r1",
            pointer: "/0/grant/kind",
            expected_value: json!("\"c 1\""),
        },
        ProjectionTestCase {
            description: "ownership keeps COPY CURRENT GRANTS",
            sql: "GRANT OWNERSHIP ON TABLE t1 TO ROLE r1 COPY CURRENT GRANTS",
            pointer: "/0/grant/current_grants",
            expected_value: json!("COPY"),
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
