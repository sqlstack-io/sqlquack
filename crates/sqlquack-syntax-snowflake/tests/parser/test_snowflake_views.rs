use crate::helpers::snowflake_kind;
use crate::test_types::SnowflakeVerdictTestCase;

#[test]
fn given_snowflake_streams_when_parsing_then_their_grammar_is_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a stream on a table with every option (recorded OK)",
            sql: "CREATE OR REPLACE STREAM IF NOT EXISTS v1 WITH TAG ( v1 = 'm' , v1 = 'm' ) COPY GRANTS ON TABLE v1 AT ( STREAM => 'm' ) APPEND_ONLY = TRUE SHOW_INITIAL_ROWS = TRUE COMMENT = 'm' COPY = ( GRANTS ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a stream on a stage with a bare comment and an empty copy (recorded OK)",
            sql: "CREATE STREAM v1 TAG ( v1 = m1 ) ON STAGE s1 COMMENT = m1 COPY = ( ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a stream on an external table reading from a timestamp (recorded OK)",
            sql: "CREATE STREAM v1 ON EXTERNAL TABLE v1 BEFORE ( TIMESTAMP => CURRENT_TIMESTAMP ( ) ) INSERT_ONLY = TRUE ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a stream missing its source kind (recorded errno 1003)",
            sql: "CREATE STREAM v1 ON v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a time travel point missing its kind (recorded errno 1003)",
            sql: "CREATE STREAM v1 ON TABLE v1 AT ( => 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a tag missing its value (recorded errno 1003)",
            sql: "CREATE STREAM v1 TAG ( v1 = ) ON VIEW v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unclosed copy list (recorded errno 1003)",
            sql: "CREATE STREAM v1 ON DYNAMIC TABLE v1 COPY = ( GRANTS ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "CREATE OR ALTER STREAM reads only a table",
            sql: "CREATE OR ALTER STREAM v1 ON VIEW v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an option on a source that cannot use it parses (recorded errno 2002)",
            sql: "CREATE STREAM changes ON STAGE order_files APPEND_ONLY = TRUE",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a quoted option name parses (recorded errno 1420)",
            sql: "CREATE STREAM changes ON TABLE orders \"APPEND_ONLY\" = TRUE",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a stage stream reading from an offset (recorded errno 91145)",
            sql: "CREATE STREAM changes ON STAGE files AT (OFFSET => -17)",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a version as the time travel point (recorded errno 2003)",
            sql: "CREATE STREAM changes ON TABLE orders AT (VERSION => 17)",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a number as a tag value (recorded errno 1008)",
            sql: "CREATE STREAM changes TAG (area = 17) ON TABLE orders",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "CREATE OR ALTER STREAM with tags (recorded errno 2003)",
            sql: "CREATE OR ALTER STREAM changes TAG (area = 'west') ON TABLE orders",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "CREATE OR ALTER STREAM never copies grants (recorded errno 1003)",
            sql: "CREATE OR ALTER STREAM changes COPY GRANTS ON TABLE orders",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a stream missing its STREAM keyword (recorded errno 1003)",
            sql: "CREATE v1 ON TABLE v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "CREATE OR ALTER missing its STREAM keyword (recorded errno 1003)",
            sql: "CREATE OR ALTER v1 ON TABLE v1 ;",
            expected_kind: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            snowflake_kind(test_case.sql),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_snowflake_views_when_parsing_then_their_grammar_is_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a join policy with its allowed join keys (recorded OK)",
            sql: "CREATE VIEW v1 JOIN POLICY v1 ALLOWED JOIN KEYS ( v1 ) AS SELECT 1 ;",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "a row access policy on no columns (recorded OK)",
            sql: "CREATE VIEW v1 ROW ACCESS POLICY v1 ON ( ) AS SELECT 1 ;",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "a recursive view (recorded OK)",
            sql: "CREATE RECURSIVE VIEW v1 AS SELECT 1 ;",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "a secure materialized view (recorded OK)",
            sql: "CREATE SECURE MATERIALIZED VIEW v1 ( v1 ) AS SELECT 1 ;",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "an empty copy list before a materialized view's columns (recorded OK)",
            sql: "CREATE MATERIALIZED VIEW v1 COPY = ( ) ( v1 ) AS SELECT 1 ;",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "CREATE OR ALTER VIEW with change tracking (recorded OK)",
            sql: "CREATE OR ALTER VIEW v1 CHANGE_TRACKING = TRUE AS SELECT 1 ;",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "a column list after a view property (recorded errno 91905)",
            sql: "CREATE VIEW my_view CHANGE_TRACKING=TRUE (id) AS SELECT 1 AS id",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "a view COMMENT without = (recorded errno 1003)",
            sql: "CREATE VIEW v COMMENT 'c' AS SELECT 1 AS a",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a column policy after the column list (recorded errno 1003)",
            sql: "CREATE VIEW v1 v1 MASKING POLICY v1 USING ( v1 , v1 ) PROJECTION POLICY v1 TAG ( v1 = 'm' ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an empty column list (recorded errno 1003)",
            sql: "CREATE MATERIALIZED VIEW v1 ( ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a string as a column name (recorded errno 1003)",
            sql: "CREATE MATERIALIZED VIEW v1 ( 'v1' ) AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "RECURSIVE after a temporary scope (recorded errno 1003)",
            sql: "CREATE OR ALTER SECURE LOCAL TEMP RECURSIVE VIEW v1 ( v1 ) CHANGE_TRACKING = TRUE COMMENT = 'm' AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a DCM definition with RECURSIVE after a temporary scope (recorded errno 1003)",
            sql: "DEFINE SECURE LOCAL TEMP RECURSIVE VIEW d1.s1.v1 ( v1 ) CHANGE_TRACKING = TRUE COMMENT = 'm' AS SELECT 1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a temporary view (recorded errno 1647, after parsing)",
            sql: "DEFINE LOCAL TEMP VIEW d1.s1.v1 AS SELECT 1 ;",
            expected_kind: Some("create_view"),
        },
        SnowflakeVerdictTestCase {
            description: "a materialized view is never temporary",
            sql: "CREATE TEMPORARY MATERIALIZED VIEW v1 AS SELECT 1 ;",
            expected_kind: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            snowflake_kind(test_case.sql),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_snowflake_view_alters_when_parsing_then_their_grammar_is_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "SET SECURE (recorded OK)",
            sql: "ALTER VIEW v1 SET SECURE ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "SET of a comment on a view that may not exist (recorded OK)",
            sql: "ALTER VIEW IF EXISTS v1 SET COMMENT = 'm' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "SET of a property with a bare value (recorded OK)",
            sql: "ALTER VIEW v1 SET COMMENT = m1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "SET of a data metric schedule with the name omitted (recorded OK)",
            sql: "ALTER VIEW SET DATA_METRIC_SCHEDULE = '1 MINUTE' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "UNSET of a property on a quoted name (recorded OK)",
            sql: "ALTER VIEW \"v1 Q\" UNSET SECURE ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "UNSET of a bare word (recorded OK)",
            sql: "ALTER VIEW v1 UNSET STEWARD ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a materialized view's RECLUSTER (recorded OK)",
            sql: "ALTER MATERIALIZED VIEW v1 RECLUSTER ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a materialized view's data metric schedule with the name omitted (recorded OK)",
            sql: "ALTER MATERIALIZED VIEW SET DATA_METRIC_SCHEDULE = '1 MINUTE' ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "no action (recorded errno 1003)",
            sql: "ALTER VIEW v1 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "UNSET with the name omitted (recorded errno 1003)",
            sql: "ALTER VIEW UNSET SECURE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "SET SECURE with other properties (recorded errno 1003)",
            sql: "ALTER VIEW IF EXISTS v1 SET SECURE CHANGE_TRACKING = TRUE COMMENT = 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "UNSET SECURE with another property (recorded errno 1003)",
            sql: "ALTER VIEW IF EXISTS v1 UNSET SECURE COMMENT ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "UNSET SECURE in a list (recorded errno 1003)",
            sql: "ALTER VIEW v1 UNSET SECURE , SECURE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "contact pairs mixed with other properties (recorded errno 1003)",
            sql: "ALTER VIEW IF EXISTS v1 SET SECURE CHANGE_TRACKING = TRUE CONTACT STEWARD = v1 , STEWARD = v1 COMMENT = 'm' ;",
            expected_kind: None,
        },
    ];
    for test_case in test_cases {
        assert_eq!(
            snowflake_kind(test_case.sql),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}
