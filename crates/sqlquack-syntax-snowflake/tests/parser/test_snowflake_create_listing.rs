use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_external_or_organization_listing_create_when_parsing_then_only_what_snowflake_parses_is_accepted(
) {
    let test_cases = [
        AcceptanceTestCase {
            description:
                "an external listing from a manifest location (recorded parse, then semantic error)",
            sql: "CREATE EXTERNAL LISTING v1 FROM 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an external listing for a package, with options (recorded parse)",
            sql: "CREATE EXTERNAL LISTING v1 APPLICATION PACKAGE \"v1 Q\" AS 'm' REVIEW = FALSE ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "an organization listing for a share (recorded parse)",
            sql: "CREATE ORGANIZATION LISTING v1 SHARE v1 FROM 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a listing needs AS or FROM (recorded syntax error)",
            sql: "CREATE EXTERNAL LISTING v1 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a manifest is no name (recorded syntax error)",
            sql: "CREATE ORGANIZATION LISTING v1 AS m1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a share needs its name (recorded syntax error)",
            sql: "CREATE EXTERNAL LISTING v1 SHARE FROM 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a share name is no string (recorded syntax error)",
            sql: "CREATE ORGANIZATION LISTING v1 SHARE 'v1' AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a package needs APPLICATION PACKAGE (recorded syntax error)",
            sql: "CREATE EXTERNAL LISTING v1 APPLICATION v1 AS 'm' ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a comment needs its value (recorded syntax error)",
            sql: "CREATE EXTERNAL LISTING v1 AS 'm' COMMENT = ;",
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
fn given_a_snowflake_external_or_organization_listing_create_when_parsing_then_the_tree_keeps_its_parts(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "a listing keeps its kind",
            sql: "CREATE ORGANIZATION LISTING l1 SHARE s1 AS $$title: t$$",
            pointer: "/0/create_object/kind",
            expected_value: json!("ORGANIZATION LISTING"),
        },
        ProjectionTestCase {
            description: "a listing keeps its share",
            sql: "CREATE ORGANIZATION LISTING l1 SHARE s1 AS $$title: t$$",
            pointer: "/0/create_object/properties/0/value/table/name/name",
            expected_value: json!("s1"),
        },
        ProjectionTestCase {
            description: "a listing keeps how its manifest is given",
            sql: "CREATE EXTERNAL LISTING l1 FROM '@st1/dir' PUBLISH = FALSE",
            pointer: "/0/create_object/source",
            expected_value: json!("FROM"),
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
