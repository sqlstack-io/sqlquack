//! The conversion rules, each with the constructs it converts and its type conditions.

use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::models::{CallShape, SpellingPair, SpellingRule};

/// HASH.
pub const HASH_RULE: &str = "rule.hash";
/// The digits of HASH's result, NUMBER(19,0), a signed 64-bit integer.
pub const HASH_RESULT_PRECISION: u8 = 19;
/// MODE, the most frequent value.
pub const MODE_RULE: &str = "rule.mode";
/// The aggregates the parser keeps as named calls that convert.
pub const NAMED_AGGREGATE_NAMES: &[&str] = &[
    "ANY_VALUE",
    "COUNT_IF",
    "BOOLOR_AGG",
    "BOOLAND_AGG",
    "OBJECT_AGG",
    "MAX_BY",
    "MIN_BY",
    "MODE",
    "MEDIAN",
    MULTI_COUNT,
];
/// COUNT of several values, which the parser keeps as a named call.
pub const MULTI_COUNT: &str = "COUNT";
/// The recorded type of ARRAY_POSITION.
pub const ARRAY_POSITION_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 9,
    scale: 0,
};
/// The function whose arrays hold only strings, so a lambda over them reads each as text.
pub const STRING_ARRAY_FUNCTION: &str = "SPLIT";
/// FILTER, which keeps an array's elements as they are.
pub const FILTER_FUNCTION: &str = "FILTER";
/// The parameter types a lambda over strings may declare.
pub const TEXT_PARAMETER_TYPES: &[&str] = &["var_char", "string", "text"];
/// The type name TO_DATE casts to.
pub const DATE_NAME: &str = "DATE";
/// The type name TO_TIMESTAMP_NTZ casts a DATE to.
pub const TIMESTAMP_NTZ_NAME: &str = "TIMESTAMP_NTZ(9)";
/// The node kind of a lambda.
pub const LAMBDA_KIND: &str = "lambda";
/// What a lambda parameter's DuckDB name starts with.
pub const LAMBDA_PARAMETER_PREFIX: &str = "lambda_";
/// The window aggregate that converts with DISTINCT, over whole partitions.
pub const DISTINCT_WINDOW_AGGREGATE: &str = "count";
/// The named aggregates that convert over a window.
pub const WINDOWED_NAMED_AGGREGATES: &[&str] = &["COUNT_IF", "BOOLOR_AGG", "BOOLAND_AGG"];
/// Window aggregates Snowflake accepts only over an unordered window.
pub const UNORDERED_WINDOW_AGGREGATES: &[&str] = &["BOOLOR_AGG", "BOOLAND_AGG"];
/// Fractional digits DuckDB's TIMESTAMP and TIMESTAMPTZ hold.
pub const MICROSECOND_DIGITS: u8 = 6;
/// The first date every DuckDB nanosecond timestamp on it can hold (DuckDB's range starts 1677-09-21).
pub const NANOSECOND_RANGE_START: (u16, u16, u16) = (1677, 9, 22);
/// The first date past DuckDB's nanosecond timestamp range (which ends on 2262-04-11).
pub const NANOSECOND_RANGE_END: (u16, u16, u16) = (2262, 4, 11);
/// Hours in a day.
pub const HOURS_PER_DAY: u16 = 24;
/// Minutes in an hour, and seconds in a minute.
pub const MINUTES_PER_HOUR: u16 = 60;
/// The largest UTC offset hour accepted in a timestamp literal.
pub const MAX_OFFSET_HOURS: u16 = 14;
/// Characters of a ` ±HH:MM` offset suffix.
pub const OFFSET_WITH_COLON: usize = 7;
/// Characters of a ` ±HHMM` offset suffix.
pub const OFFSET_WITHOUT_COLON: usize = 6;
/// Fractional-second digits a Snowflake timestamp holds at most.
pub const MAX_FRACTION_DIGITS: usize = 9;
/// The length of `YYYY-MM-DD HH:MI`, a timestamp text without seconds.
pub const MINUTE_TEXT_LENGTH: usize = 16;
/// LEFT, RIGHT, SUBSTR and LENGTH.
pub const TEXT_POSITION: &str = "rule.text_position";
/// REPLACE and SPLIT_PART.
pub const TEXT_REPLACE: &str = "rule.text_replace";
/// TRANSLATE.
pub const TEXT_TRANSLATE: &str = "rule.text_translate";
/// LATERAL FLATTEN.
pub const FLATTEN_RULE: &str = "rule.flatten";
/// START WITH ... CONNECT BY PRIOR as a recursive CTE.
pub const HIERARCHY_RULE: &str = "rule.hierarchy";
/// The pseudo-column of a row's depth in a CONNECT BY walk.
pub(crate) const HIERARCHY_LEVEL: &str = "LEVEL";
/// The prefix of the hidden columns a CONNECT BY walk carries.
pub(crate) const HIERARCHY_COLUMN_PREFIX: &str = "SQLQUACK$HIERARCHY_";
/// The recursive CTE of a CONNECT BY walk.
pub(crate) const HIERARCHY_WALK: &str = "SQLQUACK$WALK";
/// The parent row of a CONNECT BY step.
pub(crate) const HIERARCHY_PRIOR: &str = "SQLQUACK$PRIOR";
/// The depth past which a CONNECT BY walk raises rather than grow without end; never truncates.
pub(crate) const HIERARCHY_MAX_DEPTH: u32 = 10_000;
/// How deep a tree may nest for conversion, which recurses once per level, flat chains included.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_TREE_DEPTH: usize = 512;
/// The same on WebAssembly, whose stack is far smaller.
#[cfg(target_arch = "wasm32")]
pub(crate) const MAX_TREE_DEPTH: usize = 128;
/// The tag of the raise past `HIERARCHY_MAX_DEPTH`.
pub(crate) const HIERARCHY_DEPTH_TAG: &str = "connect_by_depth";
/// Tree nodes holding a query, refused in a hierarchical query's projection and ordering.
pub(crate) const HIERARCHY_NESTED_QUERIES: &[&str] = &[
    "subquery",
    "select",
    "union",
    "intersect",
    "except",
    "exists",
    "with",
];
/// PIVOT in FROM as a grouped aggregate of each listed key.
pub const PIVOT_RULE: &str = "rule.pivot";
/// The aggregates a PIVOT converts, each as itself over the rows of one key.
pub(crate) const PIVOT_AGGREGATES: &[&str] = &["SUM", "MAX"];
/// TABLE(GENERATOR(ROWCOUNT => n)) and SEQ4 / SEQ8 over it.
pub const GENERATOR_RULE: &str = "rule.generator";
/// The table function GENERATOR.
pub(crate) const GENERATOR_NAME: &str = "GENERATOR";
/// CHR, whose literal code point a SPLIT_TO_TABLE delimiter may give.
pub(crate) const CHR_NAME: &str = "CHR";
/// The column a generator's rows carry their sequence number in, read only by SEQ4 and SEQ8.
pub(crate) const GENERATOR_SEQUENCE_COLUMN: &str = "SQLQUACK$GENERATOR_SEQUENCE";
/// The largest ROWCOUNT whose sequence numbers a recording shows gap-free.
pub(crate) const GENERATOR_MAX_ROWS: u64 = 100_000;
/// LATERAL SPLIT_TO_TABLE.
pub const SPLIT_TO_TABLE_RULE: &str = "rule.split_to_table";
/// The columns SPLIT_TO_TABLE brings into scope; SEQ is not converted.
pub const SPLIT_TO_TABLE_COLUMNS: &[(&str, SnowflakeType)] = &[
    (
        "INDEX",
        SnowflakeType::Number {
            precision: 38,
            scale: 0,
        },
    ),
    ("VALUE", SnowflakeType::Varchar),
];
/// The serialised name of a LATERAL source.
pub const LATERAL_KEY: &str = "lateral";
/// The columns FLATTEN brings into scope; SEQ is not converted.
pub const FLATTEN_COLUMNS: &[(&str, SnowflakeType)] = &[
    (
        "INDEX",
        SnowflakeType::Number {
            precision: 38,
            scale: 0,
        },
    ),
    ("KEY", SnowflakeType::Varchar),
    ("PATH", SnowflakeType::Varchar),
    ("VALUE", SnowflakeType::SemiStructured),
    ("THIS", SnowflakeType::SemiStructured),
];
/// LISTAGG WITHIN GROUP.
pub const LISTAGG_RULE: &str = "rule.listagg";
/// PERCENTILE_CONT and PERCENTILE_DISC WITHIN GROUP.
pub const PERCENTILE_RULE: &str = "rule.percentile";
/// A percentile fraction's denominator: fractions are literal thousandths.
pub const PERCENTILE_DECIMALS: u32 = 1000;
/// Decimal digits a percentile fraction may have.
pub const PERCENTILE_FRACTION_DIGITS: usize = 3;
/// Digits PERCENTILE_CONT adds to its key's precision and scale.
pub const PERCENTILE_EXTRA_DIGITS: u8 = 3;
/// The widest NUMBER a PERCENTILE_CONT result may have.
pub const PERCENTILE_MAX_PRECISION: u8 = 38;
/// ILIKE with a non-ASCII literal pattern, by lower-casing both sides.
pub const ILIKE_FOLDING: &str = "rule.ilike_folding";
/// INITCAP.
pub const TEXT_INITCAP: &str = "rule.text_initcap";
/// CHR.
pub const TEXT_CHR: &str = "rule.text_chr";
/// HEX_ENCODE.
pub const TEXT_HEX: &str = "rule.text_hex";
/// TRY_HEX_DECODE_STRING.
pub const TEXT_HEX_DECODE: &str = "rule.text_hex_decode";
/// TRY_BASE64_DECODE_BINARY, its bytes read by HEX_ENCODE, TO_VARCHAR with UTF-8 and SUBSTR.
pub const BASE64_BINARY: &str = "rule.base64_binary";
/// The function whose bytes the base64 rule reads.
pub(crate) const BASE64_DECODER: &str = "TRY_BASE64_DECODE_BINARY";
/// TO_BOOLEAN and TRY_TO_BOOLEAN of text.
pub const TO_BOOLEAN_RULE: &str = "rule.to_boolean";
/// MOD and % of NUMBERs.
pub const DECIMAL_MODULO: &str = "rule.decimal_modulo";
/// CURRENT_DATE and CURRENT_TIMESTAMP.
pub const CURRENT_TIME_RULE: &str = "rule.current_time";
/// Casting text to TIME.
pub const CAST_TEXT_TIME: &str = "rule.cast_text_time";
/// Casting TIME to TIME and to text.
pub const TIME_CAST: &str = "rule.time_cast";
/// TIME arithmetic and parts.
pub const TIME_ARITHMETIC: &str = "rule.time_arithmetic";
/// GROUP BY ALL.
pub const GROUP_BY_ALL_RULE: &str = "rule.group_by_all";
/// A DATE or naive timestamp plus or minus an interval literal.
pub const INTERVAL_ARITHMETIC: &str = "rule.interval_arithmetic";
/// Why GROUP BY ALL with every SELECT item an aggregate is refused: no recording shows its rows.
pub const GROUP_BY_ALL: &str = "GROUP BY ALL where every SELECT item is an aggregate";
/// Node kinds that are aggregates, outside a window.
pub const AGGREGATE_KINDS: &[&str] = &[
    "count",
    "sum",
    "avg",
    "min",
    "max",
    "aggregate_function",
    "within_group",
];
/// Node kinds GROUP BY ALL refuses in a SELECT item.
pub const GROUP_BY_ALL_REFUSED_KINDS: &[&str] = &["window_function", "subquery", "exists"];
/// The node kind of a star.
pub const STAR_KIND: &str = "star";
/// Why an interval is refused.
pub const INTERVAL_REFUSAL: &str =
    "an INTERVAL other than a literal of one whole number of days, hours, minutes or seconds";
