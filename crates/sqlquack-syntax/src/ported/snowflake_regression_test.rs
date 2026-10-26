/// Regression tests for Snowflake SQL the Python connector generates.
use crate::dialects::{Dialect, DialectType};
use crate::expressions::{Expression, Literal};
use crate::ported::compat::{parse_one, ExpressionWalk};
use crate::tokens::TokenType;

fn snowflake_string_value(sql: &str) -> String {
    Dialect::get(DialectType::Snowflake)
        .tokenize(sql)
        .expect("Snowflake SQL should tokenize")
        .into_iter()
        .find(|token| token.token_type == TokenType::String)
        .expect("SQL should contain a string token")
        .text
}

// =====================================================================
// PRIOR identifier and CONNECT BY semantics
// Related: https://github.com/tobilg/polyglot/issues/406
// =====================================================================

// =====================================================================
// Trailing SELECT commas at nested query boundaries
// Related: https://github.com/tobilg/polyglot/issues/407
// =====================================================================

#[test]
fn test_snowflake_trailing_select_comma_scope_remains_narrow() {
    for sql in [
        "SELECT 1 AS value_column,, FROM a_relation",
        "SELECT COALESCE(1, )",
        "SELECT ARRAY_CONSTRUCT(1, )",
    ] {
        assert!(
            parse_one(sql, DialectType::Snowflake).is_err(),
            "unsupported comma placement should fail for {sql}"
        );
    }
}

// =====================================================================
// String escape semantics
// =====================================================================

#[test]
fn test_snowflake_documented_escape_sequences() {
    assert_eq!(
        snowflake_string_value(r"SELECT '\b\f\n\r\t\0'"),
        "\x08\x0C\n\r\t\0"
    );
    assert_eq!(
        snowflake_string_value(r"SELECT '\041\x21\u26c4'"),
        "!!\u{26c4}"
    );
    assert_eq!(snowflake_string_value(r#"SELECT '\'\"\\'"#), "'\"\\");
    assert_eq!(snowflake_string_value(r"SELECT '\z\a\v\Z\%\_'"), "zavZ%_");
}

#[test]
fn test_snowflake_malformed_numeric_escapes_drop_only_the_backslash() {
    assert_eq!(
        snowflake_string_value(r"SELECT '\x2-\u26c-\777'"),
        "x2-u26c-777"
    );
}

#[test]
fn test_snowflake_dollar_quoted_strings_keep_escapes_literal() {
    let sql = r"SELECT $$O\'Reilly\n\x21\u26c4$$";
    let tokens = Dialect::get(DialectType::Snowflake)
        .tokenize(sql)
        .expect("dollar-quoted string should tokenize");
    let string = tokens
        .into_iter()
        .find(|token| token.token_type == TokenType::DollarString)
        .expect("SQL should contain a dollar string");

    assert_eq!(string.text, r"O\'Reilly\n\x21\u26c4");
}

// =====================================================================
// Date/time difference semantics
// =====================================================================

// =====================================================================
// Numeric conversion overloads
// Related: https://github.com/tobilg/polyglot/issues/399
// =====================================================================

#[test]
fn test_snowflake_try_to_number_numeric_args_are_precision_and_scale() {
    let expression = parse_one(
        "SELECT TRY_TO_NUMBER('12.34', 9, 2)",
        DialectType::Snowflake,
    )
    .expect("Snowflake TRY_TO_NUMBER should parse");
    let to_number = expression
        .dfs()
        .find_map(|node| match node {
            Expression::ToNumber(to_number) => Some(to_number),
            _ => None,
        })
        .expect("TRY_TO_NUMBER should use the typed numeric conversion AST");

    assert!(to_number.format.is_none());
    assert!(matches!(
        to_number.precision.as_deref(),
        Some(Expression::Literal(literal))
            if matches!(literal.as_ref(), Literal::Number(value) if value == "9")
    ));
    assert!(matches!(
        to_number.scale.as_deref(),
        Some(Expression::Literal(literal))
            if matches!(literal.as_ref(), Literal::Number(value) if value == "2")
    ));
    assert!(to_number.safe.is_some());
}

// =====================================================================
// Category 1: TIMESTAMP_TZ(N) precision casts
// =====================================================================

#[test]
fn test_snowflake_timestamp_tz_cast_precision() {
    let sql = r#"SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_TZ(9)"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "TIMESTAMP_TZ(9) cast failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_timestamp_ntz_cast_precision() {
    let sql = r#"SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_NTZ(9)"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "TIMESTAMP_NTZ(9) cast failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_timestamp_ltz_cast_precision() {
    let sql = r#"SELECT '1987-01-30 23:59:59.123456789'::TIMESTAMP_LTZ(9)"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "TIMESTAMP_LTZ(9) cast failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_timestamptz_no_underscore_precision() {
    let sql = r#"SELECT '2024-01-01'::TIMESTAMPTZ(6)"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "TIMESTAMPTZ(6) cast failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_timestamp_tz_in_insert() {
    let sql = r#"INSERT INTO t (id, dt)
        SELECT seq4() + 0, '1987-01-30 23:59:59.123456789'::TIMESTAMP_TZ(9)
        FROM TABLE(GENERATOR(ROWCOUNT => 10000))"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "TIMESTAMP_TZ(9) in INSERT failed: {:?}",
        result.err()
    );
}

// Roundtrip: verify the generated SQL re-parses correctly

// =====================================================================
// Category 2: PUT file upload commands
// =====================================================================

#[test]
fn test_snowflake_put_file_to_stage() {
    let sql = "PUT 'file:///tmp/file0.txt' @my_stage parallel=4 source_compression=auto_detect";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with quoted file URI failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_put_file_with_placeholder() {
    let sql = "PUT file:///tmp/data.csv ?";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with ? placeholder failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_put_quoted_stage() {
    let sql = "PUT 'file:///tmp/data.csv' '@SYSTEM$BIND/1234/0/data_0_0_0.snappy.parquet' OVERWRITE=TRUE AUTO_COMPRESS=FALSE SOURCE_COMPRESSION=NONE";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with quoted stage failed: {:?}",
        result.err()
    );
}

// Minimal PUT with just source and ? target
#[test]
fn test_snowflake_put_minimal_placeholder() {
    let sql = "PUT 'file:///tmp/f.csv' ?";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT minimal with ? failed: {:?}",
        result.err()
    );
}

// PUT roundtrip

// =====================================================================
// Category 3: COPY INTO with bind params
// =====================================================================

#[test]
fn test_snowflake_copy_placeholder_target() {
    let sql = r#"COPY INTO "DB"."SCHEMA"."TABLE" FROM ? FILE_FORMAT = (TYPE = CSV)"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "COPY INTO with ? source failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_copy_into_with_on_error_placeholder() {
    let sql = r#"COPY INTO identifier(?) (name,points)
        FROM (SELECT $1:name::TEXT,$1:points::NUMBER(38, 0)
        FROM '@SNOWPARK_TEMP_STAGE_ABC123')
        FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False
        COMPRESSION=auto BINARY_AS_TEXT=FALSE) PURGE=TRUE ON_ERROR=?"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(result.is_ok(), "COPY ON_ERROR=? failed: {:?}", result.err());
}

