use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_describe_when_parsing_then_it_needs_a_kind_and_a_name() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a two-word kind is read whole (recorded semantic error)",
            sql: "DESC COMPUTE POOL v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "half a kind is no kind (recorded errno 1003)",
            sql: "DESC POOL v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a kind needs its name (recorded errno 1003)",
            sql: "DESC FILE FORMAT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a bare name is described as an object (recorded errno 2003)",
            sql: "DESC v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a word after the name is refused, not dropped",
            sql: "DESC TABLE v1 v2 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an argument type is no string (recorded errno 1003)",
            sql: "DESC FUNCTION v1 ( 'INT' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "AS RESOURCE comes before AGENT only",
            sql: "DESC AS RESOURCE STAGE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a transaction is described by its number (recorded errno 1003 without)",
            sql: "DESC TRANSACTION v1 ;",
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
fn given_a_snowflake_describe_when_parsing_then_the_tree_keeps_its_kind_name_and_options() {
    let test_cases = [
        ProjectionTestCase {
            description: "a one-word kind keeps the name after it",
            sql: "DESC WAREHOUSE v1",
            pointer: "/0/describe/target/table/name/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "DATABASE is the kind, not the name",
            sql: "DESCRIBE DATABASE v1",
            pointer: "/0/describe/kind",
            expected_value: json!("DATABASE"),
        },
        ProjectionTestCase {
            description: "a three-word kind is named whole",
            sql: "DESC CORTEX SEARCH SERVICE v1",
            pointer: "/0/describe/kind",
            expected_value: json!("CORTEX SEARCH SERVICE"),
        },
        ProjectionTestCase {
            description: "AS RESOURCE is kept",
            sql: "DESC AS RESOURCE AGENT v1",
            pointer: "/0/describe/style",
            expected_value: json!("AS RESOURCE"),
        },
        ProjectionTestCase {
            description: "the application keeps its quoting",
            sql: "DESC CONFIGURATION v1 IN APPLICATION \"a b\".v2",
            pointer: "/0/describe/properties/0",
            expected_value: json!(["in_application", "\"a b\".v2"]),
        },
        ProjectionTestCase {
            description: "a listing revision is kept",
            sql: "DESC LISTING v1 REVISION = DRAFT",
            pointer: "/0/describe/properties/0",
            expected_value: json!(["revision", "DRAFT"]),
        },
        ProjectionTestCase {
            description: "a transaction number is the target",
            sql: "DESC TRANSACTION 12",
            pointer: "/0/describe/target/literal/value",
            expected_value: json!("12"),
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
fn given_a_snowflake_show_when_parsing_then_its_kind_and_clauses_follow_the_grammar() {
    let test_cases = [
        AcceptanceTestCase {
            description: "a shown kind in its container (recorded semantic error)",
            sql: "SHOW OFFERS IN LISTING v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a kind that needs a container refuses a bare name (recorded errno 1003)",
            sql: "SHOW OFFERS IN v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "half a kind is no kind (recorded errno 1003)",
            sql: "SHOW POOLS ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "STARTS WITH takes a string (recorded errno 1003)",
            sql: "SHOW DATABASES STARTS WITH m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "WITH PRIVILEGES needs a privilege (recorded errno 1003)",
            sql: "SHOW DATABASES WITH PRIVILEGES ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a container another kind owns is refused (recorded errno 1003)",
            sql: "SHOW CHANNELS IN LISTING v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a reserved word is no part of a dotted name (recorded errno 1003)",
            sql: "SHOW REPLICATION DATABASES WITH PRIMARY v1 . SELECT ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TABLE stands alone as a container (recorded semantic error)",
            sql: "SHOW COLUMNS IN TABLE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a kind with no plain name refuses a bare APPLICATION (recorded errno 1003)",
            sql: "SHOW PRIVILEGES IN APPLICATION ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "future grants take no plain grantee, so a bare ROLE is refused (recorded errno 1003)",
            sql: "SHOW FUTURE GRANTS TO ROLE ;",
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
fn given_a_snowflake_show_when_parsing_then_the_tree_keeps_kind_container_and_operands() {
    let test_cases = [
        ProjectionTestCase {
            description: "a multi-word kind is named whole",
            sql: "SHOW COMPUTE POOL INSTANCE FAMILIES",
            pointer: "/0/show/this",
            expected_value: json!("COMPUTE POOL INSTANCE FAMILIES"),
        },
        ProjectionTestCase {
            description: "the container kind is kept",
            sql: "SHOW ENTITIES IN DCM PROJECT v1",
            pointer: "/0/show/scope_kind",
            expected_value: json!("DCM PROJECT"),
        },
        ProjectionTestCase {
            description: "the package after an artifact repository is kept",
            sql: "SHOW VERSIONS IN ARTIFACT REPOSITORY v1 FOR PACKAGE p1",
            pointer: "/0/show/for_target/table/name/name",
            expected_value: json!("p1"),
        },
        ProjectionTestCase {
            description: "the run of an experiment is kept",
            sql: "SHOW RUN METRICS IN EXPERIMENT v1 RUN r1",
            pointer: "/0/show/target/table/name/name",
            expected_value: json!("r1"),
        },
        ProjectionTestCase {
            description: "privileges are a list",
            sql: "SHOW DATABASES WITH PRIVILEGES USAGE, MODIFY",
            pointer: "/0/show/privileges",
            expected_value: json!(["USAGE", "MODIFY"]),
        },
        ProjectionTestCase {
            description: "the primary keeps its account and database apart",
            sql: "SHOW REPLICATION DATABASES WITH PRIMARY a1.\"d 1\"",
            pointer: "/0/show/target/table/schema/name",
            expected_value: json!("a1"),
        },
        ProjectionTestCase {
            description: "the primary database keeps its quoting",
            sql: "SHOW REPLICATION DATABASES WITH PRIMARY a1.\"d 1\"",
            pointer: "/0/show/target/table/name/quoted",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "a bare APPLICATION is a schema's name, as IN v1 (recorded errno 2043 for both)",
            sql: "SHOW TABLES IN APPLICATION ;",
            pointer: "/0/show/scope/table/name/name",
            expected_value: json!("APPLICATION"),
        },
        ProjectionTestCase {
            description: "a bare APPLICATION names no container kind",
            sql: "SHOW TABLES IN APPLICATION ;",
            pointer: "/0/show/scope_kind",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "a bare CLASS is a schema's name (recorded errno 2003, as IN v1, not 398014 as IN CLASS v1)",
            sql: "SHOW FUNCTIONS IN CLASS ;",
            pointer: "/0/show/scope/table/name/name",
            expected_value: json!("CLASS"),
        },
        ProjectionTestCase {
            description: "a bare grantee kind is the grantee's name (recorded errno 3044, as TO v1)",
            sql: "SHOW GRANTS TO ROLE ;",
            pointer: "/0/show/scope/table/name/name",
            expected_value: json!("ROLE"),
        },
        ProjectionTestCase {
            description: "the word ending a grantee kind is the name after the shorter kind",
            sql: "SHOW GRANTS TO APPLICATION ROLE ;",
            pointer: "/0/show/scope_kind",
            expected_value: json!("TO APPLICATION"),
        },
        ProjectionTestCase {
            description: "a caller grant's bare ROLE is the owner's name",
            sql: "SHOW CALLER GRANTS TO ROLE ;",
            pointer: "/0/show/scope/table/name/name",
            expected_value: json!("ROLE"),
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