/// The fields an interval literal the conversion reads may set.
pub const INTERVAL_FIELDS: &[&str] = &["this", "unit"];
/// The parser's kind of an interval unit written as one word, such as `DAY`.
pub const INTERVAL_SIMPLE_UNIT: &str = "simple";
/// The most digits an interval literal's amount may have, so it fits a BIGINT unit count.
pub const INTERVAL_MAX_DIGITS: usize = 9;
/// The precision of a naive timestamp plus or minus an interval, whatever the timestamp's own.
pub const INTERVAL_TIMESTAMP_PRECISION: u8 = 9;
/// The node kind of an interval.
pub const INTERVAL_KIND: &str = "interval";
/// Interval units as written, singular and plural in upper case, and the DATEADD unit each is.
pub const INTERVAL_UNITS: &[(&[&str], &str)] = &[
    (&["DAY", "DAYS"], "DAY"),
    (&["HOUR", "HOURS"], "HOUR"),
    (&["MINUTE", "MINUTES"], "MINUTE"),
    (&["SECOND", "SECONDS"], "SECOND"),
];
/// The interval units added to a DATE that keep it a DATE.
pub const INTERVAL_DATE_UNITS: &[&str] = &["DAY"];
/// TO_TIME and TIME of a naive timestamp.
pub const TIMESTAMP_TIME_RULE: &str = "rule.timestamp_time";
/// TO_TIME and TRY_TO_TIME of text.
pub const TEXT_TO_TIME_RULE: &str = "rule.text_to_time";
/// DATE_FROM_PARTS and TIMESTAMP_NTZ_FROM_PARTS.
pub const FROM_PARTS: &str = "rule.from_parts";
/// Named aggregates.
pub const NAMED_AGGREGATES: &str = "rule.named_aggregates";
/// MAX_BY and MIN_BY.
pub const VALUE_AT_EXTREME_KEY: &str = "rule.value_at_extreme_key";
/// ARRAY_POSITION.
pub const ARRAY_POSITION_RULE: &str = "rule.array_position";
/// ARRAY_CAT.
pub const ARRAY_CAT_RULE: &str = "rule.array_cat";
/// ARRAY_COMPACT.
pub const ARRAY_COMPACT_RULE: &str = "rule.array_compact";
/// GET with a computed index or key.
pub const VARIANT_COMPUTED_GET: &str = "rule.variant_computed_get";
/// PARSE_JSON of a VARIANT.
pub const PARSE_JSON_VARIANT: &str = "rule.parse_json_variant";
/// FILTER and TRANSFORM with a lambda.
pub const LAMBDA_RULE: &str = "rule.lambda";
/// VARIANT equality with text and integers.
pub const VARIANT_EQUALITY: &str = "rule.variant_equality";
/// REDUCE.
pub const REDUCE_RULE: &str = "rule.reduce";
/// Text compared with a NUMBER.
pub const TEXT_NUMBER_COMPARISON: &str = "rule.text_number_comparison";
/// PARTITION BY a VARIANT.
pub const VARIANT_PARTITION: &str = "rule.variant_partition";
/// DATE_PART of zoned timestamps.
pub const ZONED_PARTS: &str = "rule.zoned_parts";
/// DATE_TRUNC, DATEADD and DATEDIFF of TIMESTAMP_TZ values known to hold the UTC offset.
pub const UTC_OFFSET_ARITHMETIC: &str = "rule.utc_offset_arithmetic";
/// TO_DATE of dates and timestamps.
pub const TO_DATE_TEMPORAL: &str = "rule.to_date_temporal";
/// TO_TIMESTAMP and TO_TIMESTAMP_NTZ of dates and naive timestamps.
pub const TO_TIMESTAMP_TEMPORAL: &str = "rule.to_timestamp_temporal";
/// CONVERT_TIMEZONE with zone columns.
pub const ZONE_COLUMNS: &str = "rule.zone_columns";
/// MOD and % with a FLOAT and a literal divisor.
pub const FLOAT_MODULO: &str = "rule.float_modulo";
/// LIKE, NOT LIKE and ILIKE.
pub const LIKE: &str = "rule.like";
/// REGEXP_LIKE, REGEXP_REPLACE and REGEXP_SUBSTR.
pub const REGEXP: &str = "rule.regexp";
/// The POSIX class `[:space:]` in a REGEXP pattern.
pub const REGEXP_POSIX_SPACE: &str = "rule.regexp_posix_space";
/// REGEXP_LIKE with the parameters c, i and s.
pub const REGEXP_PARAMETERS: &str = "rule.regexp_parameters";
/// The POSIX class read alike by Snowflake and RE2, after its opening bracket.
pub const POSIX_SPACE_CLASS: &str = ":space:]";
/// The type of LENGTH.
pub const LENGTH_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 18,
    scale: 0,
};
/// Regular-expression shorthand classes Snowflake and RE2 both read as ASCII classes.
pub const REGEXP_SHORTHANDS: &str = "dDsSwW";
/// The REGEXP functions that read lazy quantifiers and `\n` as recorded.
pub const REGEXP_EXTRACTING_FUNCTIONS: &[&str] = &[
    "REGEXP_SUBSTR",
    "REGEXP_SUBSTR_ALL",
    "REGEXP_COUNT",
    "REGEXP_INSTR",
];
/// The escapes REGEXP_SUBSTR reads as a line feed, a tab and a carriage return.
pub const REGEXP_CONTROL_ESCAPES: &str = "ntr";
/// DATE_PART, EXTRACT and the single-part functions.
pub const DATE_PART_RULE: &str = "rule.date_part";
/// DATE_TRUNC, DATEADD and DATEDIFF.
pub const DATE_ARITHMETIC: &str = "rule.date_arithmetic";
/// CONVERT_TIMEZONE.
pub const TIME_ZONE_CONVERSION: &str = "rule.convert_timezone";
/// TO_TIMESTAMP, TO_TIMESTAMP_NTZ, TO_DATE and their TRY_ forms.
pub const TEXT_TO_TIMESTAMP_RULE: &str = "rule.to_timestamp";
/// TO_TIMESTAMP of a NUMBER with a scale: seconds with a fraction.
pub const DECIMAL_EPOCH_RULE: &str = "rule.to_timestamp_decimal";
/// LPAD and RPAD.
pub const TEXT_PAD: &str = "rule.text_pad";
/// IS DISTINCT FROM and IS NOT DISTINCT FROM.
pub const NULL_SAFE_COMPARISON: &str = "rule.null_safe_comparison";
/// ABS.
pub const NUMERIC_FUNCTION: &str = "rule.abs";
/// DECODE.
pub const DECODE_RULE: &str = "rule.decode";
/// ZEROIFNULL.
pub const ZERO_IF_NULL_RULE: &str = "rule.zeroifnull";
/// ROUND, CEIL, FLOOR and TRUNC.
pub const ROUNDING: &str = "rule.rounding";
/// SIGN.
pub const SIGN_RULE: &str = "rule.sign";
/// TO_CHAR and TO_VARCHAR of a NUMBER with a fixed-point format.
pub const NUMBER_TEXT_FORMAT: &str = "rule.number_text_format";
/// REPEAT of text.
pub const TEXT_REPEAT: &str = "rule.text_repeat";
/// BITAND, BITOR and BITXOR.
pub const BITWISE: &str = "rule.bitwise";
/// TRY_TO_DATE with a literal format.
pub const DATE_FORMAT_RULE: &str = "rule.date_format";
/// TRY_TO_TIMESTAMP_NTZ with a literal format.
pub const TIMESTAMP_FORMAT_RULE: &str = "rule.timestamp_format";
/// The function DATE(x) converts as.
pub const TO_DATE_NAME: &str = "TO_DATE";
/// TRY_PARSE_JSON, whose text no JSON reader can finish is NULL.
pub const TRY_PARSE_JSON_NAME: &str = "TRY_PARSE_JSON";
/// The type name of a cast to VARIANT.
pub const VARIANT_NAME: &str = "VARIANT";
/// PARSE_JSON and TRY_PARSE_JSON.
pub const PARSE_JSON_RULE: &str = "rule.parse_json";
/// Paths into a VARIANT: `v:a.b[0]`, `v[0]`, GET, GET_PATH and GET_IGNORE_CASE.
pub const VARIANT_PATH: &str = "rule.variant_path";
/// TYPEOF of a VARIANT.
pub const VARIANT_TYPE_NAME: &str = "rule.variant_typeof";
/// ROUND's rounding mode to the nearest even digit.
pub const HALF_TO_EVEN: &str = "HALF_TO_EVEN";
/// ROUND's rounding mode away from zero, its default.
pub const HALF_AWAY_FROM_ZERO: &str = "HALF_AWAY_FROM_ZERO";
/// The argument count of ROUND with a rounding mode.
pub const ROUND_WITH_MODE_ARGUMENTS: usize = 3;
/// The node kind of a NULL literal.
pub const NULL_KIND: &str = "null";
/// Functions whose timestamp result Snowflake was recorded widening to nanoseconds.
pub const WIDENING_DATE_ADDS: &[&str] = &["DATEADD", "TIMEADD", "TIMESTAMPADD"];
/// SYSTEM$TYPEOF with its storage suffix removed by REGEXP_REPLACE.
pub const SYSTEM_TYPE_NAME: &str = "rule.system_typeof";
/// SYSTEM$TYPEOF, the static type of an expression with a storage suffix such as `[SB1]`.
pub const SYSTEM_TYPEOF_NAME: &str = "SYSTEM$TYPEOF";
/// The REGEXP_REPLACE pattern that removes SYSTEM$TYPEOF's storage suffix.
pub const STORAGE_SUFFIX_PATTERN: &str = r"\[.*\]$";
/// IS_NULL_VALUE, STRIP_NULL_VALUE and TO_JSON of a VARIANT.
pub const VARIANT_INSPECT: &str = "rule.variant_inspect";
/// Casts of a VARIANT to scalar types.
pub const VARIANT_CAST: &str = "rule.variant_cast";
/// TO_VARCHAR of a VARIANT.
pub const VARIANT_TO_VARCHAR: &str = "rule.variant_to_varchar";
/// The one text function recordings show writing a VARIANT.
pub const VARIANT_TEXT_FUNCTION: &str = "TO_VARCHAR";
/// The alias of the query whose VARIANT result columns are written for output.
pub const OUTPUT_ALIAS: &str = "\"_sqlquack_output\"";
/// The object constructor that keeps NULL values as JSON null.
pub const KEEP_NULL_CONSTRUCTOR: &str = "OBJECT_CONSTRUCT_KEEP_NULL";
/// The constructor an object constant `{...}` converts as.
pub const OBJECT_CONSTRUCTOR: &str = "OBJECT_CONSTRUCT";
/// The node kind of an object constant `{...}`.
pub const OBJECT_CONSTANT_KIND: &str = "map_func";
/// The node kind of a bracket array literal.
pub const ARRAY_LITERAL_KIND: &str = "array_func";
/// A call to a scalar project function.
pub const PROJECT_FUNCTION_CALL: &str = "rule.project_function_call";
/// The relation a SQL function body reads its parameters from.
pub const PARAMETERS_RELATION: &str = "__SQLQUACK_PARAMETERS";
/// The alias of a converted function body in its macro.
pub const BODY_ALIAS: &str = "\"__sqlquack_body\"";
/// The wrapper Snowflake calls table functions in FROM through.
pub const TABLE_WRAPPER: &str = "TABLE";
/// The DuckDB type semi-structured values are held in.
pub const SEMI_STRUCTURED_HOLDING: &str = "JSON";
/// The lambda parameter a simple CASE binds its computed operand and WHEN values to.
pub const CASE_SUBJECT: &str = "case_subject";
/// OBJECT_PICK.
pub const OBJECT_PICK_RULE: &str = "rule.object_pick";
/// OBJECT_KEYS.
pub const OBJECT_KEYS_RULE: &str = "rule.object_keys";
/// ARRAY_CONSTRUCT.
pub const ARRAY_CONSTRUCT_RULE: &str = "rule.array_construct";
/// ARRAY_AGG WITHIN GROUP.
pub const ARRAY_AGG_RULE: &str = "rule.array_agg";
/// ARRAY_SIZE.
pub const ARRAY_SIZE_RULE: &str = "rule.array_size";
/// ARRAY_CONTAINS.
pub const ARRAY_CONTAINS_RULE: &str = "rule.array_contains";
/// ARRAY_CONSTRUCT_COMPACT.
pub const ARRAY_CONSTRUCT_COMPACT_RULE: &str = "rule.array_construct_compact";
/// ARRAY_TO_STRING.
pub const ARRAY_TO_STRING_RULE: &str = "rule.array_to_string";
/// SPLIT.
pub const SPLIT_RULE: &str = "rule.split";
/// PARSE_URL in permissive mode.
pub const PARSE_URL_RULE: &str = "rule.parse_url";
/// ARRAY_EXCEPT.
pub const ARRAY_EXCEPT_RULE: &str = "rule.array_except";
/// TO_VARIANT, and casts of text, booleans and whole numbers to VARIANT.
pub const TO_VARIANT_RULE: &str = "rule.to_variant";
/// IS_OBJECT, IS_ARRAY and IS_BOOLEAN.
pub const VARIANT_KIND_RULE: &str = "rule.variant_kind";
/// IS_DATE, IS_TIME, IS_TIMESTAMP_* and IS_BINARY.
pub const VARIANT_TYPED_CHECK: &str = "rule.variant_typed_check";
/// The one column of the parameter row of a function without parameters.
pub const NO_PARAMETERS_COLUMN: &str = "\"__sqlquack_no_parameters\"";
/// A project table function in FROM.
pub const PROJECT_TABLE_FUNCTION: &str = "rule.project_table_function";
/// The IS_ functions asking whether a VARIANT holds a typed value.
pub const TYPED_CHECKS: [&str; 6] = [
    "IS_DATE",
    "IS_TIME",
    "IS_TIMESTAMP_NTZ",
    "IS_TIMESTAMP_LTZ",
    "IS_TIMESTAMP_TZ",
    "IS_BINARY",
];
/// A cast of a VARIANT to OBJECT or ARRAY.
pub const VARIANT_KIND_CAST: &str = "rule.variant_kind_cast";
/// Each IS_ function this module converts and the DuckDB JSON type it tests for.
pub const VARIANT_KINDS: [(&str, &str); 3] = [
    ("IS_OBJECT", "OBJECT"),
    ("IS_ARRAY", "ARRAY"),
    ("IS_BOOLEAN", "BOOLEAN"),
];
/// The cast targets a VARIANT may be checked against, and the DuckDB JSON type each requires.
pub const KIND_CAST_TARGETS: [(&str, &str); 2] = [("OBJECT", "OBJECT"), ("ARRAY", "ARRAY")];
/// OBJECT_DELETE.
pub const OBJECT_DELETE_RULE: &str = "rule.object_delete";
/// OBJECT_INSERT.
pub const OBJECT_INSERT_RULE: &str = "rule.object_insert";
/// OBJECT_CONSTRUCT and OBJECT_CONSTRUCT_KEEP_NULL.
pub const OBJECT_CONSTRUCT_RULE: &str = "rule.object_construct";
/// The zone from which CONVERT_TIMEZONE's three-argument form is covered.
pub const UTC_ZONE: &str = "UTC";
/// Fractional-second digits a Snowflake timestamp holds at most, as a scale.
pub const MAX_FRACTION_DIGITS_U8: u8 = 9;
/// Digits a literal timestamp scale may have.
pub const SCALE_DIGITS: usize = 2;
/// Date and time parts with the precision of their recorded NUMBER(p,0) type.
pub const DATE_PARTS: &[(&str, u8)] = &[
    ("YEAR", 4),
    ("QUARTER", 2),
    ("MONTH", 2),
    ("WEEK", 2),
    ("WEEKISO", 2),
    ("DAY", 2),
    ("DAYOFWEEK", 2),
    ("DAYOFWEEKISO", 2),
    ("DAYOFYEAR", 4),
    ("YEAROFWEEK", 4),
    ("YEAROFWEEKISO", 4),
    ("HOUR", 2),
    ("MINUTE", 2),
    ("SECOND", 2),
    ("NANOSECOND", 9),
    ("EPOCH_SECOND", 18),
    ("EPOCH_MILLISECOND", 38),
];
/// LEAST and GREATEST.
pub const LEAST_GREATEST: &str = "rule.least_greatest";
/// STARTSWITH and ENDSWITH.
pub const TEXT_AFFIX: &str = "rule.text_affix";
/// CONTAINS.
pub const TEXT_CONTAINS: &str = "rule.contains";
/// MD5 of a text.
pub const TEXT_MD5: &str = "rule.md5";
/// EDITDISTANCE of two texts.
pub const EDIT_DISTANCE: &str = "rule.editdistance";
/// CHARINDEX and POSITION of one text in another.
pub const TEXT_SEARCH_POSITION: &str = "rule.charindex";
/// The recorded type of CHARINDEX and POSITION.
pub const TEXT_SEARCH_POSITION_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 9,
    scale: 0,
};
/// Casts between DATE and timestamp types.
pub const CAST_TEMPORAL: &str = "rule.cast_temporal";
/// Casts of VARCHAR to TIMESTAMP_NTZ, read as TO_TIMESTAMP_NTZ reads text.
pub const CAST_TEXT_TIMESTAMP: &str = "rule.cast_text_timestamp";
/// Casts of TIMESTAMP_TZ values known to be at +00:00 to TIMESTAMP_NTZ or DATE.
pub const CAST_UTC_ZONED: &str = "rule.cast_utc_zoned";

