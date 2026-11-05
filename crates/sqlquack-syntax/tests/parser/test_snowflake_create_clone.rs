use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_clone_or_replica_when_parsing_then_only_documented_forms_are_accepted() {
    let test_cases = [
        AcceptanceTestCase {
            description: "an alert clones (recorded success)",
            sql: "CREATE OR REPLACE ALERT v1 CLONE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a database role clones within its database (recorded success)",
            sql: "CREATE DATABASE ROLE IF NOT EXISTS v1 . v1 CLONE v1 . v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a dynamic table clone takes its lag and warehouse (recorded success)",
            sql: "CREATE OR REPLACE DYNAMIC TABLE v1 CLONE v1 AT ( OFFSET => -60 ) COPY GRANTS TARGET_LAG = DOWNSTREAM WAREHOUSE = v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a database clone takes its IGNORE options (recorded success)",
            sql: "CREATE DATABASE v1 CLONE v1 AT ( TIMESTAMP => CURRENT_TIMESTAMP ( ) ) IGNORE TABLES WITH INSUFFICIENT DATA RETENTION IGNORE HYBRID TABLES ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a role has no CLONE (recorded errno 1003)",
            sql: "CREATE ROLE v1 CLONE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a clone needs its source (recorded errno 1003)",
            sql: "CREATE DYNAMIC TABLE v1 CLONE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a time travel clause needs its kind's value (recorded errno 1003)",
            sql: "CREATE EVENT TABLE v1 CLONE v1 AT ( TIMESTAMP => ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a time travel clause is parenthesized (recorded errno 1003)",
            sql: "CREATE EVENT TABLE v1 CLONE v1 AT TIMESTAMP => CURRENT_TIMESTAMP ( ) ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a database clone's IGNORE TABLES needs WITH (recorded errno 1003)",
            sql: "CREATE DATABASE v1 CLONE v1 IGNORE TABLES INSUFFICIENT DATA RETENTION ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "COPY needs GRANTS or TAGS (recorded errno 1003)",
            sql: "CREATE STREAM v1 CLONE v1 COPY ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a clone takes no IF EXISTS (recorded errno 1003)",
            sql: "CREATE TABLE IF EXISTS v1 CLONE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a clone's name is no reserved keyword (recorded errno 1003)",
            sql: "CREATE STAGE SELECT CLONE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a connection replica names org, account and connection",
            sql: "CREATE CONNECTION IF NOT EXISTS v1 AS REPLICA OF v1 . v1 . v1 COMMENT = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a replication group replica names org, account and group",
            sql: "CREATE REPLICATION GROUP v1 AS REPLICA OF v1 . \"v1 Q\" . v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a replica's source has all three parts",
            sql: "CREATE FAILOVER GROUP v1 AS REPLICA OF v1 . . v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a replica needs AS",
            sql: "CREATE FAILOVER GROUP v1 REPLICA OF v1 . v1 . v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a replica needs REPLICA",
            sql: "CREATE CONNECTION v1 AS OF v1 . v1 . v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a group replica takes no COMMENT",
            sql: "CREATE REPLICATION GROUP v1 AS REPLICA OF v1 . v1 . v1 COMMENT = 'm' ;",
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
fn given_a_snowflake_clone_or_replica_when_parsing_then_the_tree_keeps_its_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "a database role clone names its kind",
            sql: "CREATE DATABASE ROLE d1.r1 CLONE d1.r2",
            pointer: "/0/create_object/kind",
            expected_value: json!("DATABASE ROLE"),
        },
        ProjectionTestCase {
            description: "a dynamic table clone keeps its lag",
            sql: "CREATE DYNAMIC TABLE t1 CLONE t2 TARGET_LAG = DOWNSTREAM",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("TARGET_LAG"),
        },
        ProjectionTestCase {
            description: "an event table clone keeps its time travel kind",
            sql: "CREATE EVENT TABLE e1 CLONE e2 AT ( OFFSET => -60 )",
            pointer: "/0/create_object/operands/1/historical_data/kind",
            expected_value: json!("OFFSET"),
        },
        ProjectionTestCase {
            description: "a table clone's time travel is typed, not raw",
            sql: "CREATE TABLE t1 CLONE t2 BEFORE ( STATEMENT => 'q1' )",
            pointer: "/0/create_table/clone_at_clause/historical_data/expression/literal/value",
            expected_value: json!("q1"),
        },
        ProjectionTestCase {
            description: "a database clone's time travel is typed, not raw",
            sql: "CREATE DATABASE d1 CLONE d2 AT ( OFFSET => -60 )",
            pointer: "/0/create_database/at_clause/historical_data/this/identifier/name",
            expected_value: json!("AT"),
        },
        ProjectionTestCase {
            description: "a schema clone keeps its INCLUDE option",
            sql: "CREATE SCHEMA s1 CLONE s2 INCLUDE INTERNAL STAGES",
            pointer: "/0/create_schema/properties/0/property/this/identifier/name",
            expected_value: json!("INCLUDE INTERNAL STAGES"),
        },
        ProjectionTestCase {
            description: "a connection replica keeps its organization",
            sql: "CREATE CONNECTION c1 AS REPLICA OF o1.a1.c2",
            pointer: "/0/create_object/operands/0/table/catalog/name",
            expected_value: json!("o1"),
        },
        ProjectionTestCase {
            description: "a failover group replica names its source",
            sql: "CREATE FAILOVER GROUP g1 AS REPLICA OF o1.a1.g2",
            pointer: "/0/create_object/source",
            expected_value: json!("REPLICA"),
        },
        ProjectionTestCase {
            description: "a database clone keeps OR REPLACE",
            sql: "CREATE OR REPLACE DATABASE d1 CLONE d2",
            pointer: "/0/create_database/or_replace",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a database clone keeps its name's quoting",
            sql: "CREATE DATABASE \"v1 Q\" CLONE d2",
            pointer: "/0/create_database/name/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a schema clone keeps OR REPLACE",
            sql: "CREATE OR REPLACE SCHEMA s1 CLONE s2",
            pointer: "/0/create_schema/or_replace",
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
