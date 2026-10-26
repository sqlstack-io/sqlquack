//! The conversion rules, each with the constructs it converts and its type conditions.

use sqlquack_core::failures::constants::{
    BOOLEAN_NOT_RECOGNIZED, DATE_FORMAT_MISMATCH, DATE_NOT_RECOGNIZED, DIVISION_BY_ZERO,
    JSON_NOT_PARSED, NOT_AN_ARRAY_ARGUMENT, NUMBER_NOT_RECOGNIZED, NUMBER_OUT_OF_RANGE,
    NUMBER_TEXT_OUT_OF_RANGE, OBJECT_KEY_EXISTS, STRING_TOO_LONG, TIMESTAMP_NOT_RECOGNIZED,
    VARIANT_NOT_CAST,
};
use sqlquack_core::failures::models::{ErrorKind, FailureMode};
use sqlquack_snowflake::typing::constants::VARIANT_COMMON_TYPE;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::models::{CallShape, RuleSpec, SpellingPair, SpellingRule};

/// HASH values differ from Snowflake's; equal inputs hash equally within one run.
pub const HASH_DIVERGENCE: &str = "hash";
/// HASH.
pub const HASH_RULE: &str = "rule.hash";
/// The digits of HASH's result, NUMBER(19,0), a signed 64-bit integer.
pub const HASH_RESULT_PRECISION: u8 = 19;
/// The divergence where DuckDB raises for a nanosecond timestamp outside its range.
pub const TIMESTAMP_RANGE: &str = "timestamp_range";
/// PERCENTILE_CONT raises where it would interpolate between two different values.
pub const PERCENTILE_INTERPOLATION: &str = "percentile_interpolation";
/// TRY_ conversions of text to DATE raise for spellings no recording shows.
pub const DATE_TEXT_SPELLING: &str = "date_text_spelling";
/// The divergence where DuckDB raises for a zoned timestamp with a sub-microsecond part.
pub const TIMESTAMP_TZ_SUBMICRO: &str = "timestamp_tz_submicro";
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
/// LEFT, RIGHT, SUBSTR and LENGTH.
pub const TEXT_POSITION: &str = "rule.text_position";
/// REPLACE and SPLIT_PART.
pub const TEXT_REPLACE: &str = "rule.text_replace";
/// TRANSLATE.
pub const TEXT_TRANSLATE: &str = "rule.text_translate";
/// LATERAL FLATTEN.
pub const FLATTEN_RULE: &str = "rule.flatten";
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
/// BITAND, BITOR and BITXOR.
pub const BITWISE: &str = "rule.bitwise";
/// TRY_TO_DATE with a literal format.
pub const DATE_FORMAT_RULE: &str = "rule.date_format";
/// The function DATE(x) converts as.
pub const TO_DATE_NAME: &str = "TO_DATE";
/// The type name of a cast to VARIANT.
pub const VARIANT_NAME: &str = "VARIANT";
/// PARSE_JSON and TRY_PARSE_JSON.
pub const PARSE_JSON_RULE: &str = "rule.parse_json";
/// Paths into a VARIANT: `v:a.b[0]`, `v[0]`, GET, GET_PATH and GET_IGNORE_CASE.
pub const VARIANT_PATH: &str = "rule.variant_path";
/// TYPEOF of a VARIANT.
pub const VARIANT_TYPE_NAME: &str = "rule.variant_typeof";
/// IS_NULL_VALUE and TO_JSON of a VARIANT.
pub const VARIANT_INSPECT: &str = "rule.variant_inspect";
/// Casts of a VARIANT to scalar types.
pub const VARIANT_CAST: &str = "rule.variant_cast";
/// The alias of the query whose VARIANT result columns are written for output.
pub const OUTPUT_ALIAS: &str = "\"_sqlquack_output\"";
/// The object constructor that keeps NULL values as JSON null.
pub const KEEP_NULL_CONSTRUCTOR: &str = "OBJECT_CONSTRUCT_KEEP_NULL";
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
/// The type of DATEDIFF.
pub const DATEDIFF_TYPE: SnowflakeType = SnowflakeType::Number {
    precision: 18,
    scale: 0,
};
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
/// Snowflake's documented abbreviations of date and time parts, with their full names.
pub const UNIT_ALIASES: &[(&str, &str)] = &[
    ("Y", "YEAR"),
    ("YY", "YEAR"),
    ("YYY", "YEAR"),
    ("YYYY", "YEAR"),
    ("YR", "YEAR"),
    ("YEARS", "YEAR"),
    ("YRS", "YEAR"),
    ("Q", "QUARTER"),
    ("QTR", "QUARTER"),
    ("QTRS", "QUARTER"),
    ("QUARTERS", "QUARTER"),
    ("MM", "MONTH"),
    ("MON", "MONTH"),
    ("MONS", "MONTH"),
    ("MONTHS", "MONTH"),
    ("W", "WEEK"),
    ("WK", "WEEK"),
    ("WEEKOFYEAR", "WEEK"),
    ("WOY", "WEEK"),
    ("WY", "WEEK"),
    ("D", "DAY"),
    ("DD", "DAY"),
    ("DAYS", "DAY"),
    ("DAYOFMONTH", "DAY"),
    ("DOW", "DAYOFWEEK"),
    ("DW", "DAYOFWEEK"),
    ("WEEKDAY", "DAYOFWEEK"),
    ("DOW_ISO", "DAYOFWEEKISO"),
    ("WEEKDAY_ISO", "DAYOFWEEKISO"),
    ("DOY", "DAYOFYEAR"),
    ("DY", "DAYOFYEAR"),
    ("YEARDAY", "DAYOFYEAR"),
    ("H", "HOUR"),
    ("HH", "HOUR"),
    ("HR", "HOUR"),
    ("HOURS", "HOUR"),
    ("HRS", "HOUR"),
    ("M", "MINUTE"),
    ("MI", "MINUTE"),
    ("MIN", "MINUTE"),
    ("MINUTES", "MINUTE"),
    ("MINS", "MINUTE"),
    ("S", "SECOND"),
    ("SEC", "SECOND"),
    ("SECONDS", "SECOND"),
    ("SECS", "SECOND"),
    ("MS", "MILLISECOND"),
    ("MSEC", "MILLISECOND"),
    ("MILLISECONDS", "MILLISECOND"),
    ("US", "MICROSECOND"),
    ("USEC", "MICROSECOND"),
    ("MICROSECONDS", "MICROSECOND"),
    ("NS", "NANOSECOND"),
    ("NSEC", "NANOSECOND"),
    ("NANOSEC", "NANOSECOND"),
    ("NANOSECONDS", "NANOSECOND"),
    ("EPOCH", "EPOCH_SECOND"),
    ("EPOCH_SECONDS", "EPOCH_SECOND"),
    ("EPOCH_MILLISECONDS", "EPOCH_MILLISECOND"),
];
/// LEAST and GREATEST.
pub const LEAST_GREATEST: &str = "rule.least_greatest";
/// STARTSWITH and ENDSWITH.
pub const TEXT_AFFIX: &str = "rule.text_affix";
/// CONTAINS.
pub const TEXT_CONTAINS: &str = "rule.contains";
/// EDITDISTANCE of two texts.
pub const EDIT_DISTANCE: &str = "rule.editdistance";
/// Casts between DATE and timestamp types.
pub const CAST_TEMPORAL: &str = "rule.cast_temporal";
/// Casts of VARCHAR to TIMESTAMP_NTZ, read as TO_TIMESTAMP_NTZ reads text.
pub const CAST_TEXT_TIMESTAMP: &str = "rule.cast_text_timestamp";
/// Casts of TIMESTAMP_TZ values known to be at +00:00 to TIMESTAMP_NTZ or DATE.
pub const CAST_UTC_ZONED: &str = "rule.cast_utc_zoned";