/// SUM of FLOAT.
pub const SUM_FLOAT: &str = "rule.sum_float";

/// SELECT with its FROM, WHERE and projection.
pub const SELECT: &str = "rule.select";
/// SELECT DISTINCT.
pub const DISTINCT: &str = "rule.distinct";
/// A table or CTE in FROM.
pub const FROM_RELATION: &str = "rule.from_relation";
/// A subquery in FROM.
pub const FROM_SUBQUERY: &str = "rule.from_subquery";
/// LATERAL over a subquery.
pub const LATERAL_SUBQUERY: &str = "rule.lateral_subquery";
/// The name of a refused FROM source without one, while collecting refusals, before its position.
pub const OPAQUE_ALIAS_PREFIX: &str = "__SQLQUACK_OPAQUE_";
/// WITH.
pub const COMMON_TABLE: &str = "rule.common_table";
/// INNER, LEFT, RIGHT and FULL joins with ON.
pub const JOIN_ON: &str = "rule.join_on";
/// CROSS and comma joins.
pub const JOIN_CROSS: &str = "rule.join_cross";
/// WHERE.
pub const WHERE: &str = "rule.where";
/// GROUP BY.
pub const GROUP_BY: &str = "rule.group_by";
/// HAVING.
pub const HAVING: &str = "rule.having";
/// QUALIFY.
pub const QUALIFY: &str = "rule.qualify";
/// ORDER BY with Snowflake's NULL placement.
pub const ORDER_BY: &str = "rule.order_by";
/// LIMIT and OFFSET.
pub const LIMIT: &str = "rule.limit";
/// `*` and `t.*`.
pub const STAR: &str = "rule.star";
/// Column references.
pub const COLUMN: &str = "rule.column";
/// Select-list aliases, and references to them.
pub const ALIAS: &str = "rule.alias";
/// UNION, UNION ALL, EXCEPT, MINUS and INTERSECT.
pub const SET_OPERATION: &str = "rule.set_operation";
/// UNION [ALL] BY NAME.
pub const UNION_BY_NAME: &str = "rule.union_by_name";
/// IN with a list.
pub const IN_LIST: &str = "rule.in_list";
/// IN with a subquery.
pub const IN_SUBQUERY: &str = "rule.in_subquery";
/// NOT IN with a subquery.
pub const NOT_IN_SUBQUERY: &str = "rule.not_in_subquery";
/// EXISTS.
pub const EXISTS: &str = "rule.exists";
/// Scalar subqueries.
pub const SCALAR_SUBQUERY: &str = "rule.scalar_subquery";
/// Numeric literals.
pub const NUMBER_LITERAL: &str = "rule.number_literal";
/// String, boolean and NULL literals.
pub const CONSTANT_LITERAL: &str = "rule.constant_literal";
/// Parentheses.
pub const PARENTHESES: &str = "rule.parentheses";
/// A comment attached to an expression.
pub const COMMENT: &str = "rule.comment";
/// Unary minus.
pub const NEGATE: &str = "rule.negate";
/// `+ - *` on NUMBER.
pub const DECIMAL_ARITHMETIC: &str = "rule.decimal_arithmetic";
/// `/` on NUMBER.
pub const DECIMAL_DIVISION: &str = "rule.decimal_division";
/// `+ - * /` with a FLOAT operand.
pub const FLOAT_ARITHMETIC: &str = "rule.float_arithmetic";
/// `= <> < <= > >=`.
pub const COMPARISON: &str = "rule.comparison";
/// A DATE compared with an ISO date string literal.
pub const DATE_LITERAL_COMPARISON: &str = "rule.date_literal_comparison";
/// AND, OR and NOT.
pub const LOGIC: &str = "rule.logic";
/// IS [NOT] NULL.
pub const IS_NULL: &str = "rule.is_null";
/// BETWEEN.
pub const BETWEEN: &str = "rule.between";
/// CASE.
pub const CASE: &str = "rule.case";
/// COALESCE, NVL and IFNULL.
pub const COALESCE: &str = "rule.coalesce";
/// IFF.
pub const IFF: &str = "rule.iff";
/// NULLIF.
pub const NULLIF: &str = "rule.nullif";
/// Casts between NUMBER and FLOAT.
pub const CAST_NUMERIC: &str = "rule.cast_numeric";
/// Casts to VARCHAR.
pub const CAST_TO_TEXT: &str = "rule.cast_to_text";
/// Casts between BOOLEAN and NUMBER.
pub const CAST_BOOLEAN: &str = "rule.cast_boolean";
/// CAST and TRY_CAST of text to NUMBER, FLOAT or BOOLEAN.
pub const CAST_PARSED_TEXT: &str = "rule.cast_parsed_text";
/// Casts of string literals.
pub const CAST_TEXT_LITERAL: &str = "rule.cast_text_literal";
/// Casts to the operand's own type, and of NULL.
pub const CAST_SAME_TYPE: &str = "rule.cast_same_type";
/// TRIM, LTRIM and RTRIM.
pub const TRIM: &str = "rule.trim";
/// UPPER and LOWER.
pub const CASE_MAPPING: &str = "rule.case_mapping";
/// `||` and CONCAT.
pub const CONCAT: &str = "rule.concat";
/// CONCAT_WS.
pub const CONCAT_WS: &str = "rule.concat_ws";
/// COUNT.
pub const COUNT: &str = "rule.count";
/// SUM.
pub const SUM: &str = "rule.sum";
/// AVG.
pub const AVERAGE: &str = "rule.average";
/// MIN and MAX.
pub const MIN_MAX: &str = "rule.min_max";
/// MIN and MAX of FLOAT.
pub const MIN_MAX_FLOAT: &str = "rule.min_max_float";
/// ROW_NUMBER, RANK and DENSE_RANK.
pub const WINDOW_RANKING: &str = "rule.window_ranking";
/// LAG and LEAD.
pub const WINDOW_OFFSET: &str = "rule.window_offset";
/// FIRST_VALUE and LAST_VALUE.
pub const WINDOW_FIRST_LAST: &str = "rule.window_first_last";
/// COUNT, SUM, MIN and MAX over a window.
pub const WINDOW_AGGREGATE: &str = "rule.window_aggregate";

