use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_backup_snapshot_set_or_application_create_when_parsing_then_only_what_snowflake_parses_is_accepted(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a backup set for a table (recorded parse, then semantic error)",
            sql: "CREATE BACKUP SET v1 FOR TABLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a backup set with every option (recorded parse)",
            sql: "CREATE OR REPLACE BACKUP SET IF NOT EXISTS v1 FOR DYNAMIC TABLE v1 WITH TAG ( v1 = 'm' , v1 = 'm' ) WITH BACKUP POLICY v1 COMMENT = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a set's tag value and comment may be names (recorded parse)",
            sql: "CREATE SNAPSHOT SET v1 FOR SCHEMA v1 TAG ( v1 = m1 ) COMMENT = m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a set's target may end in an unreserved operand word",
            sql: "CREATE SNAPSHOT SET v1 FOR TABLE s1.policy ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a set needs FOR (recorded syntax error)",
            sql: "CREATE BACKUP SET v1 DATABASE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a set is for a database, schema or table (recorded syntax error)",
            sql: "CREATE BACKUP SET v1 FOR v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a backup set's policy is a BACKUP POLICY (recorded syntax error)",
            sql: "CREATE BACKUP SET v1 FOR TABLE v1 WITH POLICY v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a set takes no property list after its target (recorded syntax error)",
            sql: "CREATE SNAPSHOT SET v1 FOR DATABASE v1 ( v1 = 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "SET alone is no kind (recorded syntax error)",
            sql: "CREATE OR ALTER SET v1 FOR TABLE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a snapshot set needs SET (recorded syntax error)",
            sql: "CREATE SNAPSHOT v1 FOR DATABASE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a snapshot of a service is still a snapshot",
            sql: "CREATE SNAPSHOT s1 FROM SERVICE svc VOLUME \"v\" INSTANCE 0 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an application from a package version and patch (recorded parse)",
            sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE v1 USING VERSION v1 PATCH 1 DEBUG_MODE = TRUE COMMENT = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an application from a listing on a release channel (recorded parse)",
            sql: "CREATE APPLICATION v1 FROM LISTING v1 USING RELEASE CHANNEL DEFAULT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an application's listing may be a string (recorded parse)",
            sql: "CREATE APPLICATION v1 FROM LISTING 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "WITH before a one-word property (recorded parse)",
            sql: "CREATE APPLICATION v1 FROM LISTING v1 WITH FEATURE = v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "WITH FEATURE POLICY is refused (recorded syntax error)",
            sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE v1 WITH FEATURE POLICY = v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an application needs FROM (recorded syntax error)",
            sql: "CREATE APPLICATION v1 APPLICATION PACKAGE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a package name is no string (recorded syntax error)",
            sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "USING needs VERSION, RELEASE CHANNEL or a path (recorded syntax error)",
            sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE v1 USING v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "PATCH needs its number (recorded syntax error)",
            sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE v1 USING VERSION v1 PATCH ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a version directory comes after USING (recorded syntax error)",
            sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE v1 '@d1.s1.st1/p1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an application role is still a role",
            sql: "CREATE APPLICATION ROLE r1 ;",
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
fn given_a_snowflake_backup_snapshot_set_or_application_create_when_parsing_then_the_tree_keeps_its_parts(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "a set keeps its kind",
            sql: "CREATE SNAPSHOT SET v1 FOR SCHEMA d1.s1",
            pointer: "/0/create_object/kind",
            expected_value: json!("SNAPSHOT SET"),
        },
        ProjectionTestCase {
            description: "a set keeps what it is for",
            sql: "CREATE BACKUP SET v1 FOR DYNAMIC TABLE d1.s1.t1",
            pointer: "/0/create_object/source",
            expected_value: json!("FOR DYNAMIC TABLE"),
        },
        ProjectionTestCase {
            description: "a set's target is its operand",
            sql: "CREATE BACKUP SET v1 FOR DYNAMIC TABLE d1.s1.t1",
            pointer: "/0/create_object/operands/0/table/schema/name",
            expected_value: json!("s1"),
        },
        ProjectionTestCase {
            description: "a set's policy is a property named for its kind",
            sql: "CREATE BACKUP SET v1 FOR TABLE t1 WITH TAG ( t1 = 'm' ) WITH BACKUP POLICY p1 COMMENT = 'm'",
            pointer: "/0/create_object/properties/1/kind",
            expected_value: json!("WITH BACKUP POLICY"),
        },
        ProjectionTestCase {
            description: "an application keeps its source",
            sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE p1 USING VERSION v2 PATCH 3",
            pointer: "/0/create_object/source",
            expected_value: json!("FROM APPLICATION PACKAGE"),
        },
        ProjectionTestCase {
            description: "an application's patch follows its version",
            sql: "CREATE APPLICATION v1 FROM APPLICATION PACKAGE p1 USING VERSION v2 PATCH 3",
            pointer: "/0/create_object/properties/1/value/literal/value",
            expected_value: json!("3"),
        },
        ProjectionTestCase {
            description: "an application's listing string stays a string",
            sql: "CREATE APPLICATION v1 FROM LISTING 'ORGDATA.L1' USING RELEASE CHANNEL DEFAULT",
            pointer: "/0/create_object/operands/0/literal/value",
            expected_value: json!("ORGDATA.L1"),
        },
        ProjectionTestCase {
            description: "a release channel keeps its name",
            sql: "CREATE APPLICATION v1 FROM LISTING 'ORGDATA.L1' USING RELEASE CHANNEL DEFAULT",
            pointer: "/0/create_object/properties/0/value/identifier/name",
            expected_value: json!("DEFAULT"),
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
