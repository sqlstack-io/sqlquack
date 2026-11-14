use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{snowflake_kind, statements};
use crate::test_types::{ProjectionTestCase, SnowflakeVerdictTestCase};

#[test]
fn given_a_snowflake_create_openflow_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a connector from a stage location (recorded parse)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN RUNTIME v1 FROM 'm' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a connector from a definition with its options (recorded parse)",
            sql: "CREATE OPENFLOW CONNECTOR IF NOT EXISTS v1 IN RUNTIME v1 FROM DEFINITION v1 DISPLAY_NAME = 'm' COMMENT = 'm' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a connector's runtime is quoted (recorded parse)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN RUNTIME \"v1 Q\" FROM DEFINITION v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a connector needs IN RUNTIME (recorded errno 1003)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN v1 FROM 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a connector needs IN before RUNTIME (recorded errno 1003)",
            sql: "CREATE OPENFLOW CONNECTOR v1 RUNTIME v1 FROM DEFINITION v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a connector's runtime is a name, not a string (recorded errno 1003)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN RUNTIME 'v1' FROM 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a connector's runtime is no reserved word (recorded errno 1003)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN RUNTIME SELECT FROM 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a connector's location is a string (recorded errno 1003)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN RUNTIME v1 FROM m1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a connector needs FROM (recorded errno 1003)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN RUNTIME v1 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "DEFINITION needs FROM (recorded errno 1003)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN RUNTIME v1 DEFINITION v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a connector definition needs its id (recorded errno 1003)",
            sql: "CREATE OPENFLOW CONNECTOR v1 IN RUNTIME v1 FROM DEFINITION ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a deployment by name alone (recorded parse)",
            sql: "CREATE OPENFLOW DEPLOYMENT v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a deployment with every option (recorded parse)",
            sql: "CREATE OPENFLOW DEPLOYMENT IF NOT EXISTS v1 DEPLOYMENT_TYPE = SNOWFLAKE VPC_TYPE = 'MANAGED' USE_PRIVATE_LINK = TRUE USE_USER_AUTH_OVER_PRIVATE_LINK = TRUE CUSTOM_INGRESS_HOSTNAME = 'm' DISPLAY_NAME = 'm' COMMENT = 'm' EVENT_TABLE = 'd1.s1.t1' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a deployment's event table may be NONE (recorded parse)",
            sql: "CREATE OPENFLOW DEPLOYMENT v1 EVENT_TABLE = NONE ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a deployment option needs a value (recorded errno 1003)",
            sql: "CREATE OPENFLOW DEPLOYMENT v1 DEPLOYMENT_TYPE = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a deployment name is no string (recorded errno 1003)",
            sql: "CREATE OPENFLOW DEPLOYMENT 'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a runtime with its required options (recorded parse)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a runtime with every option (recorded parse)",
            sql: "CREATE OPENFLOW RUNTIME IF NOT EXISTS v1 . v1 . v1 IN DEPLOYMENT v1 NODE_TYPE = SMALL NODE_TYPE_TIER = 'S1' MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 EXTERNAL_ACCESS_INTEGRATIONS = ( v1 , v1 ) DISPLAY_NAME = 'm' COMMENT = 'm' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a runtime's role may be a string (recorded parse)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = 'v1' ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a runtime's integration list may be empty (recorded parse)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 EXTERNAL_ACCESS_INTEGRATIONS = ( ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a runtime's integration list ends in a comma (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 EXTERNAL_ACCESS_INTEGRATIONS = ( v1 , ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a runtime's integration is no reserved word (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 EXTERNAL_ACCESS_INTEGRATIONS = ( SELECT ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a runtime's integrations need parentheses (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 EXTERNAL_ACCESS_INTEGRATIONS = v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a runtime needs IN DEPLOYMENT (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 DEPLOYMENT v1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a runtime's deployment is a name (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT 'v1' NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a runtime's deployment is required (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an option needs its name (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a node type needs a value (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 NODE_TYPE = MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a role is no reserved word (recorded errno 1003)",
            sql: "CREATE OPENFLOW RUNTIME v1 IN DEPLOYMENT v1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 1 EXECUTE_AS_ROLE = SELECT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "OPENFLOW needs its kind (recorded errno 1003)",
            sql: "CREATE OPENFLOW v1 IN RUNTIME v1 FROM 'm' ;",
            expected_kind: None,
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            snowflake_kind(test_case.sql).as_deref(),
            test_case.expected_kind,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_snowflake_create_openflow_when_parsing_then_the_tree_keeps_its_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "a connector keeps its runtime",
            sql: "CREATE OPENFLOW CONNECTOR c1 IN RUNTIME r1 FROM DEFINITION d1",
            pointer: "/0/create_object/operands/0/table/name/name",
            expected_value: json!("r1"),
        },
        ProjectionTestCase {
            description: "a connector keeps its definition",
            sql: "CREATE OPENFLOW CONNECTOR c1 IN RUNTIME r1 FROM DEFINITION d1",
            pointer: "/0/create_object/operands/1/table/name/name",
            expected_value: json!("d1"),
        },
        ProjectionTestCase {
            description: "a connector names its source clause",
            sql: "CREATE OPENFLOW CONNECTOR c1 IN RUNTIME r1 FROM DEFINITION d1",
            pointer: "/0/create_object/source",
            expected_value: json!("FROM DEFINITION"),
        },
        ProjectionTestCase {
            description: "a connector keeps its stage location as a string",
            sql: "CREATE OPENFLOW CONNECTOR c1 IN RUNTIME r1 FROM '@s1/p1'",
            pointer: "/0/create_object/operands/1/literal/value",
            expected_value: json!("@s1/p1"),
        },
        ProjectionTestCase {
            description: "a runtime keeps its deployment",
            sql: "CREATE OPENFLOW RUNTIME r1 IN DEPLOYMENT p1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 2 EXECUTE_AS_ROLE = o1",
            pointer: "/0/create_object/operands/0/table/name/name",
            expected_value: json!("p1"),
        },
        ProjectionTestCase {
            description: "a node type is a keyword, not a name",
            sql: "CREATE OPENFLOW RUNTIME r1 IN DEPLOYMENT p1 NODE_TYPE = small MIN_NODES = 1 MAX_NODES = 2 EXECUTE_AS_ROLE = o1",
            pointer: "/0/create_object/properties/0/value/var/this",
            expected_value: json!("SMALL"),
        },
        ProjectionTestCase {
            description: "a runtime keeps its role as a name",
            sql: "CREATE OPENFLOW RUNTIME r1 IN DEPLOYMENT p1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 2 EXECUTE_AS_ROLE = o1",
            pointer: "/0/create_object/properties/3/value/table/name/name",
            expected_value: json!("o1"),
        },
        ProjectionTestCase {
            description: "a runtime's integrations stay one list",
            sql: "CREATE OPENFLOW RUNTIME r1 IN DEPLOYMENT p1 NODE_TYPE = SMALL MIN_NODES = 1 MAX_NODES = 2 EXECUTE_AS_ROLE = o1 EXTERNAL_ACCESS_INTEGRATIONS = ( i1 , i2 )",
            pointer: "/0/create_object/properties/4/value/tuple/expressions/1/table/name/name",
            expected_value: json!("i2"),
        },
        ProjectionTestCase {
            description: "a deployment type is a keyword",
            sql: "CREATE OPENFLOW DEPLOYMENT p1 DEPLOYMENT_TYPE = byoc",
            pointer: "/0/create_object/properties/0/value/var/this",
            expected_value: json!("BYOC"),
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