#[test]
fn test_snowflake_copy_into_with_use_logical_type() {
    let sql = r#"COPY INTO identifier(?) ("DT")
        FROM (SELECT $1:"DT" FROM '@"STAGE_ABC"')
        FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False
        COMPRESSION=auto USE_LOGICAL_TYPE = TRUE) PURGE=TRUE ON_ERROR=?"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "USE_LOGICAL_TYPE failed: {:?}",
        result.err()
    );
}

// Reversed parameter order: ON_ERROR before PURGE
#[test]
fn test_snowflake_copy_reversed_param_order() {
    let sql = r#"COPY INTO identifier(?) ("A")
        FROM (SELECT $1:"A" FROM '@"STG"')
        FILE_FORMAT=(TYPE=PARQUET COMPRESSION=auto) ON_ERROR=? PURGE=TRUE"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "COPY reversed param order failed: {:?}",
        result.err()
    );
}

// Minimal COPY with ? source
#[test]
fn test_snowflake_copy_minimal_placeholder() {
    let sql = r#"COPY INTO "T" FROM ?"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "COPY minimal with ? source failed: {:?}",
        result.err()
    );
}

// COPY roundtrip

// =====================================================================
// Category 5: INSERT OVERWRITE ALL
// =====================================================================

// =====================================================================
// Exact SQL from issue #164 — verifies the issue is resolved
// =====================================================================

#[test]
fn issue_164_cat1_placeholder_bind_params() {
    let sql = r#"COPY INTO identifier(?) /* Python:snowflake.connector.pandas_tools.write_pandas() */ ("name","points") FROM (SELECT $1:"name"::TEXT,$1:"points"::NUMBER(38, 0) FROM '@"SNOWPARK_TEMP_STAGE_A49SXIO2Z6"') FILE_FORMAT=(TYPE=PARQUET USE_VECTORIZED_SCANNER=False COMPRESSION=auto BINARY_AS_TEXT=FALSE) PURGE=TRUE ON_ERROR=?"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(result.is_ok(), "Issue #164 Cat1 failed: {:?}", result.err());
}

