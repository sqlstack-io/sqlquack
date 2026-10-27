use serde_json::json;

use crate::parser::tests::helpers::first_token_text;
use crate::parser::tests::helpers::kind_at;
use crate::parser::tests::helpers::parse_error;
use crate::parser::tests::helpers::value_at;
use crate::parser::tests::test_types::DialectTokenTestCase;
use crate::parser::tests::test_types::ParseErrorTestCase;
use crate::parser::tests::test_types::ParseKindTestCase;
use crate::parser::tests::test_types::ParseValueTestCase;
use crate::tokens::models::DialectType;
use crate::tokens::models::TokenType;

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
fn given_try_to_number_with_numbers_when_parsing_snowflake_then_they_are_precision_and_scale() {
    let test_cases = [
        ParseValueTestCase {
            description: "TRY_TO_NUMBER has no format",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT TRY_TO_NUMBER('12.34', 9, 2)",
            pointer: "/0/select/expressions/0/to_number/format",
            expected_value: Ok(json!(null)),
        },
        ParseValueTestCase {
            description: "TRY_TO_NUMBER reads its second argument as the precision",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT TRY_TO_NUMBER('12.34', 9, 2)",
            pointer: "/0/select/expressions/0/to_number/precision/literal/value",
            expected_value: Ok(json!("9")),
        },
        ParseValueTestCase {
            description: "TRY_TO_NUMBER reads its third argument as the scale",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT TRY_TO_NUMBER('12.34', 9, 2)",
            pointer: "/0/select/expressions/0/to_number/scale/literal/value",
            expected_value: Ok(json!("2")),
        },
        ParseValueTestCase {
            description: "TRY_TO_NUMBER is the safe conversion",
            dialect: Some(DialectType::Snowflake),
            sql: "SELECT TRY_TO_NUMBER('12.34', 9, 2)",
            pointer: "/0/select/expressions/0/to_number/safe/boolean/value",
            expected_value: Ok(json!(true)),
        },
    ];

    for test_case in test_cases {
        assert_eq!(
            value_at(test_case.dialect, test_case.sql, test_case.pointer),
            test_case.expected_value,
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
