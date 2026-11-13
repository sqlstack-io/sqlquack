use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_alert_create_when_parsing_then_only_what_snowflake_parses_is_accepted() {
    let test_cases = [
        AcceptanceTestCase {
            description: "an alert with its condition and action (recorded parse, then semantic error)",
            sql: "CREATE ALERT v1 IF ( EXISTS ( SELECT 1 ) ) THEN SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a defined alert with every property and a state (recorded parse)",
            sql: "DEFINE ALERT d1.s1.v1 SCHEDULE = '1 MINUTE' WAREHOUSE = v1 COMMENT = 'm' CONFIG = 'm' RUNBOOK = 'm' SUSPEND_ALERT_AFTER_NUM_FAILURES = 1 STARTED IF ( EXISTS ( SELECT 1 ) ) THEN SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an alert from a template with a tag list (recorded parse)",
            sql: "CREATE ALERT v1 FROM TEMPLATE v1 TAG ( v1 = m1 ) TEMPLATE_PARAMS = m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a clone keeps its own path (recorded parse)",
            sql: "CREATE OR REPLACE ALERT v1 CLONE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "the condition needs IF (recorded syntax error)",
            sql: "CREATE ALERT v1 ( EXISTS ( SELECT 1 ) ) THEN SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "the condition needs EXISTS (recorded syntax error)",
            sql: "CREATE OR ALTER ALERT v1 IF ( ( SELECT 1 ) ) THEN SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "EXISTS needs a condition (recorded syntax error)",
            sql: "CREATE ALERT v1 IF ( EXISTS ( ) ) THEN SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "the action needs THEN (recorded syntax error)",
            sql: "CREATE ALERT v1 IF ( EXISTS ( SELECT 1 ) ) SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a template needs FROM TEMPLATE (recorded syntax error)",
            sql: "CREATE ALERT v1 FROM v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a template takes no property list (recorded syntax error)",
            sql: "CREATE ALERT v1 FROM TEMPLATE v1 ( v1 = 'm' ) ;",
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
fn given_a_snowflake_alert_create_when_parsing_then_the_tree_keeps_its_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "the condition is the first operand",
            sql: "CREATE ALERT a1 WAREHOUSE = w1 IF ( EXISTS ( SELECT c1 FROM t1 ) ) THEN CALL p1 ( )",
            pointer: "/0/create_object/operands/0/select/from/expressions/0/table/name/name",
            expected_value: json!("t1"),
        },
        ProjectionTestCase {
            description: "a state is a property without a value",
            sql: "CREATE ALERT a1 SUSPENDED IF ( EXISTS ( SELECT 1 ) ) THEN SELECT 1",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("SUSPENDED"),
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
