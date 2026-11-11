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
            description: "INSERT_ONLY belongs to external tables only",
            sql: "CREATE STREAM v1 ON TABLE v1 INSERT_ONLY = TRUE ;",
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