/// The AVG spelling, which may rely on `numeric_range`.
pub const AVG_NAME: &str = "AVG";
/// The node kind of a plain call.
pub const CALL_KIND: &str = "function";
/// The call the RLIKE operator is converted as, proven by recorded RLIKE and REGEXP_LIKE pairs.
pub const REGEXP_LIKE_NAME: &str = "REGEXP_LIKE";
/// The node kind of an aggregate call.
pub const AGGREGATE_CALL_KIND: &str = "aggregate_function";

/// Every call spelling a rule converts; each is exercised by recorded cases.
pub const CALL_SPELLINGS: &[SpellingRule] = &[
    SpellingRule {
        name: "COALESCE",
        node: "coalesce",
        shape: CallShape::Expressions,
        pair: None,
    },
    SpellingRule {
        name: "IFNULL",
        node: "coalesce",
        shape: CallShape::Renamed,
        pair: None,
    },
    SpellingRule {
        name: "NVL",
        node: "nvl",
        shape: CallShape::Fallback,
        pair: None,
    },
    SpellingRule {
        name: "IFF",
        node: "if_func",
        shape: CallShape::Conditional,
        pair: None,
    },
    SpellingRule {
        name: "UPPER",
        node: "upper",
        shape: CallShape::Unary,
        pair: None,
    },
    SpellingRule {
        name: "LOWER",
        node: "lower",
        shape: CallShape::Unary,
        pair: None,
    },
    SpellingRule {
        name: "TRIM",
        node: "trim",
        shape: CallShape::Trim,
        pair: None,
    },
    SpellingRule {
        name: "STARTSWITH",
        node: "starts_with",
        shape: CallShape::Affix,
        pair: None,
    },
    SpellingRule {
        name: "ENDSWITH",
        node: "ends_with",
        shape: CallShape::Affix,
        pair: None,
    },
    SpellingRule {
        name: "SUBSTR",
        node: "substring",
        shape: CallShape::Substring,
        pair: None,
    },
    SpellingRule {
        name: "SUBSTRING",
        node: "substring",
        shape: CallShape::Substring,
        pair: None,
    },
    SpellingRule {
        name: "LENGTH",
        node: "length",
        shape: CallShape::Unary,
        pair: None,
    },
    SpellingRule {
        name: "LEN",
        node: "length",
        shape: CallShape::Unary,
        pair: Some(LEN_PAIR),
    },
    SpellingRule {
        name: "DATEDIFF",
        node: "date_diff",
        shape: CallShape::Difference,
        pair: None,
    },
    SpellingRule {
        name: "TIMEDIFF",
        node: "date_diff",
        shape: CallShape::Difference,
        pair: Some(TIMEDIFF_PAIR),
    },
    SpellingRule {
        name: "TIMESTAMPDIFF",
        node: "date_diff",
        shape: CallShape::Difference,
        pair: Some(TIMESTAMPDIFF_PAIR),
    },
    SpellingRule {
        name: "YEAR",
        node: "year",
        shape: CallShape::Unary,
        pair: Some(PART_FUNCTIONS_PAIR),
    },
    SpellingRule {
        name: "QUARTER",
        node: "quarter",
        shape: CallShape::Unary,
        pair: Some(PART_FUNCTIONS_PAIR),
    },
    SpellingRule {
        name: "MONTH",
        node: "month",
        shape: CallShape::Unary,
        pair: Some(PART_FUNCTIONS_PAIR),
    },
    SpellingRule {
        name: "DAY",
        node: "day",
        shape: CallShape::Unary,
        pair: Some(PART_FUNCTIONS_PAIR),
    },
    SpellingRule {
        name: "HOUR",
        node: "hour",
        shape: CallShape::Unary,
        pair: Some(PART_FUNCTIONS_PAIR),
    },
    SpellingRule {
        name: "MINUTE",
        node: "minute",
        shape: CallShape::Unary,
        pair: Some(PART_FUNCTIONS_PAIR),
    },
    SpellingRule {
        name: "SECOND",
        node: "second",
        shape: CallShape::Unary,
        pair: Some(PART_FUNCTIONS_PAIR),
    },
    SpellingRule {
        name: "COUNT",
        node: "count",
        shape: CallShape::Count,
        pair: None,
    },
    SpellingRule {
        name: "SUM",
        node: "sum",
        shape: CallShape::Aggregate,
        pair: None,
    },
    SpellingRule {
        name: AVG_NAME,
        node: "avg",
        shape: CallShape::Aggregate,
        pair: None,
    },
    SpellingRule {
        name: "MIN",
        node: "min",
        shape: CallShape::Aggregate,
        pair: None,
    },
    SpellingRule {
        name: "MAX",
        node: "max",
        shape: CallShape::Aggregate,
        pair: None,
    },
    SpellingRule {
        name: "ROW_NUMBER",
        node: "row_number",
        shape: CallShape::Ranking,
        pair: None,
    },
    SpellingRule {
        name: "RANK",
        node: "rank",
        shape: CallShape::Ranking,
        pair: None,
    },
    SpellingRule {
        name: "DENSE_RANK",
        node: "dense_rank",
        shape: CallShape::Ranking,
        pair: None,
    },
    SpellingRule {
        name: "LAG",
        node: "lag",
        shape: CallShape::Offset,
        pair: None,
    },
    SpellingRule {
        name: "LEAD",
        node: "lead",
        shape: CallShape::Offset,
        pair: None,
    },
    SpellingRule {
        name: "FIRST_VALUE",
        node: "first_value",
        shape: CallShape::FirstLast,
        pair: None,
    },
    SpellingRule {
        name: "LAST_VALUE",
        node: "last_value",
        shape: CallShape::FirstLast,
        pair: None,
    },
];

/// The pair behind converting `TEXT`, `STRING` and `STRING(n)` as `VARCHAR`.
pub const TEXT_TYPE_PAIR: &str = "text_type_to_varchar";
/// The serialised type names written `TEXT` and `STRING`.
pub const TEXT_TYPE_SPELLINGS: &[&str] = &["text", "string"];
/// The pair behind converting `TO_NUMBER` and its other names as a cast to NUMBER.
pub const TO_NUMBER_PAIR: &str = "to_number_to_cast";
/// The pair behind converting `TO_VARCHAR(x)` and `TO_CHAR(x)` as a cast to VARCHAR.
pub const TO_VARCHAR_PAIR: &str = "to_varchar_to_cast";
/// TO_VARCHAR and TO_CHAR of a date or timestamp with a literal format.
pub const TEMPORAL_TEXT_FORMAT: &str = "rule.temporal_text_format";
/// Format fields: the Snowflake token, its strftime spelling, and whether it is a time field.
pub const TEMPORAL_TEXT_FIELDS: &[(&str, &str, bool)] = &[
    ("YYYY", "%Y", false),
    ("MM", "%m", false),
    ("DD", "%d", false),
    ("HH24", "%H", true),
    ("MI", "%M", true),
    ("SS", "%S", true),
    ("FF9", "%n", true),
    ("FF6", "%f", true),
    ("FF3", "%g", true),
];
/// The format element writing as many fraction digits as the timestamp's precision.
pub(crate) const BARE_FRACTION: &str = "FF";
/// What FF writes for each precision recorded: none for 0, then milli-, micro- and nanoseconds.
pub(crate) const BARE_FRACTION_PATTERNS: &[(u8, &str)] =
    &[(0, ""), (3, "%g"), (6, "%f"), (9, "%n")];
/// Snowflake's TIMESTAMP_NTZ output format, YYYY-MM-DD HH24:MI:SS.FF3, as a strftime pattern.
pub const NTZ_OUTPUT_PATTERN: &str = "%Y-%m-%d %H:%M:%S.%g";
/// Characters a format may hold between fields, written as they are.
pub const TEMPORAL_TEXT_SEPARATORS: &str = "-_/:. ";
/// The pair behind converting `TO_DOUBLE` and `TRY_TO_DOUBLE` as a cast to FLOAT.
pub const TO_DOUBLE_PAIR: &str = "to_double_to_cast";
/// The pair behind converting `MOD(a, b)` as `a % b`.
pub const MOD_PAIR: &str = "mod_function_to_operator";
/// The pair behind converting `LEN` as `LENGTH`.
pub const LEN_PAIR: &str = "len_to_length";
/// The pair behind converting `TIMEDIFF` as `DATEDIFF`.
pub const TIMEDIFF_PAIR: &str = "timediff_to_datediff";
/// The pair behind converting `TIMESTAMPDIFF` as `DATEDIFF`.
pub const TIMESTAMPDIFF_PAIR: &str = "timestampdiff_to_datediff";
/// The pair behind converting `YEAR(x)`, `QUARTER(x)` ... `SECOND(x)` as `DATE_PART(unit, x)`.
pub const PART_FUNCTIONS_PAIR: &str = "part_functions_to_date_part";
/// Spellings other engines accept that are not in Snowflake's function reference.
pub const NOT_SNOWFLAKE_FUNCTIONS: &[&str] = &[
    "CHAR_LENGTH",
    "CHARACTER_LENGTH",
    "UCASE",
    "LCASE",
    "IF",
    "IIF",
    "STARTS_WITH",
    "ENDS_WITH",
    "DATE_DIFF",
];
/// The pair behind converting `t RLIKE p` and `RLIKE(t, p)` through the `REGEXP_LIKE` rule.
pub const RLIKE_PAIR: &str = "rlike_to_regexp_like";
/// A computed `v[i]` spelled as GET.
pub const SUBSCRIPT_PAIR: &str = "subscript_to_get";
/// POSITION spelled as CHARINDEX.
pub const POSITION_PAIR: &str = "position_to_charindex";

