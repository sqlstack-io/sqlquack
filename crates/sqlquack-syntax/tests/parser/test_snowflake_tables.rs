use crate::helpers::snowflake_kind;
use crate::test_types::SnowflakeVerdictTestCase;

#[test]
fn given_snowflake_table_policies_and_properties_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a table the table model holds stays a table (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT ) ;",
            expected_kind: Some("create_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a column masking policy keeps the statement as written (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT MASKING POLICY v1 USING ( v1 , v1 ) ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "untyped columns with policies before AS (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 MASKING POLICY v1 ) ROW ACCESS POLICY v1 ON ( v1 ) AS SELECT 1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a constant clustering key (recorded OK)",
            sql: "CREATE OR ALTER TABLE v1 ( v1 INT ) CLUSTER BY ( 1 , 1 ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "copy options after LIKE (recorded OK)",
            sql: "CREATE OR REPLACE TABLE v1 LIKE v1 CLUSTER BY ( 1 , 1 ) COPY GRANTS COPY TAGS COPY = ( COMMENTS , TAGS ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "join, aggregation and storage lifecycle policies (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT ) JOIN POLICY v1 ALLOWED JOIN KEYS ( v1 , v1 ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a word comment and a word tag value (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT ) TAG ( v1 = m1 ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "constraint properties (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT UNIQUE , v1 INT REFERENCES v1 INITIALLY DEFERRED ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "an untyped column without AS (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT , v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a masking policy named by a string (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT MASKING POLICY 'v1' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a storage lifecycle policy on no columns (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT ) STORAGE LIFECYCLE POLICY v1 ON ( ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "COPY with no option (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT ) COPY ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a contact whose steward is a string (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT ) WITH CONTACT ( STEWARD = 'v1' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an integer type with a length (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT ( 1 ) ) ;",
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
