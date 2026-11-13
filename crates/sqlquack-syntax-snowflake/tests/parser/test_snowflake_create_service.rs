use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_service_or_application_service_create_when_parsing_then_only_what_snowflake_parses_is_accepted(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a service from a specification file (recorded parse, then semantic error)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 FROM SPECIFICATION_FILE = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a staged template file with USING arguments (recorded parse)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 FROM @v1 SPECIFICATION_TEMPLATE_FILE = 'm' USING ( v1 => 1 , v1 => 1 ) AUTO_RESUME = FALSE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "inline specification text, with or without SPECIFICATION (recorded parse)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 FROM $$m: 1$$ ; CREATE SERVICE v1 IN COMPUTE POOL v1 FROM SPECIFICATION $$m: 1$$ ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a tag list may end the options (recorded parse)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 FROM SPECIFICATION_FILE = 'm' WITH TAG ( v1 = 'm' ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "nothing follows the tag list (recorded syntax error)",
            sql: "CREATE SERVICE IF NOT EXISTS v1 IN COMPUTE POOL v1 FROM SPECIFICATION_FILE = 'm' WITH TAG ( v1 = 'm' ) COMMENT = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a service needs IN COMPUTE POOL (recorded syntax error)",
            sql: "CREATE SERVICE v1 IN POOL v1 FROM SPECIFICATION_FILE = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a pool name is no string (recorded syntax error)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL 'v1' FROM SPECIFICATION_FILE = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a service needs FROM and a specification (recorded syntax error)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 SPECIFICATION_FILE = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a stage needs a specification file key (recorded syntax error)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 FROM @v1 = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "plain specification text takes no USING (recorded syntax error)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 FROM $$m: 1$$ USING ( v1 => 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a template needs USING before its arguments (recorded syntax error)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 FROM SPECIFICATION_TEMPLATE_FILE = 'm' ( v1 => 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "each USING argument has a value (recorded syntax error)",
            sql: "CREATE SERVICE v1 IN COMPUTE POOL v1 FROM SPECIFICATION_TEMPLATE_FILE = 'm' USING ( v1 => ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an application service from a package version (recorded parse)",
            sql: "CREATE OR ALTER APPLICATION SERVICE v1 FROM ARTIFACT REPOSITORY v1 PACKAGE v1 VERSION v1 COMMENT = 'm' SPECIFICATION = 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an application service needs FROM (recorded syntax error)",
            sql: "CREATE APPLICATION SERVICE v1 ARTIFACT REPOSITORY v1 PACKAGE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a repository name is no string (recorded syntax error)",
            sql: "CREATE APPLICATION SERVICE v1 FROM ARTIFACT REPOSITORY 'v1' PACKAGE v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "an application service needs PACKAGE (recorded syntax error)",
            sql: "CREATE APPLICATION SERVICE v1 FROM ARTIFACT REPOSITORY v1 v1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a version alias is no string (recorded syntax error)",
            sql: "CREATE APPLICATION SERVICE v1 FROM ARTIFACT REPOSITORY v1 PACKAGE v1 VERSION 'v1' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a service from a repository is an APPLICATION SERVICE (recorded syntax error)",
            sql: "CREATE SERVICE v1 FROM ARTIFACT REPOSITORY v1 PACKAGE v1 ;",
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
fn given_a_snowflake_service_or_application_service_create_when_parsing_then_the_tree_keeps_its_parts(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "a service's pool is its operand",
            sql: "CREATE SERVICE s1 IN COMPUTE POOL p1 FROM SPECIFICATION_FILE = 'spec.yaml'",
            pointer: "/0/create_object/operands/0/table/name/name",
            expected_value: json!("p1"),
        },
        ProjectionTestCase {
            description: "a template's arguments follow its file",
            sql: "CREATE SERVICE s1 IN COMPUTE POOL p1 FROM @st1 SPECIFICATION_TEMPLATE_FILE = 'spec.yaml' USING ( k1 => 1 )",
            pointer: "/0/create_object/properties/2/kind",
            expected_value: json!("USING"),
        },
        ProjectionTestCase {
            description: "an application service keeps its kind",
            sql: "CREATE APPLICATION SERVICE s1 FROM ARTIFACT REPOSITORY r1 PACKAGE k1 VERSION v2",
            pointer: "/0/create_object/kind",
            expected_value: json!("APPLICATION SERVICE"),
        },
        ProjectionTestCase {
            description: "an application service keeps its version",
            sql: "CREATE APPLICATION SERVICE s1 FROM ARTIFACT REPOSITORY r1 PACKAGE k1 VERSION v2",
            pointer: "/0/create_object/properties/1/value/identifier/name",
            expected_value: json!("v2"),
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