/// The pair behind converting `DATE '...'`, `TIMESTAMP '...'` and `TIME '...'` as casts of their text.
pub const TYPED_LITERAL_PAIR: &str = "typed_literal_to_cast";
/// The parser's literal types for typed literals, each also the type its text is cast to.
pub(crate) const TYPED_LITERAL_KINDS: &[&str] = &["date", "timestamp", "time"];
/// The pair behind converting OBJECT_AGG of a typed value as OBJECT_AGG of its TO_VARIANT.
pub const OBJECT_AGG_VALUE_PAIR: &str = "object_agg_value_to_variant";

/// Every recorded spelling pair; `sqlquack verify` requires both spellings' results to match.
pub const SPELLING_PAIRS: &[SpellingPair] = &[
    SpellingPair {
        id: TYPED_LITERAL_PAIR,
        source: "DATE 's', TIMESTAMP 's' and TIME 's'",
        canonical: "CAST('s' AS DATE), CAST('s' AS TIMESTAMP) and CAST('s' AS TIME)",
        cases: "fidelity/typed_literals.toml",
    },
    SpellingPair {
        id: OBJECT_AGG_VALUE_PAIR,
        source: "OBJECT_AGG(k, x) of a NUMBER x",
        canonical: "OBJECT_AGG(k, TO_VARIANT(x))",
        cases: "fidelity/object_agg_values.toml",
    },
    SpellingPair {
        id: "variant_path_to_get_path",
        source: "v:a.b[0]",
        canonical: "GET_PATH(v, 'a.b[0]')",
        cases: "fidelity/variant_path.toml",
    },
    SpellingPair {
        id: TEXT_TYPE_PAIR,
        source: "TEXT, STRING and STRING(n)",
        canonical: "VARCHAR and VARCHAR(n)",
        cases: "fidelity/text_type.toml",
    },
    SpellingPair {
        id: SUBSCRIPT_PAIR,
        source: "v[i] and v[k] with an index or key computed per row",
        canonical: "GET(v, i) and GET(v, k)",
        cases: "fidelity/subscript_get.toml",
    },
    SpellingPair {
        id: POSITION_PAIR,
        source: "POSITION(s, t [, n]) and POSITION(s IN t)",
        canonical: "CHARINDEX(s, t [, n])",
        cases: "fidelity/position.toml",
    },
    SpellingPair {
        id: RLIKE_PAIR,
        source: "t RLIKE p, RLIKE(t, p [, flags])",
        canonical: "REGEXP_LIKE(t, p)",
        cases: "fidelity/rlike.toml",
    },
    SpellingPair {
        id: TO_NUMBER_PAIR,
        source: "TO_NUMBER(s [, p [, s]]), TO_DECIMAL, TO_NUMERIC and their TRY_ forms",
        canonical: "CAST(s AS NUMBER(p, s)) and TRY_CAST(s AS NUMBER(p, s))",
        cases: "fidelity/to_number.toml",
    },
    SpellingPair {
        id: TO_DOUBLE_PAIR,
        source: "TO_DOUBLE(s) and TRY_TO_DOUBLE(s)",
        canonical: "CAST(s AS FLOAT) and TRY_CAST(s AS FLOAT)",
        cases: "fidelity/to_double.toml",
    },
    SpellingPair {
        id: TO_VARCHAR_PAIR,
        source: "TO_VARCHAR(x) and TO_CHAR(x)",
        canonical: "CAST(x AS VARCHAR)",
        cases: "fidelity/to_varchar.toml",
    },
    SpellingPair {
        id: MOD_PAIR,
        source: "MOD(a, b)",
        canonical: "a % b",
        cases: "fidelity/mod.toml",
    },
    SpellingPair {
        id: "from_values_to_subquery",
        source: "FROM VALUES (...) AS v(c)",
        canonical: "FROM (VALUES (...)) AS v(c)",
        cases: "fidelity/from_values.toml",
    },
    SpellingPair {
        id: LEN_PAIR,
        source: "LEN(s)",
        canonical: "LENGTH(s)",
        cases: "fidelity/len.toml",
    },
    SpellingPair {
        id: TIMEDIFF_PAIR,
        source: "TIMEDIFF(unit, a, b)",
        canonical: "DATEDIFF(unit, a, b)",
        cases: "fidelity/timediff.toml",
    },
    SpellingPair {
        id: TIMESTAMPDIFF_PAIR,
        source: "TIMESTAMPDIFF(unit, a, b)",
        canonical: "DATEDIFF(unit, a, b)",
        cases: "fidelity/timestampdiff.toml",
    },
    SpellingPair {
        id: PART_FUNCTIONS_PAIR,
        source: "YEAR(x), QUARTER(x), MONTH(x), DAY(x), HOUR(x), MINUTE(x), SECOND(x), DAYOFWEEKISO(x)",
        canonical: "DATE_PART(unit, x)",
        cases: "fidelity/part_functions.toml",
    },
];

/// Default wall time for converting one statement: far beyond any conversion seen so far.
pub const DEFAULT_BUDGET_MILLISECONDS: u64 = 120_000;
/// Default bound on emitted SQL as a multiple of the source text.
pub const DEFAULT_BUDGET_SIZE_RATIO: u64 = 2_000;
/// Emitted SQL up to this size is within budget whatever the source's size.
pub const DEFAULT_BUDGET_MINIMUM_BYTES: u64 = 4 * 1024 * 1024;

/// The field a located parse records a node's source range in.
pub const SPAN_FIELD: &str = "span";
/// A source range's first character.
pub const SPAN_START: &str = "start";
/// A source range's end, one past its last character.
pub const SPAN_END: &str = "end";

/// Node kinds whose value is a BOOLEAN whatever their operands.
pub const PREDICATE_KINDS: &[&str] = &[
    "eq",
    "neq",
    "lt",
    "lte",
    "gt",
    "gte",
    "null_safe_eq",
    "null_safe_neq",
    "and",
    "or",
    "not",
    "is_null",
    "in",
    "between",
    "exists",
    "like",
    "i_like",
    "regexp_like",
    "starts_with",
    "ends_with",
];

/// Node kinds whose value is a VARCHAR whatever their operands.
pub const TEXT_RESULT_KINDS: &[&str] = &["upper", "lower", "trim", "substring", "concat"];

/// Functions whose documented result type is fixed, for typing them while collecting refusals.
pub const FUNCTION_RESULT_TYPES: &[(&str, SnowflakeType)] = &[
    (
        "TRY_TO_TIMESTAMP_NTZ",
        SnowflakeType::TimestampNtz { precision: 9 },
    ),
    (
        "TO_TIMESTAMP_NTZ",
        SnowflakeType::TimestampNtz { precision: 9 },
    ),
    (
        "TRY_TO_TIMESTAMP_TZ",
        SnowflakeType::TimestampTz { precision: 9 },
    ),
    (
        "TO_TIMESTAMP_TZ",
        SnowflakeType::TimestampTz { precision: 9 },
    ),
    (
        "TRY_TO_TIMESTAMP_LTZ",
        SnowflakeType::TimestampLtz { precision: 9 },
    ),
    (
        "TO_TIMESTAMP_LTZ",
        SnowflakeType::TimestampLtz { precision: 9 },
    ),
    (
        "CURRENT_TIMESTAMP",
        SnowflakeType::TimestampLtz { precision: 9 },
    ),
    ("TRY_TO_DATE", SnowflakeType::Date),
    ("TO_DATE", SnowflakeType::Date),
    ("DATE", SnowflakeType::Date),
    ("CURRENT_DATE", SnowflakeType::Date),
    ("TRY_TO_TIME", SnowflakeType::Time { precision: 9 }),
    ("TO_TIME", SnowflakeType::Time { precision: 9 }),
    ("TRY_TO_BOOLEAN", SnowflakeType::Boolean),
    ("TO_BOOLEAN", SnowflakeType::Boolean),
    ("REGEXP_LIKE", SnowflakeType::Boolean),
    ("ARRAY_CONTAINS", SnowflakeType::Boolean),
    ("IS_OBJECT", SnowflakeType::Boolean),
    ("IS_ARRAY", SnowflakeType::Boolean),
    ("TO_VARCHAR", SnowflakeType::Varchar),
    ("TO_CHAR", SnowflakeType::Varchar),
    ("REGEXP_SUBSTR", SnowflakeType::Varchar),
    ("REGEXP_REPLACE", SnowflakeType::Varchar),
    ("INITCAP", SnowflakeType::Varchar),
    ("SOUNDEX", SnowflakeType::Varchar),
    ("HEX_ENCODE", SnowflakeType::Varchar),
    ("TRY_HEX_DECODE_STRING", SnowflakeType::Varchar),
    ("SPLIT_PART", SnowflakeType::Varchar),
    ("REPLACE", SnowflakeType::Varchar),
    ("CHR", SnowflakeType::Varchar),
    ("MD5", SnowflakeType::Varchar),
    ("SHA2", SnowflakeType::Varchar),
    ("ARRAY_TO_STRING", SnowflakeType::Varchar),
    ("LISTAGG", SnowflakeType::Varchar),
    ("TYPEOF", SnowflakeType::Varchar),
    ("PARSE_JSON", SnowflakeType::SemiStructured),
    ("TRY_PARSE_JSON", SnowflakeType::SemiStructured),
    ("OBJECT_CONSTRUCT", SnowflakeType::SemiStructured),
    ("OBJECT_CONSTRUCT_KEEP_NULL", SnowflakeType::SemiStructured),
    ("OBJECT_INSERT", SnowflakeType::SemiStructured),
    ("OBJECT_AGG", SnowflakeType::SemiStructured),
    ("ARRAY_AGG", SnowflakeType::SemiStructured),
    ("ARRAY_CONSTRUCT", SnowflakeType::SemiStructured),
    ("ARRAY_CAT", SnowflakeType::SemiStructured),
    ("FILTER", SnowflakeType::SemiStructured),
    ("GET", SnowflakeType::SemiStructured),
    ("GET_PATH", SnowflakeType::SemiStructured),
    ("TO_VARIANT", SnowflakeType::SemiStructured),
    ("SPLIT", SnowflakeType::SemiStructured),
    (
        "COUNT_IF",
        SnowflakeType::Number {
            precision: 18,
            scale: 0,
        },
    ),
];
/// Node kinds whose shapes are named as calls, `fn:<NAME>`, besides generic calls.
pub(crate) const SHAPE_CALL_KINDS: &[&str] = &["aggregate_function"];
/// Node kinds an argument's source is read through: parentheses and comments.
pub(crate) const SHAPE_WRAPPER_KINDS: &[&str] = &["paren", "annotated"];
/// The shape facet naming a cast's target type family.
pub(crate) const TARGET_FACET: &str = "target";
/// The shape facet naming the scale a FLOAT is cast to.
pub(crate) const FLOAT_SCALE_FACET: &str = "float_scale";
/// The shape facet naming a LIKE's quantifier: `none`, `any` or `all`.
pub(crate) const QUANTIFIER_FACET: &str = "quantifier";
/// A LIKE's quantifier when none is written.
pub(crate) const NO_QUANTIFIER: &str = "none";
/// The shape facet recording a negation written infix, as in `x NOT LIKE p`.
pub(crate) const NEGATED_FACET: &str = "negated";
/// The value of `negated` for an infix negation.
pub(crate) const INFIX_NEGATION: &str = "infix";
/// The shape facet naming a window's function.
pub(crate) const FUNCTION_FACET: &str = "function";
/// The shape facet naming a window's frame: unordered, ordered, whole partition, rows or range.
pub(crate) const FRAME_FACET: &str = "frame";
/// The construct a window's partition and order keys convert under, outside the window's shape.
pub(crate) const WINDOW_KEYS_CONSTRUCT: &str = "node:window_keys";
/// The shape facet naming a window's partition key family: `none`, a family or `several`.
pub(crate) const PARTITION_FACET: &str = "partition";
/// The `partition` facet of a window without PARTITION BY.
pub(crate) const NO_PARTITION: &str = "none";
/// The `partition` facet of a window partitioned by several keys.
pub(crate) const SEVERAL_PARTITION_KEYS: &str = "several";
/// The LIKE wildcards, which a LIKE escape character must precede.
pub(crate) const LIKE_WILDCARDS: [char; 2] = ['%', '_'];
/// The node kind of unary minus.
pub(crate) const NEGATE_KIND: &str = "neg";
/// The node kind of a literal.
pub(crate) const LITERAL_KIND: &str = "literal";
/// The furthest text position SUBSTR, LEFT and RIGHT pass to DuckDB, beyond any Snowflake text.
pub(crate) const TEXT_POSITION_LIMIT: &str = "2147483647";
/// ROUND, whose FLOAT form to positive places overflows as Snowflake computes it.
pub(crate) const ROUND_FUNCTION: &str = "ROUND";
/// The node kind of a function call.
pub(crate) const FUNCTION_KIND: &str = "function";
/// ARRAY_CONSTRUCT, whose undefined elements text and ARRAY_TO_STRING write when it is direct.
pub(crate) const ARRAY_CONSTRUCT_NAME: &str = "ARRAY_CONSTRUCT";
/// Array builders whose ARRAY OBJECT_PICK was recorded picking by its elements.
pub(crate) const PICKED_ARRAY_FUNCTIONS: [&str; 2] = ["ARRAY_CONSTRUCT", "ARRAY_EXCEPT"];
/// The array builders ARRAY_SIZE reads through, counting a SQL NULL element as Snowflake does.
pub(crate) const COUNTED_ARRAY_FUNCTIONS: [&str; 3] =
    ["ARRAY_APPEND", "ARRAY_CAT", "ARRAY_CONSTRUCT"];
