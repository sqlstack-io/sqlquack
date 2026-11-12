use crate::parser::tests::helpers::first_token_text;
use crate::parser::tests::helpers::kind_at;
use crate::parser::tests::helpers::parse_error;
use crate::parser::tests::test_types::DialectTokenTestCase;
use crate::parser::tests::test_types::ParseErrorTestCase;
use crate::parser::tests::test_types::ParseKindTestCase;
use sqlquack_syntax::tokens::models::DialectType;
use sqlquack_syntax::tokens::models::TokenType;

#[test]
fn given_connector_generated_snowflake_sql_when_parsing_then_each_is_one_statement() {
    let test_cases = [
        ParseKindTestCase {
            description: "TIMESTAMP_TZ takes a precision in a cast",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_TZ(9)",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "TIMESTAMP_NTZ takes a precision in a cast",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_NTZ(9)",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "TIMESTAMP_LTZ takes a precision in a cast",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_LTZ(9)",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "TIMESTAMPTZ takes a precision in a cast",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT '2024-01-01'::TIMESTAMPTZ(6)",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "a precise timestamp cast inside INSERT ... SELECT",
            dialect: Some(DialectType::Snowflake),
            sql: "INSERT INTO t (id, dt)
        SELECT seq4() + 0, '1987-01-30 23:59:59.123456789'::TIMESTAMP_TZ(9)
        FROM TABLE(GENERATOR(ROWCOUNT => 10000))",
            pointer: "/0",
            expected_kind: Ok("insert"),
        },
        ParseKindTestCase {
            description: "PUT of a quoted file URI to a stage",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT 'file:///tmp/file0.txt' @my_stage parallel=4 source_compression=auto_detect",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "PUT to a placeholder stage",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT file:///tmp/data.csv ?",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "PUT to a quoted stage",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT 'file:///tmp/data.csv' '@SYSTEM$BIND/1234/0/data_0_0_0.snappy.parquet' OVERWRITE=TRUE AUTO_COMPRESS=FALSE SOURCE_COMPRESSION=NONE",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "PUT of a quoted file to a placeholder",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT 'file:///tmp/f.csv' ?",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "COPY INTO from a placeholder",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO \"DB\".\"SCHEMA\".\"TABLE\" FROM ? FILE_FORMAT = (TYPE = CSV)",
            pointer: "/0",
            expected_kind: Ok("copy"),
        },
        ParseKindTestCase {
            description: "COPY INTO with ON_ERROR as a placeholder",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO identifier(?) (name,points)
        FROM (SELECT $1:name::TEXT,$1:points::NUMBER(38, 0)
        FROM '@SNOWPARK_TEMP_STAGE_ABC123')
        FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False
        COMPRESSION=auto BINARY_AS_TEXT=FALSE) PURGE=TRUE ON_ERROR=?",
            pointer: "/0",
            expected_kind: Ok("copy"),
        },
        ParseKindTestCase {
            description: "COPY INTO with USE_LOGICAL_TYPE",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO identifier(?) (\"DT\")
        FROM (SELECT $1:\"DT\" FROM '@\"STAGE_ABC\"')
        FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False
        COMPRESSION=auto USE_LOGICAL_TYPE = TRUE) PURGE=TRUE ON_ERROR=?",
            pointer: "/0",
            expected_kind: Ok("copy"),
        },
        ParseKindTestCase {
            description: "COPY INTO with ON_ERROR before PURGE",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO identifier(?) (\"A\")
        FROM (SELECT $1:\"A\" FROM '@\"STG\"')
        FILE_FORMAT=(TYPE=PARQUET COMPRESSION=auto) ON_ERROR=? PURGE=TRUE",
            pointer: "/0",
            expected_kind: Ok("copy"),
        },
        ParseKindTestCase {
            description: "a minimal COPY INTO from a placeholder",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO \"T\" FROM ?",
            pointer: "/0",
            expected_kind: Ok("copy"),
        },
        ParseKindTestCase {
            description: "the connector's write_pandas COPY INTO",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO identifier(?) /* Python:snowflake.connector.pandas_tools.write_pandas() */ (\"name\",\"points\") FROM (SELECT $1:\"name\"::TEXT,$1:\"points\"::NUMBER(38, 0) FROM '@\"SNOWPARK_TEMP_STAGE_A49SXIO2Z6\"') FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False COMPRESSION=auto BINARY_AS_TEXT=FALSE) PURGE=TRUE ON_ERROR=?",
            pointer: "/0",
            expected_kind: Ok("copy"),
        },
        ParseKindTestCase {
            description: "the connector's PUT to a placeholder stage",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT 'file:///var/folders/temp/file0.txt' ? parallel=4 source_compression=auto_detect",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "the connector's precise timestamp insert",
            dialect: Some(DialectType::Snowflake),
            sql: "INSERT INTO test_timestamp_mixed_batches (id, dt) SELECT seq4() + 0, '1987-01-30 23:59:59.123456789'::TIMESTAMP_NTZ(9) FROM TABLE(GENERATOR(ROWCOUNT => 10000))",
            pointer: "/0",
            expected_kind: Ok("insert"),
        },
        ParseKindTestCase {
            description: "INSERT OVERWRITE ALL with WHEN branches",
            dialect: Some(DialectType::Snowflake),
            sql: "insert overwrite all when c1 > 20 then into test_tgt1 values (c1) when c1 > 10 then into test_tgt1 values (c1) into test_tgt2 values (c1) else into test_tgt2 values (c1) select c1 from test_src_multi",
            pointer: "/0",
            expected_kind: Ok("multitable_inserts"),
        },
        ParseKindTestCase {
            description: "PUT to a dollar-quoted stage",
            dialect: Some(DialectType::Snowflake),
            sql: "put file:///tmp/placeholder $$@%\"ice cream (nice)\"$$ overwrite=true",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "SELECT from a dollar-quoted stage path",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT $1 FROM $$@%\"ice cream (nice)\"/$$",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "PUT to a user stage path with a UUID",
            dialect: Some(DialectType::Snowflake),
            sql: "put file:///tmp/placeholder @~/00626646-bb1e-4729-a1ab-d4b96aebbed5/testUploadStream overwrite=true",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "PUT to another user stage path with a UUID",
            dialect: Some(DialectType::Snowflake),
            sql: "put file:///tmp/placeholder @~/00626646-bb1e-4729-a1ab-d4b96aebbed5/testCompressAndUploadStream overwrite=true",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "PUT to a named stage UUID path without compression",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT file:///tmp/test_file.csv @teststage/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=FALSE",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "PUT to a named stage UUID path with compression",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT file:///tmp/test_file.csv @teststage/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=TRUE",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "PUT of a long path to a stage UUID path",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT file:///Users/test/projects/test_file.csv @testeb07cda9279e4320a061356c5a1eef53/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=FALSE",
            pointer: "/0",
            expected_kind: Ok("put"),
        },
        ParseKindTestCase {
            description: "TIMESTAMP_TZ takes a precision in a cast is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_TZ(9)",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "TIMESTAMP_NTZ takes a precision in a cast is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_NTZ(9)",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "TIMESTAMP_LTZ takes a precision in a cast is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_LTZ(9)",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "TIMESTAMPTZ takes a precision in a cast is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT '2024-01-01'::TIMESTAMPTZ(6)",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "a precise timestamp cast inside INSERT ... SELECT is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "INSERT INTO t (id, dt)
        SELECT seq4() + 0, '1987-01-30 23:59:59.123456789'::TIMESTAMP_TZ(9)
        FROM TABLE(GENERATOR(ROWCOUNT => 10000))",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT of a quoted file URI to a stage is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT 'file:///tmp/file0.txt' @my_stage parallel=4 source_compression=auto_detect",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT to a placeholder stage is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT file:///tmp/data.csv ?",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT to a quoted stage is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT 'file:///tmp/data.csv' '@SYSTEM$BIND/1234/0/data_0_0_0.snappy.parquet' OVERWRITE=TRUE AUTO_COMPRESS=FALSE SOURCE_COMPRESSION=NONE",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT of a quoted file to a placeholder is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT 'file:///tmp/f.csv' ?",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "COPY INTO from a placeholder is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO \"DB\".\"SCHEMA\".\"TABLE\" FROM ? FILE_FORMAT = (TYPE = CSV)",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "COPY INTO with ON_ERROR as a placeholder is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO identifier(?) (name,points)
        FROM (SELECT $1:name::TEXT,$1:points::NUMBER(38, 0)
        FROM '@SNOWPARK_TEMP_STAGE_ABC123')
        FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False
        COMPRESSION=auto BINARY_AS_TEXT=FALSE) PURGE=TRUE ON_ERROR=?",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "COPY INTO with USE_LOGICAL_TYPE is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO identifier(?) (\"DT\")
        FROM (SELECT $1:\"DT\" FROM '@\"STAGE_ABC\"')
        FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False
        COMPRESSION=auto USE_LOGICAL_TYPE = TRUE) PURGE=TRUE ON_ERROR=?",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "COPY INTO with ON_ERROR before PURGE is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO identifier(?) (\"A\")
        FROM (SELECT $1:\"A\" FROM '@\"STG\"')
        FILE_FORMAT=(TYPE=PARQUET COMPRESSION=auto) ON_ERROR=? PURGE=TRUE",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "a minimal COPY INTO from a placeholder is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO \"T\" FROM ?",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "the connector's write_pandas COPY INTO is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "COPY INTO identifier(?) /* Python:snowflake.connector.pandas_tools.write_pandas() */ (\"name\",\"points\") FROM (SELECT $1:\"name\"::TEXT,$1:\"points\"::NUMBER(38, 0) FROM '@\"SNOWPARK_TEMP_STAGE_A49SXIO2Z6\"') FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False COMPRESSION=auto BINARY_AS_TEXT=FALSE) PURGE=TRUE ON_ERROR=?",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "the connector's PUT to a placeholder stage is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT 'file:///var/folders/temp/file0.txt' ? parallel=4 source_compression=auto_detect",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "the connector's precise timestamp insert is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "INSERT INTO test_timestamp_mixed_batches (id, dt) SELECT seq4() + 0, '1987-01-30 23:59:59.123456789'::TIMESTAMP_NTZ(9) FROM TABLE(GENERATOR(ROWCOUNT => 10000))",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "INSERT OVERWRITE ALL with WHEN branches is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "insert overwrite all when c1 > 20 then into test_tgt1 values (c1) when c1 > 10 then into test_tgt1 values (c1) into test_tgt2 values (c1) else into test_tgt2 values (c1) select c1 from test_src_multi",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT to a dollar-quoted stage is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "put file:///tmp/placeholder $$@%\"ice cream (nice)\"$$ overwrite=true",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "SELECT from a dollar-quoted stage path is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT $1 FROM $$@%\"ice cream (nice)\"/$$",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT to a user stage path with a UUID is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "put file:///tmp/placeholder @~/00626646-bb1e-4729-a1ab-d4b96aebbed5/testUploadStream overwrite=true",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT to another user stage path with a UUID is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "put file:///tmp/placeholder @~/00626646-bb1e-4729-a1ab-d4b96aebbed5/testCompressAndUploadStream overwrite=true",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT to a named stage UUID path without compression is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT file:///tmp/test_file.csv @teststage/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=FALSE",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT to a named stage UUID path with compression is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT file:///tmp/test_file.csv @teststage/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=TRUE",
            pointer: "/1",
            expected_kind: Ok(""),
        },
        ParseKindTestCase {
            description: "PUT of a long path to a stage UUID path is one statement",
            dialect: Some(DialectType::Snowflake),
            sql: "PUT file:///Users/test/projects/test_file.csv @testeb07cda9279e4320a061356c5a1eef53/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=FALSE",
            pointer: "/1",
            expected_kind: Ok(""),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_unsupported_snowflake_sql_when_parsing_then_parsing_fails() {
    let test_cases = [
        ParseErrorTestCase {
            description: "two commas after a projection are rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT 1 AS value_column,, FROM a_relation",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a trailing comma in COALESCE is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT COALESCE(1, )",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a trailing comma in ARRAY_CONSTRUCT is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT ARRAY_CONSTRUCT(1, )",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "an empty argument between commas is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CONCAT('a', , 'b')",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a trailing comma in TO_DECIMAL is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT TO_DECIMAL(1, )",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "an empty TIME precision is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS TIME())",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "an empty VARBINARY length is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS VARBINARY())",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "an empty NUMBER parameter list is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS NUMBER())",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a trailing comma in NUMBER parameters is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS NUMBER(9, ))",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word as a type name is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS SELECT)",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word as a dropped table is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "DROP TABLE SELECT",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word as a qualified index part is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "DROP INDEX v1.SELECT",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word as an UNDROP rename target is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "UNDROP SNAPSHOT v1 RENAME TO SELECT",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word as a truncated table is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "TRUNCATE SELECT",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word as a USE target is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "USE SELECT.v1",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word as a declared name is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "DECLARE SELECT RESULTSET; BEGIN RETURN 1; END;",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a CTE name needs AS before its body",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH t (SELECT 1 AS c) SELECT c FROM t",
            expected_error: "Expected AS before the CTE body",
        },
        ParseErrorTestCase {
            description: "prose read as a CTE needs AS before its body",
            dialect: Some(DialectType::Snowflake),
            sql: " with the correct AST shape.",
            expected_error: "Expected AS before the CTE body",
        },
        ParseErrorTestCase {
            description: "a reserved word cannot name a CTE",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH all AS (SELECT 1 AS count) SELECT all.count FROM all",
            expected_error: "Reserved word ALL cannot be an unquoted object name",
        },
        ParseErrorTestCase {
            description: "a reserved word cannot name a truncated table",
            dialect: Some(DialectType::Snowflake),
            sql: "TRUNCATE TABLE table",
            expected_error: "Reserved word TABLE cannot be an unquoted object name",
        },
        ParseErrorTestCase {
            description: "an empty named argument between commas is rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT ROUND(SCALE => 1, EXPR => 2.25, , ROUNDING_MODE => 'HALF_TO_EVEN') AS value",
            expected_error: "Expected a function argument between commas",
        },
        ParseErrorTestCase {
            description: "a string is not a star's EXCLUDE name",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT HASH(* EXCLUDE 'v1') FROM t1",
            expected_error: "Snowflake EXCLUDE names columns, not strings",
        },
        ParseErrorTestCase {
            description: "a string later in an EXCLUDE list is not a name",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT * EXCLUDE (v1, 'v1') FROM t1",
            expected_error: "Snowflake EXCLUDE names columns, not strings",
        },
        ParseErrorTestCase {
            description: "empty parentheses are not a call argument",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT SEARCH((), 'm')",
            expected_error: "Expected an expression in the parentheses",
        },
        ParseErrorTestCase {
            description: "PRECEDING needs a value or UNBOUNDED",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT PERCENT_RANK() OVER (ORDER BY c1 RANGE PRECEDING) FROM t1",
            expected_error: "Snowflake frame bounds need a value or UNBOUNDED before PRECEDING",
        },
        ParseErrorTestCase {
            description: "a frame end needs PRECEDING or FOLLOWING after its value",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT PERCENT_RANK() OVER (ORDER BY c1 RANGE BETWEEN UNBOUNDED PRECEDING AND ROW) FROM t1",
            expected_error: "Expected PRECEDING or FOLLOWING after a frame bound value",
        },
        ParseErrorTestCase {
            description: "an unclosed EXCLUDE list in COUNT stays rejected",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT COUNT(* EXCLUDE (v1) FROM t1",
            expected_error: "Expected RParen",
        },
        ParseErrorTestCase {
            description: "an anonymous procedure needs RETURNS",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () LANGUAGE SQL AS $$ BEGIN RETURN 1; END $$ CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word is not a procedure option value",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE JAVA HANDLER = SELECT CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a handler-language anonymous procedure takes no null-input clause",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE PYTHON RUNTIME_VERSION = '3.11' PACKAGES = ('snowflake-snowpark-python') HANDLER = 'run' CALLED ON NULL INPUT AS 'm' CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a Java anonymous procedure is not STRICT",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE JAVA RUNTIME_VERSION = '11' PACKAGES = ('com.snowflake:snowpark:1.14.0') HANDLER = 'm' STRICT CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "an anonymous procedure is followed by CALL",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE JAVASCRIPT AS 'm' SELECT 1",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a SQL procedure body must parse as a block",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE SQL AS $$ BEGIN LOOP $$ CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word is not a procedure parameter",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE (SELECT INT) RETURNS INT LANGUAGE JAVASCRIPT AS 'm' CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a reserved word is not a RETURNS TABLE column",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS TABLE (SELECT INT) LANGUAGE JAVASCRIPT AS 'm' CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a handler option is given once",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE PYTHON HANDLER = 'a' HANDLER = 'b' CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a following CTE needs AS",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE JAVASCRIPT AS 'm', c1 (SELECT 1) CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a following CTE needs a name",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE JAVASCRIPT AS 'm', AS (SELECT 1) CALL p1()",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "an unquoted name% column is a known limitation",
            dialect: Some(DialectType::Snowflake),
            sql: "CREATE TABLE IF NOT EXISTS identifier(?) (name% TEXT, points NUMBER(38, 0)) /* Python:snowflake.connector.pandas_tools.write_pandas() */",
            expected_error: "error",
        },
    ];

    for test_case in test_cases {
        let error = parse_error(test_case.dialect, test_case.sql);
        assert!(
            error.contains(test_case.expected_error),
            "{}: {error}",
            test_case.description
        );
    }
}

#[test]
fn given_neighbours_of_rejected_snowflake_sql_when_parsing_then_they_parse() {
    let test_cases = [
        ParseKindTestCase {
            description: "SELECT CAST(NULL AS VARCHAR()) parses",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS VARCHAR())",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "SELECT CAST(NULL AS CHAR()) parses",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS CHAR())",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "SELECT CAST(NULL AS TIME(9)) parses",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS TIME(9))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "SELECT CAST(NULL AS NUMBER(9, 2)) parses",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS NUMBER(9, 2))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "SELECT CAST(NULL AS VARBINARY(8)) parses",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT CAST(NULL AS VARBINARY(8))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "DROP TABLE 'SELECT' parses",
            dialect: Some(DialectType::Snowflake),
            sql: "DROP TABLE \"SELECT\"",
            pointer: "/0",
            expected_kind: Ok("drop_table"),
        },
        ParseKindTestCase {
            description: "USE 'SELECT'.v1 parses",
            dialect: Some(DialectType::Snowflake),
            sql: "USE \"SELECT\".v1",
            pointer: "/0",
            expected_kind: Ok("use"),
        },
        ParseKindTestCase {
            description: "SELECT * FROM TABLE(FLATTEN(INPUT => ARRAY_CONSTRUCT(1))) parses",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT * FROM TABLE(FLATTEN(INPUT => ARRAY_CONSTRUCT(1)))",
            pointer: "/0",
            expected_kind: Ok("select"),
        },
        ParseKindTestCase {
            description: "CREATE PROCEDURE p1() RETURNS TABLE parses",
            dialect: Some(DialectType::Snowflake),
            sql: "CREATE PROCEDURE p1() RETURNS TABLE () LANGUAGE SQL AS $$ BEGIN RETURN 1; END $$",
            pointer: "/0",
            expected_kind: Ok("create_procedure"),
        },
        ParseKindTestCase {
            description: "DECLARE 'SELECT' RESULTSET; BEGIN RETURN 1; END; parses",
            dialect: Some(DialectType::Snowflake),
            sql: "DECLARE \"SELECT\" RESULTSET; BEGIN RETURN 1; END;",
            pointer: "/0",
            expected_kind: Ok("scripting_block"),
        },
        ParseKindTestCase {
            description: "WITH p1 AS PROCEDURE parses",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE (v1 INT, v2 INT DEFAULT 2) RETURNS INT NOT NULL LANGUAGE JAVASCRIPT CALLED ON NULL INPUT AS 'm', c1 AS (SELECT 1) CALL p1(v1 => 1) INTO :x",
            pointer: "/0",
            expected_kind: Ok("call"),
        },
        ParseKindTestCase {
            description: "WITH p1 AS PROCEDURE parses",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS INT LANGUAGE JAVA RUNTIME_VERSION = '11' PACKAGES = ('com.snowflake:snowpark:1.14.0') HANDLER = 'm' CALL p1()",
            pointer: "/0",
            expected_kind: Ok("call"),
        },
        ParseKindTestCase {
            description: "untyped parts, name option values and an expression CTE after a procedure parse",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE (v1) RETURNS TABLE (c1) LANGUAGE JAVA RUNTIME_VERSION = r1 IMPORTS = (m1) HANDLER = h1, c2 AS (1) CALL p1()",
            pointer: "/0/call/with/ctes/1/this",
            expected_kind: Ok("literal"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_snowflake_string_escapes_when_tokenizing_then_they_decode_as_documented() {
    let test_cases = [
        DialectTokenTestCase {
            description: "documented control escapes decode",
            dialect: DialectType::Snowflake,
            sql: "SELECT '\\b\\f\\n\\r\\t\\0'",
            token_type: TokenType::String,
            expected_text: "\x08\x0C\n\r\t\0",
        },
        DialectTokenTestCase {
            description: "octal, hex and unicode escapes decode",
            dialect: DialectType::Snowflake,
            sql: "SELECT '\\041\\x21\\u26c4'",
            token_type: TokenType::String,
            expected_text: "!!\u{26c4}",
        },
        DialectTokenTestCase {
            description: "quote and backslash escapes decode",
            dialect: DialectType::Snowflake,
            sql: "SELECT '\\'\\\"\\\\'",
            token_type: TokenType::String,
            expected_text: "'\"\\",
        },
        DialectTokenTestCase {
            description: "unknown escapes drop the backslash",
            dialect: DialectType::Snowflake,
            sql: "SELECT '\\z\\a\\v\\Z\\%\\_'",
            token_type: TokenType::String,
            expected_text: "zavZ%_",
        },
        DialectTokenTestCase {
            description: "malformed numeric escapes drop only the backslash",
            dialect: DialectType::Snowflake,
            sql: "SELECT '\\x2-\\u26c-\\777'",
            token_type: TokenType::String,
            expected_text: "x2-u26c-777",
        },
        DialectTokenTestCase {
            description: "dollar-quoted strings keep escapes literally",
            dialect: DialectType::Snowflake,
            sql: "SELECT $$O\\'Reilly\\n\\x21\\u26c4$$",
            token_type: TokenType::DollarString,
            expected_text: "O\\'Reilly\\n\\x21\\u26c4",
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            first_token_text(test_case.dialect, test_case.sql, test_case.token_type),
            test_case.expected_text,
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_an_anonymous_procedure_call_when_parsing_snowflake_then_its_parts_are_kept() {
    let test_cases = [
        ParseKindTestCase {
            description: "the statement is a call",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS VARCHAR LANGUAGE SQL AS $$ BEGIN AWAIT ALL; END; $$ CALL p1()",
            pointer: "/0",
            expected_kind: Ok("call"),
        },
        ParseKindTestCase {
            description: "the call keeps the procedure it names",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS VARCHAR LANGUAGE SQL AS $$ BEGIN AWAIT ALL; END; $$ CALL p1()",
            pointer: "/0/call/with/ctes/0/this",
            expected_kind: Ok("create_procedure"),
        },
        ParseKindTestCase {
            description: "a SQL body is parsed as a scripting block",
            dialect: Some(DialectType::Snowflake),
            sql: "WITH p1 AS PROCEDURE () RETURNS VARCHAR LANGUAGE SQL AS $$ BEGIN AWAIT ALL; END; $$ CALL p1()",
            pointer: "/0/call/with/ctes/0/this/create_procedure/body/Statements/0",
            expected_kind: Ok("scripting_block"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_alter_session_set_items_when_parsing_then_names_and_values_are_kept() {
    let test_cases = [
        ParseKindTestCase {
            description: "a keyword parameter name is a column",
            dialect: Some(DialectType::Snowflake),
            sql: "ALTER SESSION SET TIMEZONE = 'UTC'",
            pointer: "/0/alter_session/expressions/0/set_item/name",
            expected_kind: Ok("column"),
        },
        ParseKindTestCase {
            description: "a string value is a literal",
            dialect: Some(DialectType::Snowflake),
            sql: "ALTER SESSION SET QUERY_TAG = 'nightly'",
            pointer: "/0/alter_session/expressions/0/set_item/value",
            expected_kind: Ok("literal"),
        },
        ParseKindTestCase {
            description: "a number value after a comma is kept",
            dialect: Some(DialectType::Snowflake),
            sql: "ALTER SESSION SET QUERY_TAG = 'nightly', LOCK_TIMEOUT = 10",
            pointer: "/0/alter_session/expressions/1/set_item/value",
            expected_kind: Ok("literal"),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            kind_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_kind.map(str::to_owned),
            "{}",
            test_case.description
        );
    }
}

#[test]
fn given_a_malformed_alter_session_set_item_when_parsing_then_the_statement_fails() {
    let test_cases = [
        ParseErrorTestCase {
            description: "an unclosed parenthesized value fails",
            dialect: Some(DialectType::Snowflake),
            sql: "ALTER SESSION SET QUERY_TAG = (1",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a stray closing parenthesis as the value fails",
            dialect: Some(DialectType::Snowflake),
            sql: "ALTER SESSION SET QUERY_TAG = )",
            expected_error: "Parse error",
        },
        ParseErrorTestCase {
            description: "a missing parameter name fails",
            dialect: Some(DialectType::Snowflake),
            sql: "ALTER SESSION SET = 1",
            expected_error: "Parse error",
        },
    ];

    for test_case in test_cases {
        let error = parse_error(test_case.dialect, test_case.sql);
        assert!(
            error.contains(test_case.expected_error),
            "{}: {error}",
            test_case.description
        );
    }
}
