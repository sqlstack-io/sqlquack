use serde_json::json;

use crate::helpers::snowflake_kind;
use crate::helpers::statements;
use crate::test_types::{ProjectionTestCase, SnowflakeVerdictTestCase};
use sqlquack_syntax::{CallMode, Dialect};

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
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "untyped columns with policies before AS (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 MASKING POLICY v1 ) ROW ACCESS POLICY v1 ON ( v1 ) AS SELECT 1 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a constant clustering key (recorded OK)",
            sql: "CREATE OR ALTER TABLE v1 ( v1 INT ) CLUSTER BY ( 1 , 1 ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "copy options after LIKE (recorded OK)",
            sql: "CREATE OR REPLACE TABLE v1 LIKE v1 CLUSTER BY ( 1 , 1 ) COPY GRANTS COPY TAGS COPY = ( COMMENTS , TAGS ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "join, aggregation and storage lifecycle policies (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT ) JOIN POLICY v1 ALLOWED JOIN KEYS ( v1 , v1 ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a word comment and a word tag value (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT ) TAG ( v1 = m1 ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "constraint properties (recorded OK)",
            sql: "CREATE TABLE v1 ( v1 INT UNIQUE , v1 INT REFERENCES v1 INITIALLY DEFERRED ) ;",
            expected_kind: Some("create_object"),
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
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a template from a string after $ (recorded errno 1003)",
            sql: "CREATE TABLE v1 USING TEMPLATE $'v1' ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a target lag and warehouse before AS (recorded OK)",
            sql: "CREATE TABLE v1 CLUSTER BY ( 1 ) TARGET_LAG = '1 minute' WAREHOUSE = v1 AS SELECT 1 ;",
            expected_kind: Some("create_object"),
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
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "a bare external REFERENCES constraint (recorded errno 2003, after parsing)",
            sql: "CREATE EXTERNAL TABLE v1 ( REFERENCES v1 ( \"v1 Q\" ) ) LOCATION = @s1 FILE_FORMAT = ( FORMAT_NAME = 'm' ) ;",
            expected_kind: Some("create_object"),
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
            expected_kind: Some("create_object"),
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

#[test]
fn given_a_snowflake_external_table_when_parsing_then_its_columns_and_properties_are_structured() {
    let test_cases = [
        ProjectionTestCase {
            description: "the statement is an external table",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/kind",
            expected_value: json!("EXTERNAL TABLE"),
        },
        ProjectionTestCase {
            description: "OR REPLACE is kept",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/or_replace",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "the column keeps its name",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/column_def/name/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "the column keeps its UNIQUE constraint",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/column_def/unique",
            expected_value: json!(true),
        },
        ProjectionTestCase {
            description: "the column keeps its expression as a computed column",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/column_def/constraints/0/ComputedColumn/persisted",
            expected_value: json!(false),
        },
        ProjectionTestCase {
            description: "PARTITION BY is the first property",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("PARTITION BY"),
        },
        ProjectionTestCase {
            description: "LOCATION is the second property",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/properties/1/kind",
            expected_value: json!("LOCATION"),
        },
        ProjectionTestCase {
            description: "FILE_FORMAT is the third property",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/properties/2/kind",
            expected_value: json!("FILE_FORMAT"),
        },
        ProjectionTestCase {
            description: "TAG is the fourth property",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/properties/3/kind",
            expected_value: json!("TAG"),
        },
        ProjectionTestCase {
            description: "there are only four properties",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/properties/4",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "LOCATION keeps its stage",
            sql: "CREATE OR REPLACE EXTERNAL TABLE v1 ( v1 INT AS ( VALUE : c1 :: INT ) UNIQUE ) PARTITION BY ( v1 ) LOCATION = @s1/p1 FILE_FORMAT = ( TYPE = PARQUET ) TAG ( t1 = 'm' ) ;",
            pointer: "/0/create_object/properties/1/value/literal/value",
            expected_value: json!("@s1/p1"),
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
fn given_a_snowflake_table_with_policies_when_parsing_then_its_columns_and_properties_are_structured(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "the statement is a table",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/kind",
            expected_value: json!("TABLE"),
        },
        ProjectionTestCase {
            description: "TRANSIENT is its modifier",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/modifier",
            expected_value: json!("TRANSIENT"),
        },
        ProjectionTestCase {
            description: "the second column keeps its name",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/1/column_def/name/name",
            expected_value: json!("v2"),
        },
        ProjectionTestCase {
            description: "the constraint is not a column",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/2",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "the first column is NOT NULL",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/column_def/nullable",
            expected_value: json!(false),
        },
        ProjectionTestCase {
            description: "the first column keeps its masking policy",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/column_def/options/0/masking_policy_column_constraint/this/table/name/name",
            expected_value: json!("p1"),
        },
        ProjectionTestCase {
            description: "the primary key is the first property",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("PRIMARY KEY"),
        },
        ProjectionTestCase {
            description: "the row access policy is the second property",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/properties/1/kind",
            expected_value: json!("ROW ACCESS POLICY"),
        },
        ProjectionTestCase {
            description: "there are only two properties",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/properties/2",
            expected_value: json!(null),
        },
        ProjectionTestCase {
            description: "the primary key keeps its name",
            sql: "CREATE TRANSIENT TABLE v1 ( v1 INT NOT NULL MASKING POLICY p1 USING ( v1 , v2 ) , v2 INT , CONSTRAINT k1 PRIMARY KEY ( v1 ) RELY ) ROW ACCESS POLICY p2 ON ( v1 ) ;",
            pointer: "/0/create_object/properties/0/name/name",
            expected_value: json!("k1"),
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
fn given_a_snowflake_dynamic_iceberg_table_when_parsing_then_its_query_and_properties_are_structured(
) {
    let test_cases = [
        ProjectionTestCase {
            description: "the statement is a dynamic iceberg table",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT COMMENT 'm' ) TARGET_LAG = '1 minute' WAREHOUSE = w1 EXECUTE AS USER u1 AS SELECT 1 AS v1 ;",
            pointer: "/0/create_object/kind",
            expected_value: json!("DYNAMIC ICEBERG TABLE"),
        },
        ProjectionTestCase {
            description: "its source is a query",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT COMMENT 'm' ) TARGET_LAG = '1 minute' WAREHOUSE = w1 EXECUTE AS USER u1 AS SELECT 1 AS v1 ;",
            pointer: "/0/create_object/source",
            expected_value: json!("AS"),
        },
        ProjectionTestCase {
            description: "the column keeps its comment",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT COMMENT 'm' ) TARGET_LAG = '1 minute' WAREHOUSE = w1 EXECUTE AS USER u1 AS SELECT 1 AS v1 ;",
            pointer: "/0/create_object/operands/0/tuple/expressions/0/column_def/comment",
            expected_value: json!("m"),
        },
        ProjectionTestCase {
            description: "the query keeps its select list",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT COMMENT 'm' ) TARGET_LAG = '1 minute' WAREHOUSE = w1 EXECUTE AS USER u1 AS SELECT 1 AS v1 ;",
            pointer: "/0/create_object/operands/1/select/expressions/0/alias/alias/name",
            expected_value: json!("v1"),
        },
        ProjectionTestCase {
            description: "TARGET_LAG is the first property",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT COMMENT 'm' ) TARGET_LAG = '1 minute' WAREHOUSE = w1 EXECUTE AS USER u1 AS SELECT 1 AS v1 ;",
            pointer: "/0/create_object/properties/0/kind",
            expected_value: json!("TARGET_LAG"),
        },
        ProjectionTestCase {
            description: "WAREHOUSE is the second property",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT COMMENT 'm' ) TARGET_LAG = '1 minute' WAREHOUSE = w1 EXECUTE AS USER u1 AS SELECT 1 AS v1 ;",
            pointer: "/0/create_object/properties/1/kind",
            expected_value: json!("WAREHOUSE"),
        },
        ProjectionTestCase {
            description: "EXECUTE AS USER is the third property",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT COMMENT 'm' ) TARGET_LAG = '1 minute' WAREHOUSE = w1 EXECUTE AS USER u1 AS SELECT 1 AS v1 ;",
            pointer: "/0/create_object/properties/2/kind",
            expected_value: json!("EXECUTE AS USER"),
        },
        ProjectionTestCase {
            description: "there are only three properties",
            sql: "CREATE DYNAMIC ICEBERG TABLE v1 ( v1 INT COMMENT 'm' ) TARGET_LAG = '1 minute' WAREHOUSE = w1 EXECUTE AS USER u1 AS SELECT 1 AS v1 ;",
            pointer: "/0/create_object/properties/3",
            expected_value: json!(null),
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
fn given_snowflake_event_routing_and_feature_tables_when_parsing_then_their_grammar_is_checked() {
    let test_cases = [
        SnowflakeVerdictTestCase {
            description: "an event table with table properties (recorded OK)",
            sql: "CREATE EVENT TABLE v1 CLUSTER BY ( 1 , 1 ) WITH COMMENT = 'm' TAG ( v1 = 'm' ) ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an event table with a column list (recorded errno 1003)",
            sql: "CREATE EVENT TABLE v1 ( v1 = 'm' ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a routing rule missing a field name (recorded errno 1003)",
            sql: "CREATE EVENT ROUTING TABLE v1 WITH RULES v1 = ( REGION_GROUP = g1 , = ( 'm' ) , DESTINATION_ACCOUNT = v1 . v1 ) ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "an online feature table (recorded OK)",
            sql: "CREATE ONLINE FEATURE TABLE v1 PRIMARY KEY ( v1 ) TARGET_LAG = '1 minute' WAREHOUSE = 'v1' TAG ( v1 = m1 ) FROM t2 ;",
            expected_kind: Some("create_object"),
        },
        SnowflakeVerdictTestCase {
            description: "an online feature table with a reserved warehouse name (recorded errno 1003)",
            sql: "CREATE ONLINE FEATURE TABLE v1 PRIMARY KEY ( v1 ) TARGET_LAG = '1 minute' WAREHOUSE = SELECT FROM t2 ;",
            expected_kind: None,
        },
        SnowflakeVerdictTestCase {
            description: "a partial online feature kind (recorded errno 1003)",
            sql: "CREATE FEATURE TABLE v1 PRIMARY KEY ( v1 ) TARGET_LAG = '1 minute' WAREHOUSE = v1 FROM t2 ;",
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
