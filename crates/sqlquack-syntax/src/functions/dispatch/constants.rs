//! Constants of functions dispatch.

use crate::functions::dispatch::models::ParserDispatchBehavior;
use crate::functions::dispatch::models::ParserDispatchSpec;

/// Parser dispatch metadata currently migrated from parser-local phase-3 lists.
pub(crate) const PARSER_DISPATCH_SPECS: &[ParserDispatchSpec] = &[
    ParserDispatchSpec {
        name: "TRY_TO_DATE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_TO_TIMESTAMP",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_TO_TIME",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_TO_DOUBLE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_TO_BOOLEAN",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_TO_BINARY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_TO_GEOGRAPHY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_TO_GEOMETRY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_PARSE_JSON",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_TO_DECFLOAT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_BASE64_DECODE_BINARY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_BASE64_DECODE_STRING",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_HEX_DECODE_BINARY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TRY_HEX_DECODE_STRING",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_DATE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_TIMESTAMP",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_TIMESTAMP_NTZ",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_TIMESTAMP_LTZ",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_TIMESTAMP_TZ",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_TIME",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_CHAR",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_VARCHAR",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_DOUBLE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_BOOLEAN",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_BINARY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_VARIANT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_OBJECT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_ARRAY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_GEOGRAPHY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TO_GEOMETRY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TIME_SLICE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "MAP_CAT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "MAP_DELETE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "MAP_INSERT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "MAP_PICK",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "MAP_SIZE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "HEX_ENCODE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "HEX_DECODE_STRING",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "HEX_DECODE_BINARY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BASE64_ENCODE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BASE64_DECODE_STRING",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BASE64_DECODE_BINARY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "COMPRESS",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "DECOMPRESS_BINARY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "DECOMPRESS_STRING",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "STRTOK",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "SPLIT_PART",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "SOUNDEX",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "SOUNDEX_P123",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "RTRIMMED_LENGTH",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BIT_LENGTH",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "UNICODE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "JAROWINKLER_SIMILARITY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "EDITDISTANCE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "FACTORIAL",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "SQUARE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "CBRT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "SINH",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "COSH",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "TANH",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "ASINH",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "ACOSH",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "ATANH",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "WIDTH_BUCKET",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "NORMAL",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "UNIFORM",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "ZIPF",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BITAND",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BITOR",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BITXOR",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BITNOT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BITSHIFTLEFT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "BITSHIFTRIGHT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "GETBIT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "SETBIT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "REGEXP_LIKE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "RLIKE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "REGEXP_REPLACE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "REGEXP_SUBSTR",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "REGEXP_SUBSTR_ALL",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "REGEXP_INSTR",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "REGEXP_COUNT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "GET",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "LIKE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "ILIKE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "XMLGET",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "CHECK_XML",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "VECTOR_COSINE_SIMILARITY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "VECTOR_INNER_PRODUCT",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "VECTOR_L1_DISTANCE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "VECTOR_L2_DISTANCE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "WEEKISO",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "YEAROFWEEK",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "YEAROFWEEKISO",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "MONTHNAME",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "DAYNAME",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "PREVIOUS_DAY",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "CONVERT_TIMEZONE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "RANDSTR",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "PARSE_URL",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "PARSE_IP",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "IDENTIFIER",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "EQUAL_NULL",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "IS_NULL_VALUE",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "NULLIFZERO",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "ZEROIFNULL",
        behavior: ParserDispatchBehavior::ExprListFunction,
    },
    ParserDispatchSpec {
        name: "GREATEST",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "LEAST",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "GREATEST_IGNORE_NULLS",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "LEAST_IGNORE_NULLS",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "GROUPING_ID",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "GROUPING",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "UUID_STRING",
        behavior: ParserDispatchBehavior::OptionalExprListFunction,
    },
    ParserDispatchSpec {
        name: "SEARCH",
        behavior: ParserDispatchBehavior::FunctionArgumentsFunction,
    },
    ParserDispatchSpec {
        name: "SEARCH_IP",
        behavior: ParserDispatchBehavior::FunctionArgumentsFunction,
    },
    ParserDispatchSpec {
        name: "STAR",
        behavior: ParserDispatchBehavior::FunctionArgumentsFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_ACCOUNT",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_ACCOUNT_NAME",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_AVAILABLE_ROLES",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_CLIENT",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_IP_ADDRESS",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_DATABASE",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_SECONDARY_ROLES",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_SESSION",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_STATEMENT",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_VERSION",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_TRANSACTION",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_WAREHOUSE",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_ORGANIZATION_USER",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_REGION",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_ROLE",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_ROLE_TYPE",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CURRENT_ORGANIZATION_NAME",
        behavior: ParserDispatchBehavior::ZeroArgFunction,
    },
    ParserDispatchSpec {
        name: "CHECK_JSON",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "JSON_EXTRACT_PATH_TEXT",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "GET_PATH",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "OBJECT_CONSTRUCT",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "OBJECT_CONSTRUCT_KEEP_NULL",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "OBJECT_INSERT",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "OBJECT_DELETE",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "OBJECT_PICK",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "ARRAY_CONSTRUCT",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "ARRAY_CONSTRUCT_COMPACT",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "ARRAY_SLICE",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "ARRAY_FLATTEN",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByFilter,
    },
    ParserDispatchSpec {
        name: "AI_AGG",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "AI_SUMMARIZE_AGG",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "AI_CLASSIFY",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BITMAP_BUCKET_NUMBER",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BITMAP_CONSTRUCT_AGG",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BITMAP_COUNT",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BITMAP_OR_AGG",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BOOLAND",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BOOLOR",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BOOLXOR",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BOOLXOR_AGG",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BOOLAND_AGG",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "BOOLOR_AGG",
        behavior: ParserDispatchBehavior::ExprListMaybeAggregateByAggSuffix,
    },
    ParserDispatchSpec {
        name: "HASH",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "HASH_AGG",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "MD5",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "MD5_HEX",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "MD5_BINARY",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "SHA1",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "SHA1_HEX",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "SHA1_BINARY",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "SHA2",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "SHA2_HEX",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "SHA2_BINARY",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "MINHASH",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "FARM_FINGERPRINT",
        behavior: ParserDispatchBehavior::HashLike,
    },
    ParserDispatchSpec {
        name: "HLL",
        behavior: ParserDispatchBehavior::HllAggregate,
    },
    ParserDispatchSpec {
        name: "PERCENTILE",
        behavior: ParserDispatchBehavior::PercentileAggregate,
    },
    ParserDispatchSpec {
        name: "APPROX_TOP_K",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "APPROX_TOP_K_ACCUMULATE",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "APPROX_TOP_K_COMBINE",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "APPROX_TOP_K_ESTIMATE",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "APPROX_PERCENTILE_ACCUMULATE",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "APPROX_PERCENTILE_COMBINE",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "APPROX_PERCENTILE_ESTIMATE",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "OBJECT_AGG",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_AVGX",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_AVGY",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_COUNT",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_INTERCEPT",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_R2",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_SXX",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_SXY",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_SYY",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_SLOPE",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_VALX",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "REGR_VALY",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "CORR",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "COVAR_POP",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "COVAR_SAMP",
        behavior: ParserDispatchBehavior::ExprListAggregate,
    },
    ParserDispatchSpec {
        name: "KURTOSIS",
        behavior: ParserDispatchBehavior::UnaryAggregate,
    },
    ParserDispatchSpec {
        name: "SKEW",
        behavior: ParserDispatchBehavior::UnaryAggregate,
    },
    ParserDispatchSpec {
        name: "TRANSLATE",
        behavior: ParserDispatchBehavior::TranslateNonTeradata,
    },
];
