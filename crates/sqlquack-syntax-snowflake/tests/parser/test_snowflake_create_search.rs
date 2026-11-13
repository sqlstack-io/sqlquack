use serde_json::json;

use sqlquack_syntax::{CallMode, Dialect};

use crate::helpers::{accepted, statements};
use crate::test_types::{AcceptanceTestCase, ProjectionTestCase};

#[test]
fn given_a_snowflake_cortex_search_service_create_when_parsing_then_only_what_snowflake_parses_is_accepted(
) {
    let test_cases = [
        AcceptanceTestCase {
            description: "a service on one search column (recorded parse, then semantic error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON v1 ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a service with every option (recorded parse)",
            sql: "CREATE OR REPLACE CORTEX SEARCH SERVICE IF NOT EXISTS v1 ON v1 PRIMARY KEY ( v1 , v1 ) ATTRIBUTES v1 , v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' EMBEDDING_MODEL = v1 REFRESH_MODE = FULL INITIALIZE = ON_CREATE FULL_INDEX_BUILD_INTERVAL_DAYS = 1 REQUEST_LOGGING = TRUE AUTO_SUSPEND = 1 COMMENT = 'm' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a service with text and vector indexes (recorded parse)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 TEXT INDEXES v1 , v1 VECTOR INDEXES v1 , v1 ATTRIBUTES v1 WAREHOUSE = 'v1' TARGET_LAG = '1 minute' AS SELECT * FROM ( SELECT 1 ) ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a quoted reserved word is a column (recorded parse)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON \"SELECT\" ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: true,
        },
        AcceptanceTestCase {
            description: "a search column is no string (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON 'v1' ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ON needs its column (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a service needs ON or TEXT INDEXES (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 v1 ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "TEXT needs INDEXES (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 TEXT v1 VECTOR INDEXES v1 ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "text indexes need vector indexes (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 TEXT INDEXES v1 INDEXES v1 ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a primary key lists at least one column (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON v1 PRIMARY KEY ( ) ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "ATTRIBUTES needs a column (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON v1 ATTRIBUTES WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "a property needs its name (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON v1 ATTRIBUTES v1 = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "the source is a query after AS (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON v1 ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "the query needs AS (recorded syntax error)",
            sql: "CREATE CORTEX SEARCH SERVICE v1 ON v1 ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' SELECT 1 ;",
            dialect: Dialect::Snowflake,
            expected_accepted: false,
        },
        AcceptanceTestCase {
            description: "SEARCH SERVICE needs CORTEX (recorded syntax error)",
            sql: "CREATE SEARCH SERVICE v1 ON v1 ATTRIBUTES v1 WAREHOUSE = v1 TARGET_LAG = '1 minute' AS SELECT 1 ;",
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
fn given_a_snowflake_cortex_search_service_create_when_parsing_then_the_tree_keeps_its_parts() {
    let test_cases = [
        ProjectionTestCase {
            description: "a service keeps its kind",
            sql: "CREATE CORTEX SEARCH SERVICE s1 ON c1 WAREHOUSE = w1 TARGET_LAG = '1 hour' AS SELECT c1 FROM t1",
            pointer: "/0/create_object/kind",
            expected_value: json!("CORTEX SEARCH SERVICE"),
        },
        ProjectionTestCase {
            description: "a service on one column keeps that column",
            sql: "CREATE CORTEX SEARCH SERVICE s1 ON c1 WAREHOUSE = w1 TARGET_LAG = '1 hour' AS SELECT c1 FROM t1",
            pointer: "/0/create_object/properties/0/value/identifier/name",
            expected_value: json!("c1"),
        },
        ProjectionTestCase {
            description: "vector indexes keep their second column",
            sql: "CREATE CORTEX SEARCH SERVICE s1 TEXT INDEXES c1 VECTOR INDEXES c2 , c3 WAREHOUSE = w1 TARGET_LAG = '1 hour' AS SELECT c1 FROM t1",
            pointer: "/0/create_object/properties/1/value/tuple/expressions/1/identifier/name",
            expected_value: json!("c3"),
        },
        ProjectionTestCase {
            description: "the query is the service's operand",
            sql: "CREATE CORTEX SEARCH SERVICE s1 ON c1 WAREHOUSE = w1 TARGET_LAG = '1 hour' AS SELECT c1 FROM t1",
            pointer: "/0/create_object/operands/0/select/from/expressions/0/table/name/name",
            expected_value: json!("t1"),
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