/// The divergence where DuckDB raises an error for a value beyond its numeric range.
pub const NUMERIC_RANGE: &str = "numeric_range";

/// SELECT with its FROM, WHERE and projection.
pub const SELECT: &str = "rule.select";
/// SELECT DISTINCT.
pub const DISTINCT: &str = "rule.distinct";
/// A table or CTE in FROM.
pub const FROM_RELATION: &str = "rule.from_relation";
/// A subquery in FROM.
pub const FROM_SUBQUERY: &str = "rule.from_subquery";
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

/// Every rule.
pub const RULES: &[RuleSpec] = &[
    RuleSpec {
        id: SELECT,
        constructs: &["node:select", "clause:with"],
        conditions: "Every result column has a name: a column, or an alias; columns differing only in case are refused.",
        failures: &[],
    },
    RuleSpec {
        id: DISTINCT,
        constructs: &["node:select"],
        conditions: "DISTINCT over the converted columns.",
        failures: &[],
    },
    RuleSpec {
        id: FROM_RELATION,
        constructs: &["node:table"],
        conditions: "The relation is a CTE or has typed columns in the schema.",
        failures: &[],
    },
    RuleSpec {
        id: FROM_SUBQUERY,
        constructs: &["node:subquery"],
        conditions: "The subquery converts.",
        failures: &[],
    },
    RuleSpec {
        id: COMMON_TABLE,
        constructs: &["clause:with"],
        conditions: "Not recursive; no column list.",
        failures: &[],
    },
    RuleSpec {
        id: JOIN_ON,
        constructs: &["join:inner", "join:left", "join:right", "join:full"],
        conditions: "An ON condition of type BOOLEAN; no USING.",
        failures: &[],
    },
    RuleSpec {
        id: JOIN_CROSS,
        constructs: &["join:cross", "join:implicit"],
        conditions: "No condition.",
        failures: &[],
    },
    RuleSpec {
        id: WHERE,
        constructs: &["clause:where"],
        conditions: "A BOOLEAN condition.",
        failures: &[],
    },
    RuleSpec {
        id: GROUP_BY,
        constructs: &["clause:group_by"],
        conditions: "Positions or expressions over FROM columns; no GROUP BY ALL, ROLLUP, CUBE or GROUPING SETS.",
        failures: &[],
    },
    RuleSpec {
        id: HAVING,
        constructs: &["clause:having"],
        conditions: "A BOOLEAN condition.",
        failures: &[],
    },
    RuleSpec {
        id: QUALIFY,
        constructs: &["clause:qualify"],
        conditions: "A BOOLEAN condition.",
        failures: &[],
    },
    RuleSpec {
        id: ORDER_BY,
        constructs: &["clause:order_by"],
        conditions: "NULL placement is spelled out as Snowflake's default: last ascending, first descending. An unqualified name that is both a select alias and a FROM column means the alias, bare or in an expression (batch 11); in a grouped query or beside an aggregate, window or subquery it is refused. HAVING and QUALIFY read the FROM column (batch 11).",
        failures: &[],
    },
    RuleSpec {
        id: LIMIT,
        constructs: &["clause:limit"],
        conditions: "Integer literals.",
        failures: &[],
    },
    RuleSpec {
        id: STAR,
        constructs: &["node:star"],
        conditions: "No EXCLUDE, REPLACE or RENAME; expands to the known columns.",
        failures: &[],
    },
    RuleSpec {
        id: COLUMN,
        constructs: &["node:column"],
        conditions: "The name resolves to exactly one column.",
        failures: &[],
    },
    RuleSpec {
        id: ALIAS,
        constructs: &["node:alias"],
        conditions: "A single name.",
        failures: &[],
    },
    RuleSpec {
        id: UNION_BY_NAME,
        constructs: &["clause:union_by_name"],
        conditions: "UNION or UNION ALL BY NAME: columns matched by resolved name, the left branch's first, then the right's new ones; a branch lacking a column fills NULL; names repeating or differing only in case are refused.",
        failures: &[],
    },
    RuleSpec {
        id: SET_OPERATION,
        constructs: &["node:union", "node:except", "node:intersect"],
        conditions: "Branches with the same column count whose types unify; differing branches are cast to the unified type.",
        failures: &[FailureMode {
            kind: ErrorKind::NumericOverflow,
            snowflake: &[NUMBER_OUT_OF_RANGE],
        }],
    },
    RuleSpec {
        id: IN_LIST,
        constructs: &["node:in"],
        conditions: "Every item compares with the operand without implicit conversion.",
        failures: &[],
    },
    RuleSpec {
        id: IN_SUBQUERY,
        constructs: &["node:in"],
        conditions: "IN or NOT IN; one column comparable with the operand.",
        failures: &[],
    },
    RuleSpec {
        id: NOT_IN_SUBQUERY,
        constructs: &["node:in"],
        conditions: "NOT IN with a one-column subquery, a set operation only when every branch has a FROM (batches 9 and 10 recorded NOT IN (SELECT 1 UNION ALL SELECT NULL) keeping rows, as if the NULL were absent, while branches reading relations and derived tables follow SQL's rules), in a WHERE condition outside any call (in a projection Snowflake raises error 2031, as recorded): NULL when the operand is NULL or the subquery holds a NULL and no match, false on a match, true otherwise; true for an empty subquery.",
        failures: &[],
    },
    RuleSpec {
        id: EXISTS,
        constructs: &["node:exists"],
        conditions: "The subquery converts.",
        failures: &[],
    },
    RuleSpec {
        id: SCALAR_SUBQUERY,
        constructs: &["node:subquery"],
        conditions: "Exactly one column.",
        failures: &[],
    },
    RuleSpec {
        id: NUMBER_LITERAL,
        constructs: &["node:literal"],
        conditions: "Plain digits with an optional fraction that keeps a non-zero digit; typed NUMBER(digits, fraction digits) with trailing zeros dropped.",
        failures: &[],
    },
    RuleSpec {
        id: CONSTANT_LITERAL,
        constructs: &["node:literal", "node:boolean", "node:null"],
        conditions: "String, TRUE, FALSE or NULL.",
        failures: &[],
    },
    RuleSpec {
        id: PARENTHESES,
        constructs: &["node:paren"],
        conditions: "None.",
        failures: &[],
    },
    RuleSpec {
        id: COMMENT,
        constructs: &["node:annotated"],
        conditions: "None; a comment never changes a value, so the wrapped expression converts alone and the comment is dropped.",
        failures: &[],
    },
    RuleSpec {
        id: NEGATE,
        constructs: &["node:neg"],
        conditions: "A NUMBER or FLOAT operand; the type is kept.",
        failures: &[],
    },
    RuleSpec {
        id: DECIMAL_ARITHMETIC,
        constructs: &["node:add", "node:sub", "node:mul"],
        conditions: "NUMBER operands; `+`, `-` and `*` whose exact result type needs more than 38 digits rely on `numeric_range`; `*` refuses when a scale cut would need more than 38 intermediate digits.",
        failures: &[FailureMode {
            kind: ErrorKind::NumericOverflow,
            snowflake: &[NUMBER_OUT_OF_RANGE],
        }],
    },
    RuleSpec {
        id: DECIMAL_DIVISION,
        constructs: &["node:div"],
        conditions: "NUMBER operands whose divisor precision plus the scale shift stays within 38 digits.",
        failures: &[
            FailureMode {
                kind: ErrorKind::NumericOverflow,
                snowflake: &[NUMBER_OUT_OF_RANGE],
            },
            FailureMode {
                kind: ErrorKind::DivisionByZero,
                snowflake: &[DIVISION_BY_ZERO],
            },
        ],
    },
    RuleSpec {
        id: FLOAT_ARITHMETIC,
        constructs: &["node:add", "node:sub", "node:mul", "node:div"],
        conditions: "A FLOAT and a FLOAT or NUMBER operand.",
        failures: &[FailureMode {
            kind: ErrorKind::DivisionByZero,
            snowflake: &[DIVISION_BY_ZERO],
        }],
    },
    RuleSpec {
        id: COMPARISON,
        constructs: &[
            "node:eq", "node:neq", "node:lt", "node:lte", "node:gt", "node:gte",
        ],
        conditions: "Both NUMBER or FLOAT (compared as doubles when one is FLOAT), both VARCHAR (bytewise), both BOOLEAN, both DATE, both TIMESTAMP_NTZ (any precisions) or both zoned (TIMESTAMP_TZ or TIMESTAMP_LTZ, compared as instants). NUMBERs whose common type needs more than 38 digits rely on numeric_range; Snowflake also raises 100046 for values beyond it, as recorded.",
        failures: &[FailureMode {
            kind: ErrorKind::NumericOverflow,
            snowflake: &[NUMBER_OUT_OF_RANGE],
        }],
    },
    RuleSpec {
        id: DATE_LITERAL_COMPARISON,
        constructs: &[
            "node:eq", "node:neq", "node:lt", "node:lte", "node:gt", "node:gte",
        ],
        conditions: "A DATE and a string literal in YYYY-MM-DD form.",
        failures: &[],
    },
    RuleSpec {
        id: LOGIC,
        constructs: &["node:and", "node:or", "node:not"],
        conditions: "BOOLEAN operands.",
        failures: &[],
    },
    RuleSpec {
        id: IS_NULL,
        constructs: &["node:is_null"],
        conditions: "None.",
        failures: &[],
    },
    RuleSpec {
        id: BETWEEN,
        constructs: &["node:between"],
        conditions: "Bounds of the operand's type family, as for comparisons, without FLOAT and NUMBER mixed.",
        failures: &[],
    },
    RuleSpec {
        id: CASE,
        constructs: &["node:case"],
        conditions: "BOOLEAN conditions, or a simple CASE whose values compare with the operand; results unify.",
        failures: &[],
    },
    RuleSpec {
        id: COALESCE,
        constructs: &["fn:COALESCE", "fn:IFNULL", "fn:NVL"],
        conditions: "Arguments unify.",
        failures: &[],
    },
    RuleSpec {
        id: IFF,
        constructs: &["fn:IFF"],
        conditions: "A BOOLEAN condition; branches unify.",
        failures: &[],
    },
    RuleSpec {
        id: NULLIF,
        constructs: &["fn:NULLIF"],
        conditions: "Arguments unify to a comparable type.",
        failures: &[],
    },
    RuleSpec {
        id: CAST_NUMERIC,
        constructs: &["node:cast"],
        conditions: "NUMBER to NUMBER or FLOAT; FLOAT to NUMBER of scale 0; not TRY_CAST.",
        failures: &[FailureMode {
            kind: ErrorKind::NumericOverflow,
            snowflake: &[NUMBER_OUT_OF_RANGE],
        }],
    },
    RuleSpec {
        id: CAST_TO_TEXT,
        constructs: &["node:cast"],
        conditions: "From NUMBER, BOOLEAN, DATE or VARCHAR; a length is enforced by raising an error; not TRY_CAST unless from VARCHAR.",
        failures: &[FailureMode {
            kind: ErrorKind::StringTooLong,
            snowflake: &[STRING_TOO_LONG],
        }],
    },
    RuleSpec {
        id: CAST_BOOLEAN,
        constructs: &["node:cast"],
        conditions: "BOOLEAN to NUMBER, NUMBER to BOOLEAN; not TRY_CAST.",
        failures: &[],
    },
    RuleSpec {
        id: CAST_PARSED_TEXT,
        constructs: &["node:cast", "node:try_cast"],
        conditions: "VARCHAR to NUMBER, FLOAT or BOOLEAN; only recorded spellings convert, around which spaces, tabs and line feeds are ignored: decimal text with optional sign, point and exponent (an exponent marker without digits counts as exponent 0, an exponent without mantissa as zero); nan, -nan, inf, +inf, -inf, infinity, -infinity in any case; true/false, t/f, yes/no, y/n, on/off, 1/0 in any case. Other text fails, or is NULL under TRY_CAST; FLOAT overflow is infinite under CAST and NULL under TRY_CAST. VARCHAR to DATE reads the spellings TO_DATE reads, and fails for others; TRY_CAST to DATE is NULL for empty text, impossible values and the spellings recorded returning NULL, and raises for other spellings, relying on date_text_spelling.",
        failures: &[
            FailureMode {
                kind: ErrorKind::NumberText,
                snowflake: &[NUMBER_NOT_RECOGNIZED],
            },
            FailureMode {
                kind: ErrorKind::BooleanText,
                snowflake: &[BOOLEAN_NOT_RECOGNIZED],
            },
            FailureMode {
                kind: ErrorKind::DateText,
                snowflake: &[DATE_NOT_RECOGNIZED],
            },
            FailureMode {
                kind: ErrorKind::TimestampText,
                snowflake: &[TIMESTAMP_NOT_RECOGNIZED],
            },
            FailureMode {
                kind: ErrorKind::NumericOverflow,
                snowflake: &[NUMBER_OUT_OF_RANGE, NUMBER_TEXT_OUT_OF_RANGE],
            },
        ],
    },
    RuleSpec {
        id: CAST_TEXT_LITERAL,
        constructs: &["node:cast", "node:try_cast"],
        conditions: "A string literal in a form both engines read the same: plain decimal for NUMBER, decimal with exponent for FLOAT, true or false for BOOLEAN, YYYY-MM-DD for DATE; YYYY-MM-DD[ HH:MI:SS[.f]] for timestamps, with a ' ±HH:MM' offset for TIMESTAMP_TZ and TIMESTAMP_LTZ, and no more fractional digits than the target precision. An NTZ(7-9) literal outside 1677-09-22..2262-04-10 relies on timestamp_range; a zoned literal with a non-zero sub-microsecond part relies on timestamp_tz_submicro and raises.",
        failures: &[FailureMode {
            kind: ErrorKind::StringTooLong,
            snowflake: &[STRING_TOO_LONG],
        }],
    },
    RuleSpec {
        id: DATE_PART_RULE,
        constructs: &[
            "fn:DATE_PART",
            "node:extract",
            "fn:YEAR",
            "fn:QUARTER",
            "fn:MONTH",
            "fn:DAY",
            "fn:HOUR",
            "fn:MINUTE",
            "fn:SECOND",
        ],
        conditions: "A DATE or TIMESTAMP_NTZ value under WEEK_START 0 and WEEK_OF_YEAR_POLICY 0: YEAR, QUARTER, MONTH, WEEK and WEEKISO (ISO weeks), DAY, DAYOFWEEK (Sunday 0), DAYOFWEEKISO, DAYOFYEAR, YEAROFWEEK and YEAROFWEEKISO (ISO years), EPOCH_SECOND and EPOCH_MILLISECOND (toward the past), and for timestamps HOUR, MINUTE, SECOND and NANOSECOND; typed NUMBER(p,0) as recorded.",
        failures: &[],
    },
    RuleSpec {
        id: DATE_ARITHMETIC,
        constructs: &[
            "fn:DATE_TRUNC",
            "fn:DATEADD",
            "fn:TIMESTAMPADD",
            "fn:DATEDIFF",
            "fn:TIMEDIFF",
            "fn:TIMESTAMPDIFF",
        ],
        conditions: "DATE or TIMESTAMP_NTZ values. DATE_TRUNC to YEAR, QUARTER, MONTH, WEEK (Monday) or DAY, and for timestamps HOUR, MINUTE or SECOND, keeping the type. DATEADD of a NUMBER(p,0) amount of YEAR, QUARTER, MONTH (clamped to the month's end), WEEK or DAY, and for timestamps HOUR, MINUTE or SECOND, keeping the type and its nanoseconds; precision 7-9 relies on timestamp_range. DATEDIFF between two DATEs or two timestamps counts YEAR, QUARTER, MONTH and DAY boundaries of the dates, and for timestamps HOUR, MINUTE, SECOND, MILLISECOND or MICROSECOND boundaries toward the past.",
        failures: &[],
    },
    RuleSpec {
        id: TIME_ZONE_CONVERSION,
        constructs: &["fn:CONVERT_TIMEZONE"],
        conditions: "Literal IANA zone names. Three arguments on TIMESTAMP_NTZ, giving the wall time in the target zone at the same precision, a repeated source wall time read as its earlier instant and a skipped one with the offset before the transition; two arguments on any timestamp, giving the same instant as TIMESTAMP_TZ, with the target zone checked. Precision 7-9 relies on timestamp_range or timestamp_tz_submicro.",
        failures: &[],
    },
    RuleSpec {
        id: PARSE_JSON_RULE,
        constructs: &["fn:PARSE_JSON", "fn:TRY_PARSE_JSON"],
        conditions: "One VARCHAR argument. Empty or blank text is NULL; text DuckDB does not read as JSON, numbers with an exponent, an all-zero fraction, six leading fraction zeros, more than 15 decimal digits or 39 integer digits, NaN, Infinity, undefined, trailing commas, surrogate escapes and repeated keys raise, relying on json_fidelity; for TRY_PARSE_JSON too. A VARIANT column of a base table is guarded the same way.",
        failures: &[FailureMode {
            kind: ErrorKind::JsonParse,
            snowflake: &[JSON_NOT_PARSED],
        }],
    },
    RuleSpec {
        id: VARIANT_PATH,
        constructs: &[
            "node:j_s_o_n_extract",
            "node:subscript",
            "fn:GET",
            "fn:GET_PATH",
            "fn:GET_IGNORE_CASE",
        ],
        conditions: "A VARIANT and a literal path of keys and whole-number indexes, keys without quotes or backslashes; a missing step is SQL NULL. GET_IGNORE_CASE prefers the exact key, else the one ASCII-case match; several matches or non-ASCII keys raise, relying on json_fidelity.",
        failures: &[],
    },
    RuleSpec {
        id: VARIANT_TYPE_NAME,
        constructs: &["fn:TYPEOF"],
        conditions: "A VARIANT: INTEGER, DECIMAL, VARCHAR, BOOLEAN, OBJECT, ARRAY or NULL_VALUE.",
        failures: &[],
    },
    RuleSpec {
        id: VARIANT_INSPECT,
        constructs: &["fn:IS_NULL_VALUE", "fn:TO_JSON"],
        conditions: "A VARIANT. TO_JSON of an object whose keys are not in byte order raises, relying on json_fidelity.",
        failures: &[],
    },
    RuleSpec {
        id: PROJECT_FUNCTION_CALL,
        constructs: &["fn:<udf>"],
        conditions: "A project function the schema declares, with as many arguments as parameters: each argument of the parameter's type, NULL, or a NUMBER widened to a NUMBER or FLOAT parameter; the result cast to the declared type. A SQL function declared to return NUMBER converts only when its body is of exactly that type, since batch 9 recorded Snowflake returning the body's type and value uncast. Python functions take and return only VARCHAR, NUMBER, FLOAT and BOOLEAN.",
        failures: &[],
    },
    RuleSpec {
        id: PROJECT_TABLE_FUNCTION,
        constructs: &["fn:TABLE"],
        conditions: "TABLE(f(...)) of a project table function the schema declares, each argument of exactly its parameter's type and free of column references (batch 10 recorded error 1044 for a NUMBER(2,0) literal to a NUMBER(10,2) parameter); each column cast to its declared type.",
        failures: &[],
    },
    RuleSpec {
        id: OBJECT_PICK_RULE,
        constructs: &["fn:OBJECT_PICK"],
        conditions: "A VARIANT and literal text keys, or one VARIANT array of strings: keys matched exactly and case-sensitively, missing keys ignored, keys kept in byte order; SQL NULL and JSON null objects give SQL NULL, other non-objects raise; an array holding other than strings raises, relying on json_fidelity.",
        failures: &[FailureMode {
            kind: ErrorKind::VariantCast,
            snowflake: &[VARIANT_NOT_CAST],
        }],
    },
    RuleSpec {
        id: OBJECT_KEYS_RULE,
        constructs: &["fn:OBJECT_KEYS"],
        conditions: "A VARIANT: an object's keys as an array in byte order; SQL NULL and JSON null give SQL NULL, other non-objects raise.",
        failures: &[FailureMode {
            kind: ErrorKind::VariantCast,
            snowflake: &[VARIANT_NOT_CAST],
        }],
    },
    RuleSpec {
        id: ARRAY_AGG_RULE,
        constructs: &["fn:ARRAY_AGG", "node:within_group"],
        conditions: "ARRAY_AGG WITHIN GROUP (ORDER BY ...), not as a window function: the non-NULL values in order as an ARRAY, values written as TO_VARIANT writes them and a JSON null kept; an empty ARRAY when no value is non-NULL; DISTINCT only of text ordered by itself.",
        failures: &[],
    },
    RuleSpec {
        id: ARRAY_SIZE_RULE,
        constructs: &["fn:ARRAY_SIZE"],
        conditions: "A VARIANT or ARRAY: an array's element count as NUMBER(9,0), as recorded; SQL NULL, JSON null and values other than arrays give NULL, as Snowflake documents.",
        failures: &[],
    },
    RuleSpec {
        id: ARRAY_CONSTRUCT_COMPACT_RULE,
        constructs: &["fn:ARRAY_CONSTRUCT_COMPACT"],
        conditions: "Values as for rule.array_construct: an ARRAY of the non-NULL values in order; a JSON null element raises, relying on json_fidelity, until a recording shows whether Snowflake drops it.",
        failures: &[],
    },
    RuleSpec {
        id: ARRAY_CONTAINS_RULE,
        constructs: &["fn:ARRAY_CONTAINS"],
        conditions: "A VARIANT value and a VARIANT or ARRAY: whether an element equals the value, strings, integers, booleans and JSON null compared as JSON text; SQL NULL arguments give NULL; a decimal or nested value or element, or a value other than an array, raises, relying on json_fidelity.",
        failures: &[],
    },
    RuleSpec {
        id: ARRAY_TO_STRING_RULE,
        constructs: &["fn:ARRAY_TO_STRING"],
        conditions: "A VARIANT or ARRAY and a VARCHAR separator: strings joined as their text, numbers, booleans, objects and arrays as JSON; an empty array gives empty text, SQL NULL and JSON null give NULL; a JSON null element raises 100071 and a value other than an array 100098, as recorded; objects with keys out of byte order raise, relying on json_fidelity.",
        failures: &[
            FailureMode {
                kind: ErrorKind::VariantCast,
                snowflake: &[VARIANT_NOT_CAST],
            },
            FailureMode {
                kind: ErrorKind::InvalidArgument,
                snowflake: &[NOT_AN_ARRAY_ARGUMENT],
            },
        ],
    },
    RuleSpec {
        id: SPLIT_RULE,
        constructs: &["fn:SPLIT"],
        conditions: "VARCHAR text and separator: an ARRAY of the parts between occurrences of the separator, empty parts kept; an empty separator keeps the text whole; NULL arguments give NULL.",
        failures: &[],
    },
    RuleSpec {
        id: ARRAY_CONSTRUCT_RULE,
        constructs: &["fn:ARRAY_CONSTRUCT"],
        conditions: "VARIANT, text, boolean or NUMBER(p,0) elements; a SQL NULL element, which Snowflake keeps as undefined, raises, relying on json_fidelity. NUMBER elements rely on json_fidelity.",
        failures: &[],
    },
    RuleSpec {
        id: ARRAY_EXCEPT_RULE,
        constructs: &["fn:ARRAY_EXCEPT"],
        conditions: "Two VARIANT arrays of strings: each element of the second removes one equal element of the first, order kept; SQL NULL gives NULL; non-arrays (which Snowflake wraps) and other elements, JSON null included, raise relying on json_fidelity.",
        failures: &[],
    },
    RuleSpec {
        id: TO_VARIANT_RULE,
        constructs: &["fn:TO_VARIANT", "node:cast"],
        conditions: "TO_VARIANT or a cast to VARIANT of text, a boolean or NUMBER(p,0), stored as the JSON string, boolean or number, NUMBER relying on json_fidelity; of a DATE, TIMESTAMP_NTZ or TIMESTAMP_LTZ, stored as the JSON string Snowflake writes for it (YYYY-MM-DD, YYYY-MM-DD HH:MI:SS.FFF, and ' Z' for LTZ), relying on variant_typed_values; values with sub-millisecond digits, or an LTZ outside UTC, raise. Other types are refused.",
        failures: &[],
    },
    RuleSpec {
        id: VARIANT_TYPED_CHECK,
        constructs: &[
            "fn:IS_DATE",
            "fn:IS_TIME",
            "fn:IS_TIMESTAMP_NTZ",
            "fn:IS_TIMESTAMP_LTZ",
            "fn:IS_TIMESTAMP_TZ",
            "fn:IS_BINARY",
        ],
        conditions: "A VARIANT: false, as SQLQuack's VARIANTs hold no typed values; a string in the VARIANT text of a date or a millisecond timestamp raises, relying on variant_typed_values, since it may be one in Snowflake. SQL NULL stays NULL.",
        failures: &[],
    },
    RuleSpec {
        id: VARIANT_KIND_RULE,
        constructs: &["fn:IS_OBJECT", "fn:IS_ARRAY", "fn:IS_BOOLEAN"],
        conditions: "A VARIANT: whether it holds an object, an array or a boolean; JSON null is false and SQL NULL stays NULL.",
        failures: &[],
    },
    RuleSpec {
        id: VARIANT_KIND_CAST,
        constructs: &["node:cast"],
        conditions: "A VARIANT cast to OBJECT or ARRAY: a value of that kind unchanged, SQL NULL and JSON null give SQL NULL; another value raises for OBJECT and becomes a one-element array for ARRAY, as batch 9 recorded.",
        failures: &[FailureMode {
            kind: ErrorKind::VariantCast,
            snowflake: &[VARIANT_NOT_CAST],
        }],
    },
    RuleSpec {
        id: OBJECT_DELETE_RULE,
        constructs: &["fn:OBJECT_DELETE"],
        conditions: "A VARIANT and literal text keys, matched exactly and case-sensitively, never read as paths: missing keys leave the object unchanged, SQL NULL and JSON null give SQL NULL, and other non-objects raise.",
        failures: &[FailureMode {
            kind: ErrorKind::VariantCast,
            snowflake: &[VARIANT_NOT_CAST],
        }],
    },
    RuleSpec {
        id: OBJECT_INSERT_RULE,
        constructs: &["fn:OBJECT_INSERT"],
        conditions: "A VARIANT, a literal text key, a VARIANT, text, boolean or NUMBER(p,0) value and an optional literal update flag: a SQL NULL value leaves the object unchanged, JSON null is inserted, an existing key raises unless updating, keys stay in byte order; SQL NULL and JSON null objects give SQL NULL, other non-objects raise. NUMBER values rely on json_fidelity.",
        failures: &[
            FailureMode {
                kind: ErrorKind::VariantCast,
                snowflake: &[VARIANT_NOT_CAST],
            },
            FailureMode {
                kind: ErrorKind::InvalidArgument,
                snowflake: &[OBJECT_KEY_EXISTS],
            },
        ],
    },
    RuleSpec {
        id: OBJECT_CONSTRUCT_RULE,
        constructs: &["fn:OBJECT_CONSTRUCT", "fn:OBJECT_CONSTRUCT_KEEP_NULL"],
        conditions: "Text keys and VARIANT, text, boolean or NUMBER(p,0) values: NULL keys are skipped, SQL NULL values are skipped (kept as JSON null by _KEEP_NULL), repeated keys raise, keys are in byte order. Other value types are refused: Snowflake keeps them typed inside the VARIANT, and no recording yet shows TYPEOF of a decimal there. NUMBER values rely on json_fidelity.",
        failures: &[FailureMode {
            kind: ErrorKind::InvalidArgument,
            snowflake: &[OBJECT_KEY_EXISTS],
        }],
    },
    RuleSpec {
        id: VARIANT_CAST,
        constructs: &["node:cast"],
        conditions: "A VARIANT to VARIANT, VARCHAR, VARCHAR(n), NUMBER, FLOAT, BOOLEAN, DATE or TIMESTAMP_NTZ(9): strings and numbers read as their text, text longer than VARCHAR(n) raises, JSON null is NULL; objects, arrays and other kinds raise, as do objects written as text with keys out of byte order, relying on json_fidelity.",
        failures: &[
            FailureMode {
                kind: ErrorKind::VariantCast,
                snowflake: &[VARIANT_NOT_CAST],
            },
            FailureMode {
                kind: ErrorKind::NumericOverflow,
                snowflake: &[NUMBER_OUT_OF_RANGE],
            },
            FailureMode {
                kind: ErrorKind::StringTooLong,
                snowflake: &[STRING_TOO_LONG],
            },
        ],
    },
    RuleSpec {
        id: DECIMAL_EPOCH_RULE,
        constructs: &["fn:TO_TIMESTAMP", "fn:TO_TIMESTAMP_NTZ"],
        conditions: "A NUMBER(p,s) with 0 < s <= 9 and no scale argument: seconds since the epoch with a fraction, typed TIMESTAMP_NTZ(s), as Snowflake documents; scales above 6 rely on timestamp_range.",
        failures: &[],
    },
    RuleSpec {
        id: TEXT_PAD,
        constructs: &["fn:LPAD", "fn:RPAD"],
        conditions: "VARCHAR text and pad (a space by default) and an integer NUMBER length, counted in characters: shorter text is padded, longer text cut to the length; an empty pad only cuts; a length of zero or less gives empty text; NULL arguments give NULL.",
        failures: &[],
    },
    RuleSpec {
        id: NULL_SAFE_COMPARISON,
        constructs: &["node:null_safe_eq", "node:null_safe_neq", "fn:EQUAL_NULL"],
        conditions: "Operands rule.comparison compares: NULL is equal to NULL and unequal to any value; never NULL.",
        failures: &[],
    },
    RuleSpec {
        id: NUMERIC_FUNCTION,
        constructs: &["fn:ABS"],
        conditions: "A NUMBER or FLOAT, the result of the same type.",
        failures: &[],
    },
    RuleSpec {
        id: HASH_RULE,
        constructs: &["fn:HASH"],
        conditions: "One or more VARCHAR, integer NUMBER, BOOLEAN, DATE or VARIANT values, or NULL, relying on the known divergence hash: a signed 64-bit NUMBER(19,0) that is deterministic and equal for equal arguments in the same order within one run, never Snowflake's value. Integer NUMBERs hash by value whatever their precision; a VARIANT hashes by its JSON text with keys in byte order, raising where DuckDB keeps another order (json_fidelity). NUMBERs with a scale, FLOAT and timestamps are refused.",
        failures: &[],
    },
    RuleSpec {
        id: DECODE_RULE,
        constructs: &["fn:DECODE"],
        conditions: "An expression compared with each search as rule.comparison compares them, NULL matching NULL; the first matching result, else the default or NULL; results and default unify.",
        failures: &[],
    },
    RuleSpec {
        id: ZERO_IF_NULL_RULE,
        constructs: &["fn:ZEROIFNULL"],
        conditions: "A NUMBER or FLOAT: the value, or zero where it is NULL, of the value's type.",
        failures: &[],
    },
    RuleSpec {
        id: ROUNDING,
        constructs: &["fn:ROUND", "fn:CEIL", "fn:FLOOR", "fn:TRUNC", "fn:TRUNCATE"],
        conditions: "A NUMBER or FLOAT; ROUND to literal places (0 by default, possibly negative), half away from zero; CEIL, FLOOR and TRUNC to whole numbers. A NUMBER(p,s) whose scale drops to d is NUMBER(min(38, p + 1), max(d, 0)), else its own type, as recorded; a FLOAT stays FLOAT, a zero result positive. A result beyond NUMBER(38,0) raises, as Snowflake does.",
        failures: &[FailureMode {
            kind: ErrorKind::NumericOverflow,
            snowflake: &[NUMBER_OUT_OF_RANGE],
        }],
    },
    RuleSpec {
        id: SIGN_RULE,
        constructs: &["fn:SIGN"],
        conditions: "A NUMBER, giving -1, 0 or 1 as NUMBER(2,0), or a FLOAT, giving a FLOAT.",
        failures: &[],
    },
    RuleSpec {
        id: BITWISE,
        constructs: &["fn:BITAND", "fn:BITOR", "fn:BITXOR"],
        conditions: "Two integer NUMBERs combined as 128-bit two's complement, typed as recorded: NUMBER(p,0) of the wider operand for BITAND and BITXOR, NUMBER(p+1,0) for BITOR; NULL gives NULL.",
        failures: &[],
    },
    RuleSpec {
        id: DATE_FORMAT_RULE,
        constructs: &["fn:TO_DATE", "fn:TRY_TO_DATE"],
        conditions: "TO_DATE or TRY_TO_DATE of text with a literal format YYYYMMDD, YYYY-MM-DD, YYYY/MM/DD, DD.MM.YYYY, DD.MM.YY, DD/MM/YYYY, MM/DD/YYYY, YY-MM-DD or DD-MM-YY, in any case: text spelled exactly in the format's digits and separators is its date; an impossible date is NULL under TRY_TO_DATE and raises 100097 under TO_DATE, as recorded; two-digit years read from 1970 (00-69 as 20xx); NULL is NULL, and empty text NULL under TRY_TO_DATE. Any other text raises, relying on date_text_spelling.",
        failures: &[FailureMode {
            kind: ErrorKind::DateText,
            snowflake: &[DATE_FORMAT_MISMATCH],
        }],
    },
    RuleSpec {
        id: TEXT_TO_TIMESTAMP_RULE,
        constructs: &[
            "fn:DATE",
            "fn:TO_TIMESTAMP",
            "fn:TO_TIMESTAMP_NTZ",
            "fn:TRY_TO_TIMESTAMP",
            "fn:TRY_TO_TIMESTAMP_NTZ",
            "fn:TO_DATE",
            "fn:TRY_TO_DATE",
        ],
        conditions: "Text, without a format, in the recorded spellings, around which spaces, tabs and line feeds are ignored: YYYY-MM-DD with an optional ' ' or 'T' and HH:MI:SS[.f(1-9)], and an optional Z or ±HH:MM that is ignored; or, for timestamps, non-negative integer text read as seconds, milliseconds, microseconds or nanoseconds by magnitude. Dates also read MM/DD/YYYY, DD-Mon-YY(YY) in title or upper case (two-digit years as written), unpadded fields, RFC 2822 at +0000 and epoch integer text. Other text raises. TRY_TO_DATE is NULL for empty text, impossible values and the spellings recorded returning NULL, and raises for other spellings, relying on date_text_spelling; the other TRY_ forms are refused, since a NULL could hide a value. Timestamps are TIMESTAMP_NTZ(9) and rely on timestamp_range. TO_TIMESTAMP of a NUMBER(p,0) counts seconds, or units of a literal scale, typed TIMESTAMP_NTZ(scale).",
        failures: &[
            FailureMode {
                kind: ErrorKind::DateText,
                snowflake: &[DATE_NOT_RECOGNIZED],
            },
            FailureMode {
                kind: ErrorKind::TimestampText,
                snowflake: &[TIMESTAMP_NOT_RECOGNIZED],
            },
        ],
    },
    RuleSpec {
        id: TEXT_POSITION,
        constructs: &[
            "fn:LEFT",
            "fn:RIGHT",
            "fn:SUBSTR",
            "fn:SUBSTRING",
            "fn:LENGTH",
            "fn:LEN",
        ],
        conditions: "VARCHAR text and NUMBER(p,0) positions, counted in characters. LEFT and RIGHT give the empty string for a negative length; SUBSTR counts start 0 as 1 and negative starts from the end, with a length only as a non-negative integer literal; LENGTH is NUMBER(18,0).",
        failures: &[],
    },
    RuleSpec {
        id: FLATTEN_RULE,
        constructs: &["node:lateral", "fn:FLATTEN"],
        conditions: "LATERAL FLATTEN of a VARIANT input after another FROM source, with OUTER => a boolean literal at most: INDEX and PATH [i] for array elements, KEY and PATH for object keys (raising for keys that are not plain names), VALUE and THIS; scalars, JSON null and NULL give no rows, or one row of NULLs under OUTER => TRUE. SEQ, PATH =>, RECURSIVE => and MODE => are not converted.",
        failures: &[],
    },
    RuleSpec {
        id: LISTAGG_RULE,
        constructs: &["node:within_group", "fn:LISTAGG"],
        conditions: "LISTAGG of VARCHAR WITHIN GROUP (ORDER BY ...), with a literal delimiter or none; NULL values are skipped and no non-NULL value gives the empty string; DISTINCT only ordered by the value itself. Alone or OVER (PARTITION BY ...).",
        failures: &[],
    },
    RuleSpec {
        id: PERCENTILE_RULE,
        constructs: &[
            "node:within_group",
            "fn:PERCENTILE_CONT",
            "fn:PERCENTILE_DISC",
        ],
        conditions: "One NUMBER key and a literal fraction from 0 to 1 in thousandths. PERCENTILE_CONT of NUMBER(p,s) with p at most 35 is NUMBER(p+3,s+3) and exact where the fraction lands on a value; between two different values it raises, relying on percentile_interpolation, since Snowflake interpolates with rounding error in the last digits; PERCENTILE_DISC keeps the key's type, ascending only. Alone or OVER (PARTITION BY ...).",
        failures: &[],
    },
    RuleSpec {
        id: TEXT_TRANSLATE,
        constructs: &["fn:TRANSLATE"],
        conditions: "A VARCHAR text and literal character lists: the source non-empty without repeated characters, the target no longer than the source; source characters past the target's end are removed.",
        failures: &[],
    },
    RuleSpec {
        id: TEXT_REPLACE,
        constructs: &["fn:REPLACE", "fn:SPLIT_PART"],
        conditions: "VARCHAR operands. REPLACE without a replacement removes matches, and an empty search changes nothing. SPLIT_PART takes a literal delimiter (when empty, only part 1, which is the whole text); part 0 counts as 1, negative parts count from the end, and a part past the end is empty.",
        failures: &[],
    },
    RuleSpec {
        id: LIKE,
        constructs: &["node:like", "node:i_like"],
        conditions: "VARCHAR operands, case-sensitive for LIKE, with no default escape character; ESCAPE as given. ILIKE only with an ASCII literal pattern. Not LIKE ANY or LIKE ALL.",
        failures: &[],
    },
    RuleSpec {
        id: REGEXP,
        constructs: &[
            "fn:REGEXP_LIKE",
            "fn:REGEXP_REPLACE",
            "fn:REGEXP_SUBSTR",
            "node:regexp_like",
        ],
        conditions: "A literal pattern without lookarounds, group modifiers, lazy quantifiers, POSIX classes or escapes other than \\d \\s \\w (ASCII, as recorded), their negations and escaped punctuation. REGEXP_LIKE matches the whole text; REGEXP_REPLACE replaces every match from position 1, case-sensitively or with 'i', with group references only; REGEXP_SUBSTR from position 1, any occurrence, with 'e' and a group, NULL when there is no match.",
        failures: &[],
    },
    RuleSpec {
        id: REGEXP_PARAMETERS,
        constructs: &["fn:REGEXP_LIKE"],
        conditions: "REGEXP_LIKE with a literal parameter string of c, i and s, as RE2 inline flags: i matches case-insensitively, c case-sensitively (the last wins), s lets . match a line feed.",
        failures: &[],
    },
    RuleSpec {
        id: REGEXP_POSIX_SPACE,
        constructs: &["fn:REGEXP_LIKE", "fn:REGEXP_REPLACE", "fn:REGEXP_SUBSTR"],
        conditions: "A REGEXP pattern as for rule.regexp that also uses [:space:] inside a bracket expression: ASCII space, tab, line feed, vertical tab, form feed and carriage return, as RE2 reads it.",
        failures: &[],
    },
    RuleSpec {
        id: LEAST_GREATEST,
        constructs: &["fn:LEAST", "fn:GREATEST"],
        conditions: "Arguments that compare directly with each other; the result has their unified type and is NULL when any argument is NULL (DuckDB's own functions skip NULLs).",
        failures: &[],
    },
    RuleSpec {
        id: TEXT_CONTAINS,
        constructs: &["fn:CONTAINS"],
        conditions: "VARCHAR operands, compared bytewise; empty text is contained in any text; NULL when either operand is NULL.",
        failures: &[],
    },
    RuleSpec {
        id: EDIT_DISTANCE,
        constructs: &["fn:EDITDISTANCE"],
        conditions: "Two VARCHAR operands and no maximum distance: the Levenshtein distance counted in characters (code points), case-sensitive, as NUMBER(9,0); NULL when either operand is NULL.",
        failures: &[],
    },
    RuleSpec {
        id: TEXT_AFFIX,
        constructs: &["fn:STARTSWITH", "fn:ENDSWITH"],
        conditions: "VARCHAR operands, compared bytewise; an empty affix matches; NULL when either operand is NULL.",
        failures: &[],
    },
    RuleSpec {
        id: TEMPORAL_TEXT_FORMAT,
        constructs: &["fn:TO_VARCHAR", "fn:TO_CHAR"],
        conditions: "A DATE or TIMESTAMP_NTZ and a literal format of YYYY, MM and DD and, for timestamps, HH24, MI and SS fields, in any case, between the separators - _ / : . and space: each field zero-padded, as strftime writes it.",
        failures: &[],
    },
    RuleSpec {
        id: CAST_TEXT_TIMESTAMP,
        constructs: &["node:cast"],
        conditions: "CAST of non-literal VARCHAR to TIMESTAMP_NTZ(p): the text read as rule.to_timestamp reads it without a format, then cast from TIMESTAMP_NTZ(9) to the precision; relies on timestamp_range. TRY_CAST stays refused.",
        failures: &[FailureMode {
            kind: ErrorKind::TimestampText,
            snowflake: &[TIMESTAMP_NOT_RECOGNIZED],
        }],
    },
    RuleSpec {
        id: CAST_UTC_ZONED,
        constructs: &["node:cast"],
        conditions: "A TIMESTAMP_TZ from CONVERT_TIMEZONE('UTC', x) of two arguments, whose offset is +00:00, cast to TIMESTAMP_NTZ or DATE: its UTC wall time, then as a TIMESTAMP_NTZ cast. Zoned values of unknown offset are refused, since DuckDB keeps instants without their offsets.",
        failures: &[],
    },
    RuleSpec {
        id: CAST_TEMPORAL,
        constructs: &["node:cast"],
        conditions: "CAST between DATE, TIMESTAMP_NTZ, TIMESTAMP_TZ and TIMESTAMP_LTZ, not TRY_CAST: NTZ to fewer digits and NTZ to DATE truncate toward the past; NTZ or DATE to NTZ(7-9) from a microsecond source relies on timestamp_range; NTZ or DATE to TIMESTAMP_TZ or TIMESTAMP_LTZ reads the wall time in the session time zone, truncating to the target digits, and keeping 7-9 digits relies on timestamp_tz_submicro, raising on a non-zero sub-microsecond part. Zoned sources are not covered.",
        failures: &[],
    },
    RuleSpec {
        id: CAST_SAME_TYPE,
        constructs: &["node:cast", "node:try_cast"],
        conditions: "The operand already has the target type, or is NULL.",
        failures: &[FailureMode {
            kind: ErrorKind::StringTooLong,
            snowflake: &[STRING_TOO_LONG],
        }],
    },
    RuleSpec {
        id: TRIM,
        constructs: &["fn:TRIM", "fn:LTRIM", "fn:RTRIM"],
        conditions: "VARCHAR operands; without characters only spaces are removed.",
        failures: &[],
    },
    RuleSpec {
        id: CASE_MAPPING,
        constructs: &["fn:UPPER", "fn:LOWER"],
        conditions: "A VARCHAR operand; characters with multi-character mappings are expanded first.",
        failures: &[],
    },
    RuleSpec {
        id: VARIANT_COMMON_TYPE,
        constructs: &["fn:COALESCE", "fn:IFF", "fn:NVL", "node:case", "node:union"],
        conditions: "Branches that are all VARIANT, OBJECT or ARRAY unify as VARIANT, held as DuckDB JSON; a JSON null is a value, not SQL NULL, so COALESCE keeps it.",
        failures: &[],
    },
    RuleSpec {
        id: CONCAT_WS,
        constructs: &["fn:CONCAT_WS"],
        conditions: "A separator and at least one part, each VARCHAR, NUMBER, BOOLEAN or DATE as for CONCAT; NULL when any part is NULL, or the separator is NULL and there are two parts or more; a single part is returned whatever the separator, as recorded.",
        failures: &[],
    },
    RuleSpec {
        id: CONCAT,
        constructs: &["node:concat", "fn:CONCAT"],
        conditions: "VARCHAR, NUMBER, BOOLEAN or DATE operands; NUMBER, BOOLEAN and DATE are formatted as Snowflake does.",
        failures: &[],
    },
    RuleSpec {
        id: COUNT,
        constructs: &["fn:COUNT"],
        conditions: "COUNT(*), COUNT(x) or COUNT(DISTINCT x); typed NUMBER(18,0). COUNT of a VARIANT skips JSON null as well as NULL; COUNT(DISTINCT) of a VARIANT is refused.",
        failures: &[],
    },
    RuleSpec {
        id: SUM,
        constructs: &["fn:SUM"],
        conditions: "A NUMBER of precision at most 26.",
        failures: &[FailureMode {
            kind: ErrorKind::NumericOverflow,
            snowflake: &[NUMBER_OUT_OF_RANGE],
        }],
    },
    RuleSpec {
        id: AVERAGE,
        constructs: &["fn:AVG"],
        conditions: "A NUMBER of scale at most 6 and precision at most 26.",
        failures: &[],
    },
    RuleSpec {
        id: MIN_MAX_FLOAT,
        constructs: &["fn:MIN", "fn:MAX"],
        conditions: "MIN and MAX of FLOAT, NULLs skipped: NaN orders above every number and infinity, as DuckDB orders it.",
        failures: &[],
    },
    RuleSpec {
        id: MIN_MAX,
        constructs: &["fn:MIN", "fn:MAX"],
        conditions: "NUMBER, VARCHAR, BOOLEAN, DATE or a timestamp type; the result keeps the argument type.",
        failures: &[],
    },
    RuleSpec {
        id: WINDOW_RANKING,
        constructs: &[
            "fn:ROW_NUMBER",
            "fn:RANK",
            "fn:DENSE_RANK",
            "node:window_function",
        ],
        conditions: "An ORDER BY and no frame; typed NUMBER(18,0).",
        failures: &[],
    },
    RuleSpec {
        id: WINDOW_OFFSET,
        constructs: &["fn:LAG", "fn:LEAD", "node:window_function"],
        conditions: "An ORDER BY, no frame, an integer literal offset and a default of the value's type.",
        failures: &[],
    },
    RuleSpec {
        id: WINDOW_FIRST_LAST,
        constructs: &["fn:FIRST_VALUE", "fn:LAST_VALUE", "node:window_function"],
        conditions: "An ORDER BY; no frame or the whole partition, which is Snowflake's default for these functions.",
        failures: &[],
    },
    RuleSpec {
        id: WINDOW_AGGREGATE,
        constructs: &[
            "fn:COUNT",
            "fn:SUM",
            "fn:MIN",
            "fn:MAX",
            "node:window_function",
        ],
        conditions: "The aggregate's own conditions; without a frame, a running RANGE frame when ordered.",
        failures: &[FailureMode {
            kind: ErrorKind::NumericOverflow,
            snowflake: &[NUMBER_OUT_OF_RANGE],
        }],
    },
];

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
];
/// Characters a format may hold between fields, written as they are.
pub const TEMPORAL_TEXT_SEPARATORS: &str = "-_/:. ";
/// The pair behind converting `TO_DOUBLE` and `TRY_TO_DOUBLE` as a cast to FLOAT.
pub const TO_DOUBLE_PAIR: &str = "to_double_to_cast";
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
/// The pair behind converting `t RLIKE p` through the `REGEXP_LIKE` rule.
pub const RLIKE_PAIR: &str = "rlike_to_regexp_like";

/// Every recorded spelling pair; `sqlquack verify` requires both spellings' results to match.
pub const SPELLING_PAIRS: &[SpellingPair] = &[
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
        id: "position_to_charindex",
        source: "POSITION(s, t [, n]) and POSITION(s IN t)",
        canonical: "CHARINDEX(s, t [, n])",
        cases: "fidelity/position.toml",
    },
    SpellingPair {
        id: RLIKE_PAIR,
        source: "t RLIKE p",
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
        id: "mod_function_to_operator",
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
        source: "YEAR(x), QUARTER(x), MONTH(x), DAY(x), HOUR(x), MINUTE(x), SECOND(x)",
        canonical: "DATE_PART(unit, x)",
        cases: "fidelity/part_functions.toml",
    },
];
