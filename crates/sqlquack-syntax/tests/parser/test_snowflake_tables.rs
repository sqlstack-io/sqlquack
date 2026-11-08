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

#[test]
fn given_incomplete_snowflake_constraint_clauses_when_parsing_then_they_are_refused() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "MATCH with no match type (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT REFERENCES v1 MATCH ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "NO with no ACTION (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT , FOREIGN KEY ( v1 ) REFERENCES v1 ON DELETE NO ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "INITIALLY with no mode (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT , UNIQUE ( v1 ) INITIALLY ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a constraint COMMENT with no string (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT , UNIQUE ( v1 ) COMMENT ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a primary key on no columns (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT , PRIMARY KEY ( ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a reserved word as a tag name (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT TAG ( SELECT = 'm' ) ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a parenthesized property list after WITH (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT ) WITH ( STEWARD = v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a complete match type stays a table (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT REFERENCES v1 MATCH SIMPLE ) ;",
            expected_kind: Some("create_table"),
        },
        SnowflakeVerdictTestCase {
            description: "a complete referential action stays a table (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT , FOREIGN KEY ( v1 ) REFERENCES v1 ON DELETE NO ACTION ) ;",
            expected_kind: Some("create_table"),
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
fn given_snowflake_table_templates_and_conversions_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a template from a session variable (recorded OK)",
            sql: "CREATE TABLE v1 USING TEMPLATE $v1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a template from a string after $ (recorded errno 1003)",
            sql: "CREATE TABLE v1 USING TEMPLATE $'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a target lag and warehouse before AS (recorded OK)",
            sql: "CREATE TABLE v1 CLUSTER BY ( 1 ) TARGET_LAG = '1 minute' WAREHOUSE = v1 AS SELECT 1 ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a conversion to a managed Iceberg table (recorded OK)",
            sql: "ALTER ICEBERG TABLE v1 CONVERT TO MANAGED BASE_LOCATION = m1 ;",
            expected_kind: Some("alter_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a conversion property with no value (recorded errno 1003)",
            sql: "ALTER ICEBERG TABLE v1 CONVERT TO MANAGED BASE_LOCATION = ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a conversion of a plain table (recorded errno 1003)",
            sql: "ALTER TABLE v1 CONVERT TO MANAGED ;",
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
fn given_snowflake_external_tables_procedures_and_unloads_when_parsing_then_recorded_verdicts_hold()
{
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an external column with no type or expression (recorded errno 2003, after parsing)",
            sql: "CREATE EXTERNAL TABLE v1 ( v1 ) LOCATION = @s1 FILE_FORMAT = ( FORMAT_NAME = 'm' ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a bare external REFERENCES constraint (recorded errno 2003, after parsing)",
            sql: "CREATE EXTERNAL TABLE v1 ( REFERENCES v1 ( \"v1 Q\" ) ) LOCATION = @s1 FILE_FORMAT = ( FORMAT_NAME = 'm' ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "a bare external REFERENCES with an empty column list (recorded errno 1003)",
            sql: "CREATE EXTERNAL TABLE v1 ( REFERENCES v1 ( ) ) LOCATION = @s1 FILE_FORMAT = ( FORMAT_NAME = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an external stage quoted with a blank (recorded errno 1003)",
            sql: "CREATE EXTERNAL TABLE v1 ( v1 INT AS 1 ) LOCATION = @\"v1 Q\".s1 FILE_FORMAT = ( FORMAT_NAME = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an external stage named SELECT (recorded OK)",
            sql: "CREATE EXTERNAL TABLE v1 ( v1 INT AS 1 ) LOCATION = @SELECT.s1 FILE_FORMAT = ( FORMAT_NAME = 'm' ) ;",
            expected_kind: Some("raw"),
        },
        SnowflakeVerdictTestCase {
            description: "VALIDATE on a constraint without ENABLE (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT REFERENCES v1 VALIDATE ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "NOVALIDATE on an out-of-line key without ENABLE (recorded errno 1003)",
            sql: "CREATE TABLE v1 ( v1 INT , PRIMARY KEY ( v1 ) NOVALIDATE ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "volatility in a Java procedure (recorded errno 1003)",
            sql: "CREATE PROCEDURE v1 ( ) RETURNS INT LANGUAGE JAVA RUNTIME_VERSION = 'm' PACKAGES = ( 'com.snowflake:snowpark:1.14.0' ) HANDLER = 'm' VOLATILE ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "null handling in a secure Python procedure (recorded errno 1003)",
            sql: "DEFINE TEMP SECURE PROCEDURE d1.s1.v1 ( v1 INT DEFAULT 1 , v1 INT DEFAULT 1 ) COPY GRANTS RETURNS INT NOT NULL LANGUAGE PYTHON RUNTIME_VERSION = '3.10' ARTIFACT_REPOSITORY = v1 PACKAGES = ( 'm' , 'm' ) IMPORTS = ( 'm' , 'm' ) HANDLER = 'm' EXTERNAL_ACCESS_INTEGRATIONS = ( v1 , v1 ) SECRETS = ( 'm' = d1.s1.v1 , 'm' = d1.s1.v1 ) CALLED ON NULL INPUT VOLATILE COMMENT = 'm' COPY = ( GRANTS ) EXECUTE AS OWNER AS 'm' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an unload PARTITION BY after another option (recorded errno 1003)",
            sql: "COPY INTO 's3://b1/p1/' FROM v1 . v1 STORAGE_INTEGRATION = i1 ENCRYPTION = ( TYPE = 'AWS_SSE_KMS' KMS_KEY_ID = 'm' ) PARTITION BY 1 FILE_FORMAT = ( FORMAT_NAME = 'f1' ) OVERWRITE = TRUE SINGLE = TRUE MAX_FILE_SIZE = 1 INCLUDE_QUERY_ID = TRUE DETAILED_OUTPUT = TRUE VALIDATION_MODE = RETURN_ROWS HEADER = TRUE ;",
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
fn given_a_snowflake_sample_clause_when_parsing_then_recorded_verdicts_hold() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "a sample size that is a string (recorded errno 1003)",
            sql: "SELECT 1 FROM t1 SAMPLE ( 'x' ROWS ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a negative sample size (recorded errno 1003)",
            sql: "SELECT * FROM t1 SAMPLE ( -1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a negative sample seed (recorded errno 1003)",
            sql: "SELECT * FROM t1 SAMPLE SYSTEM ( 10 ) SEED ( -1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "keeps an unsigned sample seed",
            sql: "SELECT * FROM t1 SAMPLE SYSTEM ( 10 ) SEED ( 1 ) ;",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description:
                "keeps a session variable as the sample size (recorded errno 2211, after parsing)",
            sql: "SELECT user_id, value FROM table_name SAMPLE BERNOULLI ($s) SEED (0)",
            expected_kind: Some("select"),
        },
        SnowflakeVerdictTestCase {
            description: "a dollar sign without a variable name as the sample size (recorded errno 1003)",
            sql: "SELECT * FROM t1 SAMPLE ( $ ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a space between the dollar sign and the variable name in the sample size (recorded errno 1003)",
            sql: "SELECT user_id, value FROM table_name SAMPLE BERNOULLI ($ s)",
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