/// Why SYSTEM$TYPEOF text refuses where its storage suffix would be read.
pub(crate) const TYPE_SUFFIX_UNREMOVED: &str = "SYSTEM$TYPEOF text read before \
    REGEXP_REPLACE(..., '\\[.*\\]$', '') removes its storage suffix, which depends on Snowflake's plan";
/// Why a construct that reads VARIANTs only as JSON refuses a VARIANT built from a FLOAT.
pub(crate) const VARIANT_HELD_UNREAD: &str = "a VARIANT built from a FLOAT, which this construct \
                                               reads only as JSON, where the FLOAT's type is lost";

/// The aggregate taking the value at the largest key; MIN_BY takes the smallest.
pub const MAX_BY_FUNCTION: &str = "MAX_BY";
/// Why removing duplicate VARIANTs refuses: DuckDB compares JSON as written, keys in their order.
pub const DUPLICATE_VARIANTS: &str = "removing duplicate VARIANT values, which DuckDB compares by their JSON as written, not as VARIANT equality does";

/// `$n` column positions over the only FROM source.
pub const POSITIONAL_COLUMN: &str = "rule.positional_column";
/// How the parsed tree marks a `$n` parameter.
pub(crate) const DOLLAR_PARAMETER: &str = "Dollar";
/// VALUES rows as a query or FROM source.
pub const VALUES_ROWS: &str = "rule.values_rows";
/// The column names Snowflake gives VALUES rows: COLUMN1, COLUMN2 and so on.
pub(crate) const VALUES_COLUMN_PREFIX: &str = "COLUMN";
/// NTILE, which the parser keeps as a generic call.
pub(crate) const NTILE_NAME: &str = "NTILE";
/// RATIO_TO_REPORT, which the parser keeps as a generic call.
pub(crate) const RATIO_TO_REPORT_NAME: &str = "RATIO_TO_REPORT";
/// IS_INTEGER, which reads TYPEOF's INTEGER where IS_DECIMAL also reads DECIMAL.
pub(crate) const IS_INTEGER_NAME: &str = "IS_INTEGER";
/// The most VALUES rows converted.
pub(crate) const VALUES_ROWS_LIMIT: usize = 1000;
/// DATEDIFF units whose boundaries every whole-minute UTC offset shares, counted on UTC instants.
pub(crate) const OFFSET_FREE_UNITS: [&str; 2] = ["MINUTE", "SECOND"];
/// JOIN ... USING.
pub const JOIN_USING: &str = "rule.join_using";
/// ASOF JOIN ... MATCH_CONDITION.
pub const ASOF_JOIN: &str = "rule.asof_join";
/// How the parsed tree names an ASOF join's kind.
pub(crate) const ASOF_KIND: &str = "AsOf";
/// The relation name given a FROM subquery written without an alias, which no source name can match.
pub(crate) const UNALIASED_SUBQUERY_PREFIX: &str = "__SQLQUACK_SUBQUERY_";
/// Text branches of COALESCE, NVL, IFF and CASE read as the other branches' type.
pub const TEXT_BRANCHES: &str = "rule.text_branches";
/// COALESCE, NVL, IFF and CASE whose branches are all untyped NULLs.
pub const NULL_BRANCHES: &str = "rule.null_branches";
/// The VARCHAR type's name.
pub(crate) const VARCHAR_NAME: &str = "VARCHAR";
/// Set operations reading a branch's VARIANT as text beside another branch's text.
pub const VARIANT_TEXT_BRANCHES: &str = "rule.variant_text_branches";
/// VARIANTs told apart by the text Snowflake writes for them, in DISTINCT and GROUP BY.
pub const VARIANT_EQUALITY_BY_TEXT: &str = "rule.variant_equality_by_text";
/// JAROWINKLER_SIMILARITY.
pub const JARO_WINKLER: &str = "rule.jaro_winkler";
/// FIRST_VALUE and LAST_VALUE over an explicit ROWS frame.
pub const WINDOW_VALUE_FRAME: &str = "rule.window_value_frame";
/// SUBSTR lengths computed from whole NUMBERs, negative ones as zero.
pub const COMPUTED_SUBSTR_LENGTH: &str = "rule.computed_substr_length";
/// DATEADD and TIMESTAMPADD amounts with a fraction, rounded half away from zero.
pub const FRACTIONAL_DATE_AMOUNT: &str = "rule.fractional_date_amount";
/// A DATE plus or minus a whole number of days.
pub const DATE_DAY_ARITHMETIC: &str = "rule.date_day_arithmetic";
/// The unit DATE arithmetic with a number adds.
pub(crate) const DAY_UNIT: &str = "DAY";
/// NULLIFZERO.
pub const NULL_IF_ZERO_RULE: &str = "rule.null_if_zero";
/// LN and EXP of a NUMBER or FLOAT.
pub const LOGARITHM: &str = "rule.logarithm";
/// POWER(10, n) of a whole exponent.
pub const POWER_RULE: &str = "rule.power";
/// The one base POWER converts, as its literal is written.
pub(crate) const POWER_BASE: &str = "10";
/// The natural logarithm's name, beside EXP.
pub(crate) const LN_NAME: &str = "LN";
/// ARRAY_SORT of one array.
pub const ARRAY_SORT_RULE: &str = "rule.array_sort";
/// ARRAY_FLATTEN.
pub const ARRAY_FLATTEN_RULE: &str = "rule.array_flatten";
/// TO_ARRAY.
pub const TO_ARRAY_RULE: &str = "rule.to_array";
/// ARRAY_REVERSE.
pub const ARRAY_REVERSE_RULE: &str = "rule.array_reverse";
/// ARRAY_MAX and ARRAY_MIN.
pub const ARRAY_EXTREME_RULE: &str = "rule.array_extreme";
/// ARRAY_MAX's name, beside ARRAY_MIN.
pub(crate) const ARRAY_MAX_NAME: &str = "ARRAY_MAX";
/// ARRAY_INTERSECTION, ARRAYS_OVERLAP and ARRAY_DISTINCT.
pub const ARRAY_SET_RULE: &str = "rule.array_set";
/// ARRAY_APPEND.
pub const ARRAY_APPEND_RULE: &str = "rule.array_append";
/// ARRAY_SLICE.
pub const ARRAY_SLICE_RULE: &str = "rule.array_slice";
/// ARRAY_GENERATE_RANGE.
pub const ARRAY_RANGE_RULE: &str = "rule.array_range";
/// ARRAY_SORT, which sorts ARRAY_AGG's values in the aggregate and other arrays as values.
pub(crate) const SORTED_ARRAY_FUNCTION: &str = "ARRAY_SORT";
/// REGEXP_COUNT and REGEXP_INSTR.
pub const REGEXP_COUNTING: &str = "rule.regexp_counting";
/// The recorded type of REGEXP_COUNT and REGEXP_INSTR.
pub const REGEXP_COUNT_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 18,
    scale: 0,
};
/// SHA2 of text.
pub const TEXT_SHA2: &str = "rule.text_sha2";
/// SHA2's default digest size, the one DuckDB computes.
pub(crate) const SHA2_DEFAULT_BITS: u64 = 256;
/// The function decoding hexadecimal text to NULL rather than failing.
pub(crate) const TRYING_HEX_DECODE: &str = "TRY_HEX_DECODE_STRING";
/// Snowflake's message for HEX_DECODE_STRING of text that is not hexadecimal UTF-8.
pub(crate) const HEX_DECODE_MESSAGE: &str = "The following string is not a legal hex-encoded value";
/// DAYNAME.
pub const DAY_NAME_RULE: &str = "rule.day_name";
/// LAST_DAY of a DATE or TIMESTAMP_NTZ in its month.
pub const LAST_DAY_RULE: &str = "rule.last_day";
/// SQRT's name, beside LN and EXP.
pub(crate) const SQRT_NAME: &str = "SQRT";
/// TRY_TO_DOUBLE of a FLOAT, the value itself.
pub const FLOAT_TO_DOUBLE: &str = "rule.float_to_double";
/// REGEXP_REPLACE and REGEXP_SUBSTR with a pattern from a column.
pub const REGEXP_COLUMN_PATTERN: &str = "rule.regexp_column_pattern";
/// MAX of JSON-held VARIANTs.
pub const VARIANT_MAX: &str = "rule.variant_max";
/// The parser's node kind for MAX.
pub const MAX_KIND: &str = "max";
/// The OBJECT type name in a cast.
pub const OBJECT_TYPE_NAME: &str = "OBJECT";
/// DIV0 and DIV0NULL.
pub const DIV0_RULE: &str = "rule.div0";
/// MEDIAN.
pub const MEDIAN_RULE: &str = "rule.median";
/// MEDIAN of NUMBER(p,s) is NUMBER(p+3,s+3), its precision capped at 38, as recorded.
pub const MEDIAN_EXTRA_DIGITS: u8 = 3;
/// Significant digits at the result scale up to which a double average rounds back exactly.
pub const MEDIAN_EXACT_DIGITS: u8 = 15;
/// The alias prefix of the relation holding a FULL JOIN USING's merged keys.
pub const USING_ALIAS_PREFIX: &str = "__SQLQUACK_USING_";
/// DuckDB's FULL JOIN keyword.
pub const FULL_JOIN_KEYWORD: &str = "FULL JOIN";
/// The parser's literal kind for a dollar-quoted string, whose text it keeps as written.
pub const DOLLAR_STRING_KIND: &str = "dollar_string";
/// A NUMBER or FLOAT plus or minus an untyped NULL.
pub const NULL_ARITHMETIC: &str = "rule.null_arithmetic";
/// The precision an untyped NULL widens a sum to, as recorded.
pub const NULL_SUM_PRECISION: u8 = 18;
/// The pieces of a TIME format recorded through TO_VARCHAR: clock fields and separators.
pub const CLOCK_FORMAT_FIELDS: &[&str] = &[
    "HH24", "MI", "SS", "FF1", "FF2", "FF3", "FF4", "FF5", "FF6", ":", ".",
];
/// IFF, which passes either branch up as it is.
pub const IFF_NAME: &str = "IFF";
/// Text every converted window function holds, which a window cannot hold inside its partition.
pub const WINDOW_MARKER: &str = " OVER (";
/// FLATTEN columns that depend on the input array's order: its positions and the array itself.
pub const FLATTEN_ORDERED_COLUMNS: &[&str] = &["PATH", "INDEX", "THIS"];
/// Calls to a function a user override replaces.
pub const USER_OVERRIDE_RULE: &str = "rule.user_override";
/// The prefix of the macro converted SQL calls a user override by.
pub const OVERRIDE_MACRO_PREFIX: &str = "__sqlquack_override__";
/// Time zone names recorded as accepted by Snowflake, which matches them exactly, case included.
pub(crate) const RECORDED_TIME_ZONES: &[&str] = &[
    "Africa/Abidjan",
    "Africa/Accra",
    "Africa/Addis_Ababa",
    "Africa/Algiers",
    "Africa/Asmara",
    "Africa/Asmera",
    "Africa/Bamako",
    "Africa/Bangui",
    "Africa/Banjul",
    "Africa/Bissau",
    "Africa/Blantyre",
    "Africa/Brazzaville",
    "Africa/Bujumbura",
    "Africa/Cairo",
    "Africa/Casablanca",
    "Africa/Ceuta",
    "Africa/Conakry",
    "Africa/Dakar",
    "Africa/Dar_es_Salaam",
    "Africa/Djibouti",
    "Africa/Douala",
    "Africa/El_Aaiun",
    "Africa/Freetown",
    "Africa/Gaborone",
    "Africa/Harare",
    "Africa/Johannesburg",
    "Africa/Juba",
    "Africa/Kampala",
    "Africa/Khartoum",
    "Africa/Kigali",
    "Africa/Kinshasa",
    "Africa/Lagos",
    "Africa/Libreville",
    "Africa/Lome",
    "Africa/Luanda",
    "Africa/Lubumbashi",
    "Africa/Lusaka",
    "Africa/Malabo",
    "Africa/Maputo",
    "Africa/Maseru",
    "Africa/Mbabane",
    "Africa/Mogadishu",
    "Africa/Monrovia",
    "Africa/Nairobi",
    "Africa/Ndjamena",
    "Africa/Niamey",
    "Africa/Nouakchott",
    "Africa/Ouagadougou",
    "Africa/Porto-Novo",
    "Africa/Sao_Tome",
    "Africa/Timbuktu",
    "Africa/Tripoli",
    "Africa/Tunis",
    "Africa/Windhoek",
    "America/Adak",
    "America/Anchorage",
    "America/Anguilla",
    "America/Antigua",
    "America/Araguaina",
    "America/Argentina/Buenos_Aires",
    "America/Argentina/Catamarca",
    "America/Argentina/ComodRivadavia",
    "America/Argentina/Cordoba",
    "America/Argentina/Jujuy",
    "America/Argentina/La_Rioja",
    "America/Argentina/Mendoza",
    "America/Argentina/Rio_Gallegos",
    "America/Argentina/Salta",
    "America/Argentina/San_Juan",
    "America/Argentina/San_Luis",
    "America/Argentina/Tucuman",
    "America/Argentina/Ushuaia",
    "America/Aruba",
    "America/Asuncion",
    "America/Atikokan",
    "America/Atka",
    "America/Bahia",
    "America/Bahia_Banderas",
    "America/Barbados",
    "America/Belem",
    "America/Belize",
    "America/Blanc-Sablon",
    "America/Boa_Vista",
    "America/Bogota",
    "America/Boise",
    "America/Buenos_Aires",
    "America/Cambridge_Bay",
    "America/Campo_Grande",
    "America/Cancun",
    "America/Caracas",
    "America/Catamarca",
    "America/Cayenne",
    "America/Cayman",
    "America/Chicago",
    "America/Chihuahua",
    "America/Ciudad_Juarez",
    "America/Coral_Harbour",
    "America/Cordoba",
    "America/Costa_Rica",
    "America/Coyhaique",
    "America/Creston",
    "America/Cuiaba",
    "America/Curacao",
    "America/Danmarkshavn",
    "America/Dawson",
    "America/Dawson_Creek",
    "America/Denver",
    "America/Detroit",
    "America/Dominica",
    "America/Edmonton",
    "America/Eirunepe",
    "America/El_Salvador",
    "America/Ensenada",
    "America/Fort_Nelson",
    "America/Fort_Wayne",
    "America/Fortaleza",
    "America/Glace_Bay",
    "America/Godthab",
    "America/Goose_Bay",
    "America/Grand_Turk",
    "America/Grenada",
    "America/Guadeloupe",
    "America/Guatemala",
    "America/Guayaquil",
    "America/Guyana",
    "America/Halifax",
    "America/Havana",
    "America/Hermosillo",
    "America/Indiana/Indianapolis",
    "America/Indiana/Knox",
    "America/Indiana/Marengo",
    "America/Indiana/Petersburg",
    "America/Indiana/Tell_City",
    "America/Indiana/Vevay",
    "America/Indiana/Vincennes",
    "America/Indiana/Winamac",
    "America/Indianapolis",
    "America/Inuvik",
    "America/Iqaluit",
    "America/Jamaica",
    "America/Jujuy",
    "America/Juneau",
    "America/Kentucky/Louisville",
    "America/Kentucky/Monticello",
    "America/Knox_IN",
    "America/Kralendijk",
    "America/La_Paz",
    "America/Lima",
    "America/Los_Angeles",
    "America/Louisville",
    "America/Lower_Princes",
    "America/Maceio",
    "America/Managua",
    "America/Manaus",
    "America/Marigot",
    "America/Martinique",
    "America/Matamoros",
    "America/Mazatlan",
    "America/Mendoza",
    "America/Menominee",
    "America/Merida",
    "America/Metlakatla",
    "America/Mexico_City",
    "America/Miquelon",
    "America/Moncton",
    "America/Monterrey",
    "America/Montevideo",
    "America/Montreal",
    "America/Montserrat",
    "America/Nassau",
    "America/New_York",
    "America/Nipigon",
    "America/Nome",
    "America/Noronha",
    "America/North_Dakota/Beulah",
    "America/North_Dakota/Center",
    "America/North_Dakota/New_Salem",
    "America/Nuuk",
    "America/Ojinaga",
    "America/Panama",
    "America/Pangnirtung",
    "America/Paramaribo",
    "America/Phoenix",
    "America/Port-au-Prince",
    "America/Port_of_Spain",
    "America/Porto_Acre",
    "America/Porto_Velho",
    "America/Puerto_Rico",
    "America/Punta_Arenas",
    "America/Rainy_River",
    "America/Rankin_Inlet",
    "America/Recife",
    "America/Regina",
    "America/Resolute",
    "America/Rio_Branco",
    "America/Rosario",
    "America/Santa_Isabel",
    "America/Santarem",
    "America/Santiago",
    "America/Santo_Domingo",
    "America/Sao_Paulo",
    "America/Scoresbysund",
    "America/Shiprock",
    "America/Sitka",
    "America/St_Barthelemy",
    "America/St_Johns",
    "America/St_Kitts",
    "America/St_Lucia",
    "America/St_Thomas",
    "America/St_Vincent",
    "America/Swift_Current",
    "America/Tegucigalpa",
    "America/Thule",
    "America/Thunder_Bay",
    "America/Tijuana",
    "America/Toronto",
    "America/Tortola",
    "America/Vancouver",
    "America/Virgin",
    "America/Whitehorse",
    "America/Winnipeg",
    "America/Yakutat",
    "America/Yellowknife",
    "Antarctica/Casey",
    "Antarctica/Davis",
    "Antarctica/DumontDUrville",
    "Antarctica/Macquarie",
    "Antarctica/Mawson",
    "Antarctica/McMurdo",
    "Antarctica/Palmer",
    "Antarctica/Rothera",
    "Antarctica/South_Pole",
    "Antarctica/Syowa",
    "Antarctica/Troll",
    "Antarctica/Vostok",
    "Arctic/Longyearbyen",
    "Asia/Aden",
    "Asia/Almaty",
    "Asia/Amman",
    "Asia/Anadyr",
    "Asia/Aqtau",
    "Asia/Aqtobe",
    "Asia/Ashgabat",
    "Asia/Ashkhabad",
    "Asia/Atyrau",
    "Asia/Baghdad",
    "Asia/Bahrain",
    "Asia/Baku",
    "Asia/Bangkok",
    "Asia/Barnaul",
    "Asia/Beirut",
    "Asia/Bishkek",
    "Asia/Brunei",
    "Asia/Calcutta",
    "Asia/Chita",
    "Asia/Choibalsan",
    "Asia/Chongqing",
    "Asia/Chungking",
    "Asia/Colombo",
    "Asia/Dacca",
    "Asia/Damascus",
    "Asia/Dhaka",
    "Asia/Dili",
    "Asia/Dubai",
    "Asia/Dushanbe",
    "Asia/Famagusta",
    "Asia/Gaza",
    "Asia/Harbin",
    "Asia/Hebron",
    "Asia/Ho_Chi_Minh",
    "Asia/Hong_Kong",
    "Asia/Hovd",
    "Asia/Irkutsk",
    "Asia/Istanbul",
    "Asia/Jakarta",
    "Asia/Jayapura",
    "Asia/Jerusalem",
    "Asia/Kabul",
    "Asia/Kamchatka",
    "Asia/Karachi",
    "Asia/Kashgar",
    "Asia/Kathmandu",
    "Asia/Katmandu",
    "Asia/Khandyga",
    "Asia/Kolkata",
    "Asia/Krasnoyarsk",
    "Asia/Kuala_Lumpur",
    "Asia/Kuching",
    "Asia/Kuwait",
    "Asia/Macao",
    "Asia/Macau",
    "Asia/Magadan",
    "Asia/Makassar",
    "Asia/Manila",
    "Asia/Muscat",
    "Asia/Nicosia",
    "Asia/Novokuznetsk",
    "Asia/Novosibirsk",
    "Asia/Omsk",
    "Asia/Oral",
    "Asia/Phnom_Penh",
    "Asia/Pontianak",
    "Asia/Pyongyang",
    "Asia/Qatar",
    "Asia/Qostanay",
    "Asia/Qyzylorda",
    "Asia/Rangoon",
    "Asia/Riyadh",
    "Asia/Saigon",
    "Asia/Sakhalin",
    "Asia/Samarkand",
    "Asia/Seoul",
    "Asia/Shanghai",
    "Asia/Singapore",
    "Asia/Srednekolymsk",
    "Asia/Taipei",
    "Asia/Tashkent",
    "Asia/Tbilisi",
    "Asia/Tehran",
    "Asia/Tel_Aviv",
    "Asia/Thimbu",
    "Asia/Thimphu",
    "Asia/Tokyo",
    "Asia/Tomsk",
    "Asia/Ujung_Pandang",
    "Asia/Ulaanbaatar",
    "Asia/Ulan_Bator",
    "Asia/Urumqi",
    "Asia/Ust-Nera",
    "Asia/Vientiane",
    "Asia/Vladivostok",
    "Asia/Yakutsk",
    "Asia/Yangon",
    "Asia/Yekaterinburg",
    "Asia/Yerevan",
    "Atlantic/Azores",
    "Atlantic/Bermuda",
    "Atlantic/Canary",
    "Atlantic/Cape_Verde",
    "Atlantic/Faeroe",
    "Atlantic/Faroe",
    "Atlantic/Jan_Mayen",
    "Atlantic/Madeira",
    "Atlantic/Reykjavik",
    "Atlantic/South_Georgia",
    "Atlantic/St_Helena",
    "Atlantic/Stanley",
    "Australia/ACT",
    "Australia/Adelaide",
    "Australia/Brisbane",
    "Australia/Broken_Hill",
    "Australia/Canberra",
    "Australia/Currie",
    "Australia/Darwin",
    "Australia/Eucla",
    "Australia/Hobart",
    "Australia/LHI",
    "Australia/Lindeman",
    "Australia/Lord_Howe",
    "Australia/Melbourne",
    "Australia/NSW",
    "Australia/North",
    "Australia/Perth",
    "Australia/Queensland",
    "Australia/South",
    "Australia/Sydney",
    "Australia/Tasmania",
    "Australia/Victoria",
    "Australia/West",
    "Australia/Yancowinna",
    "Brazil/Acre",
    "Brazil/DeNoronha",
    "Brazil/East",
    "Brazil/West",
    "CET",
    "CST6CDT",
    "Canada/Atlantic",
    "Canada/Central",
    "Canada/Eastern",
    "Canada/Mountain",
    "Canada/Newfoundland",
    "Canada/Pacific",
    "Canada/Saskatchewan",
    "Canada/Yukon",
    "Chile/Continental",
    "Chile/EasterIsland",
    "Cuba",
    "EET",
    "EST",
    "EST5EDT",
    "Egypt",
    "Eire",
    "Etc/GMT",
    "Etc/GMT+0",
    "Etc/GMT+1",
    "Etc/GMT+10",
    "Etc/GMT+11",
    "Etc/GMT+12",
    "Etc/GMT+2",
    "Etc/GMT+3",
    "Etc/GMT+4",
    "Etc/GMT+5",
    "Etc/GMT+6",
    "Etc/GMT+7",
    "Etc/GMT+8",
    "Etc/GMT+9",
    "Etc/GMT-0",
    "Etc/GMT-1",
    "Etc/GMT-10",
    "Etc/GMT-11",
    "Etc/GMT-12",
    "Etc/GMT-13",
    "Etc/GMT-14",
    "Etc/GMT-2",
    "Etc/GMT-3",
    "Etc/GMT-4",
    "Etc/GMT-5",
    "Etc/GMT-6",
    "Etc/GMT-7",
    "Etc/GMT-8",
    "Etc/GMT-9",
    "Etc/GMT0",
    "Etc/Greenwich",
    "Etc/UCT",
    "Etc/UTC",
    "Etc/Universal",
    "Etc/Zulu",
    "Europe/Amsterdam",
    "Europe/Andorra",
    "Europe/Astrakhan",
    "Europe/Athens",
    "Europe/Belfast",
    "Europe/Belgrade",
    "Europe/Berlin",
    "Europe/Bratislava",
    "Europe/Brussels",
    "Europe/Bucharest",
    "Europe/Budapest",
    "Europe/Busingen",
    "Europe/Chisinau",
    "Europe/Copenhagen",
    "Europe/Dublin",
    "Europe/Gibraltar",
    "Europe/Guernsey",
    "Europe/Helsinki",
    "Europe/Isle_of_Man",
    "Europe/Istanbul",
    "Europe/Jersey",
    "Europe/Kaliningrad",
    "Europe/Kiev",
    "Europe/Kirov",
    "Europe/Kyiv",
    "Europe/Lisbon",
    "Europe/Ljubljana",
    "Europe/London",
    "Europe/Luxembourg",
    "Europe/Madrid",
    "Europe/Malta",
    "Europe/Mariehamn",
    "Europe/Minsk",
    "Europe/Monaco",
    "Europe/Moscow",
    "Europe/Nicosia",
    "Europe/Oslo",
    "Europe/Paris",
    "Europe/Podgorica",
    "Europe/Prague",
    "Europe/Riga",
    "Europe/Rome",
    "Europe/Samara",
    "Europe/San_Marino",
    "Europe/Sarajevo",
    "Europe/Saratov",
    "Europe/Simferopol",
    "Europe/Skopje",
    "Europe/Sofia",
    "Europe/Stockholm",
    "Europe/Tallinn",
    "Europe/Tirane",
    "Europe/Tiraspol",
    "Europe/Ulyanovsk",
    "Europe/Uzhgorod",
    "Europe/Vaduz",
    "Europe/Vatican",
    "Europe/Vienna",
    "Europe/Vilnius",
    "Europe/Volgograd",
    "Europe/Warsaw",
    "Europe/Zagreb",
    "Europe/Zaporozhye",
    "Europe/Zurich",
    "Factory",
    "GB",
    "GB-Eire",
    "GMT",
    "GMT+0",
    "GMT-0",
    "GMT0",
    "Greenwich",
    "HST",
    "Hongkong",
    "Iceland",
    "Indian/Antananarivo",
    "Indian/Chagos",
    "Indian/Christmas",
    "Indian/Cocos",
    "Indian/Comoro",
    "Indian/Kerguelen",
    "Indian/Mahe",
    "Indian/Maldives",
    "Indian/Mauritius",
    "Indian/Mayotte",
    "Indian/Reunion",
    "Iran",
    "Israel",
    "Jamaica",
    "Japan",
    "Kwajalein",
    "Libya",
    "MET",
    "MST",
    "MST7MDT",
    "Mexico/BajaNorte",
    "Mexico/BajaSur",
    "Mexico/General",
    "NZ",
    "NZ-CHAT",
    "Navajo",
    "PRC",
    "PST8PDT",
    "Pacific/Apia",
    "Pacific/Auckland",
    "Pacific/Bougainville",
    "Pacific/Chatham",
    "Pacific/Chuuk",
    "Pacific/Easter",
    "Pacific/Efate",
    "Pacific/Enderbury",
    "Pacific/Fakaofo",
    "Pacific/Fiji",
    "Pacific/Funafuti",
    "Pacific/Galapagos",
    "Pacific/Gambier",
    "Pacific/Guadalcanal",
    "Pacific/Guam",
    "Pacific/Honolulu",
    "Pacific/Johnston",
    "Pacific/Kanton",
    "Pacific/Kiritimati",
    "Pacific/Kosrae",
    "Pacific/Kwajalein",
    "Pacific/Majuro",
    "Pacific/Marquesas",
    "Pacific/Midway",
    "Pacific/Nauru",
    "Pacific/Niue",
    "Pacific/Norfolk",
    "Pacific/Noumea",
    "Pacific/Pago_Pago",
    "Pacific/Palau",
    "Pacific/Pitcairn",
    "Pacific/Pohnpei",
    "Pacific/Ponape",
    "Pacific/Port_Moresby",
    "Pacific/Rarotonga",
    "Pacific/Saipan",
    "Pacific/Samoa",
    "Pacific/Tahiti",
    "Pacific/Tarawa",
    "Pacific/Tongatapu",
    "Pacific/Truk",
    "Pacific/Wake",
    "Pacific/Wallis",
    "Pacific/Yap",
    "Poland",
    "Portugal",
    "ROC",
    "ROK",
    "Singapore",
    "Turkey",
    "UCT",
    "US/Alaska",
    "US/Aleutian",
    "US/Arizona",
    "US/Central",
    "US/East-Indiana",
    "US/Eastern",
    "US/Hawaii",
    "US/Indiana-Starke",
    "US/Michigan",
    "US/Mountain",
    "US/Pacific",
    "US/Samoa",
    "UTC",
    "Universal",
    "W-SU",
    "WET",
    "Zulu",
];
/// The keyword of an inner join, whose USING keys all take the earlier side's type, as recorded.
pub(crate) const INNER_JOIN_KEYWORD: &str = "JOIN";
/// The sequence-number column of FLATTEN and SPLIT_TO_TABLE, which no rule converts.
pub(crate) const SEQUENCE_COLUMN: &str = "SEQ";
/// Columns FLATTEN and SPLIT_TO_TABLE both bring into scope, which mark a source as one of them.
pub(crate) const SEQUENCE_PEERS: [&str; 2] = ["INDEX", "VALUE"];
/// Offsets that leave a zoned value's wall time at UTC's.
pub(crate) const ZERO_OFFSETS: &[&str] = &["+00:00", "-00:00"];
/// The suffix of the TIMESTAMP_LTZ conversion functions' names.
pub(crate) const LOCAL_ZONE_SUFFIX: &str = "_LTZ";
/// The prefix of the conversion functions that give NULL where their strict forms raise.
pub(crate) const TRY_PREFIX: &str = "TRY_";
/// Why a FROM subquery reading an enclosing query's column is refused.
pub(crate) const CORRELATED_DERIVED: &str =
    "a FROM subquery reading a column of an enclosing query, which no rule converts";
/// How a name several FROM relations hold is refused.
pub(crate) const AMBIGUOUS_SUFFIX: &str = "is ambiguous";
/// A JSON-held VARIANT as a sort key, in ARRAY_SORT's recorded order of kinds.
pub const VARIANT_ORDER_KEY: &str = "rule.variant_order_key";