#[test]
fn issue_164_cat2_put_placeholder_stage() {
    let sql =
        "PUT 'file:///var/folders/temp/file0.txt' ? parallel=4 source_compression=auto_detect";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(result.is_ok(), "Issue #164 Cat2 failed: {:?}", result.err());
}

#[test]
fn issue_164_cat3_timestamp_precision() {
    let sql = "INSERT INTO test_timestamp_mixed_batches (id, dt) SELECT seq4() + 0, '1987-01-30 23:59:59.123456789'::TIMESTAMP_NTZ(9) FROM TABLE(GENERATOR(ROWCOUNT => 10000))";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(result.is_ok(), "Issue #164 Cat3 failed: {:?}", result.err());
}

#[test]
fn issue_164_cat4_insert_overwrite_all() {
    let sql = "insert overwrite all when c1 > 20 then into test_tgt1 values (c1) when c1 > 10 then into test_tgt1 values (c1) into test_tgt2 values (c1) else into test_tgt2 values (c1) select c1 from test_src_multi";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(result.is_ok(), "Issue #164 Cat4 failed: {:?}", result.err());
}

/// `name%`, an unquoted identifier the connector's `write_pandas()` generates.
#[test]
fn issue_164_cat5_name_percent_known_limitation() {
    let sql = r#"CREATE TABLE IF NOT EXISTS identifier(?) (name% TEXT, points NUMBER(38, 0)) /* Python:snowflake.connector.pandas_tools.write_pandas() */"#;
    let result = parse_one(sql, DialectType::Snowflake);
    // Expected to fail — unquoted identifiers with % are not valid SQL.
    // The connector should quote these as "name%" instead.
    assert!(
        result.is_err(),
        "name% should fail to parse (known limitation)"
    );
}

// =====================================================================
// Compatibility: verify non-COPY/PUT Snowflake syntax still works
// =====================================================================

// =====================================================================
// Category A: DollarString in PUT (source/target)
// Related: https://github.com/tobilg/polyglot/issues/165
// =====================================================================

#[test]
fn test_snowflake_put_dollar_quoted_stage() {
    // JDBC driver wraps stage references in $$...$$ for special characters
    let sql = r#"put file:///tmp/placeholder $$@%"ice cream (nice)"$$ overwrite=true"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with dollar-quoted stage target failed: {:?}",
        result.err()
    );
}

// =====================================================================
// Category B: DollarString as table source in SELECT FROM
// Related: https://github.com/tobilg/polyglot/issues/165
// =====================================================================

#[test]
fn test_snowflake_select_from_dollar_quoted_stage() {
    // JDBC driver uses $$...$$ for stage paths in FROM clause
    let sql = r#"SELECT $1 FROM $$@%"ice cream (nice)"/$$"#;
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "SELECT FROM dollar-quoted stage path failed: {:?}",
        result.err()
    );
}

// =====================================================================
// Category C: @~ user stage with UUID path segments
// Related: https://github.com/tobilg/polyglot/issues/165
// =====================================================================

#[test]
fn test_snowflake_put_user_stage_uuid_path() {
    let sql =
        "put file:///tmp/placeholder @~/00626646-bb1e-4729-a1ab-d4b96aebbed5/testUploadStream overwrite=true";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with @~/UUID/name path failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_put_user_stage_uuid_path_compress() {
    let sql =
        "put file:///tmp/placeholder @~/00626646-bb1e-4729-a1ab-d4b96aebbed5/testCompressAndUploadStream overwrite=true";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with @~/UUID/compressUpload path failed: {:?}",
        result.err()
    );
}

// =====================================================================
// Category D: Named stage with UUID subpath + AUTO_COMPRESS
// Related: https://github.com/tobilg/polyglot/issues/165
// =====================================================================

#[test]
fn test_snowflake_put_named_stage_uuid_subpath_auto_compress() {
    let sql = "PUT file:///tmp/test_file.csv @teststage/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=FALSE";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with @stage/UUID AUTO_COMPRESS=FALSE failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_put_named_stage_uuid_subpath_auto_compress_true() {
    let sql = "PUT file:///tmp/test_file.csv @teststage/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=TRUE";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with @stage/UUID AUTO_COMPRESS=TRUE failed: {:?}",
        result.err()
    );
}

#[test]
fn test_snowflake_put_long_path_stage_uuid() {
    // Real JDBC test path with long file URI and named stage + UUID
    let sql = "PUT file:///Users/test/projects/test_file.csv @testeb07cda9279e4320a061356c5a1eef53/c8b31cea-a6d1-4413-936d-bf8c9d63ab9f AUTO_COMPRESS=FALSE";
    let result = parse_one(sql, DialectType::Snowflake);
    assert!(
        result.is_ok(),
        "PUT with long path + @stage/UUID failed: {:?}",
        result.err()
    );
}
