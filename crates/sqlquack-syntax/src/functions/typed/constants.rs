//! Constants of functions typed.

use crate::functions::typed::models::TypedDispatchGroup;
use crate::functions::typed::models::TypedDispatchGroupSpec;
use crate::functions::typed::models::TypedFunctionSpec;
use crate::functions::typed::models::TypedParseKind;

/// Canonical typed function specs with alias spellings.
pub(crate) const TYPED_FUNCTION_SPECS: &[TypedFunctionSpec] = &[
    TypedFunctionSpec {
        canonical_name: "COUNT_IF",
        aliases: &["COUNTIF"],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "STARTS_WITH",
        aliases: &["STARTSWITH"],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ENDS_WITH",
        aliases: &["ENDSWITH"],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "DAYOFWEEK",
        aliases: &["DAY_OF_WEEK"],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "DAYOFYEAR",
        aliases: &["DAY_OF_YEAR"],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "DAYOFMONTH",
        aliases: &["DAY_OF_MONTH"],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "WEEKOFYEAR",
        aliases: &["WEEK_OF_YEAR"],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "SUBSTRING",
        aliases: &["SUBSTR"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "DATE_PART",
        aliases: &["DATEPART"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "DATEADD",
        aliases: &["DATE_ADD", "TIMEADD", "TIMESTAMPADD"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "DATEDIFF",
        aliases: &["DATE_DIFF", "TIMEDIFF", "TIMESTAMPDIFF"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TRY_CAST",
        aliases: &["TRYCAST"],
        parse_kind: TypedParseKind::CastLike,
    },
    TypedFunctionSpec {
        canonical_name: "LENGTH",
        aliases: &["LEN", "CHAR_LENGTH", "CHARACTER_LENGTH", "TEXTLEN"],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "LOWER",
        aliases: &["LCASE"],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "UPPER",
        aliases: &["UCASE"],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "CEIL",
        aliases: &["CEILING"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TO_NUMBER",
        aliases: &["TO_DECIMAL", "TO_NUMERIC"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TRY_TO_NUMBER",
        aliases: &["TRY_TO_DECIMAL", "TRY_TO_NUMERIC"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TIMESTAMP_FROM_PARTS",
        aliases: &["TIMESTAMPFROMPARTS"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TIMESTAMP_NTZ_FROM_PARTS",
        aliases: &["TIMESTAMPNTZFROMPARTS"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TIMESTAMP_LTZ_FROM_PARTS",
        aliases: &["TIMESTAMPLTZFROMPARTS"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TIMESTAMP_TZ_FROM_PARTS",
        aliases: &["TIMESTAMPTZFROMPARTS"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "DATE_FROM_PARTS",
        aliases: &["DATEFROMPARTS"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TIME_FROM_PARTS",
        aliases: &["TIMEFROMPARTS"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TYPEOF",
        aliases: &["TOTYPENAME"],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "SIN",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "COS",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "TAN",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ASIN",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ACOS",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ATAN",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "RADIANS",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "DEGREES",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ATAN2",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "YEAR",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "MONTH",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "DAY",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "HOUR",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "MINUTE",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "SECOND",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "DAYOFWEEK_ISO",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "QUARTER",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "EPOCH",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "EPOCH_MS",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_LENGTH",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_SIZE",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "CARDINALITY",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_REVERSE",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_DISTINCT",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_COMPACT",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "EXPLODE",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "EXPLODE_OUTER",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "MAP_FROM_ENTRIES",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "MAP_KEYS",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "MAP_VALUES",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "MAP_FROM_ARRAYS",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "MAP_CONTAINS_KEY",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ELEMENT_AT",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ABS",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "SQRT",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "EXP",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "LN",
        aliases: &[],
        parse_kind: TypedParseKind::Unary,
    },
    TypedFunctionSpec {
        canonical_name: "CONTAINS",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "MOD",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "POW",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ADD_MONTHS",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "MONTHS_BETWEEN",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "NEXT_DAY",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_CONTAINS",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_POSITION",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_APPEND",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_PREPEND",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_UNION",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_EXCEPT",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_REMOVE",
        aliases: &[],
        parse_kind: TypedParseKind::Binary,
    },
    TypedFunctionSpec {
        canonical_name: "RANDOM",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "RAND",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "PI",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "LAST_DAY",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "POSITION",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "STRPOS",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "LOCATE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "INSTR",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "NORMALIZE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "INITCAP",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "FLOOR",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "LOG",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "FLATTEN",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_INTERSECT",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "CURRENT_SCHEMAS",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "COALESCE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "IFNULL",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "NVL",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "NVL2",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "EXTRACT",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "STRUCT",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "CHAR",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "CHR",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "RANGE_N",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "XMLTABLE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "XMLELEMENT",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "XMLATTRIBUTES",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "XMLCOMMENT",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "MATCH",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TRANSFORM",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "CONVERT",
        aliases: &["TRY_CONVERT"],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TRIM",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "OVERLAY",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "COUNT",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "LIST",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "MAP",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "SUM",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "AVG",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "MIN",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "MAX",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_AGG",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "ARRAY_CONCAT_AGG",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "STDDEV",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "STDDEV_POP",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "STDDEV_SAMP",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "VARIANCE",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "VAR_POP",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "VAR_SAMP",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "BOOL_AND",
        aliases: &["LOGICAL_AND", "EVERY"],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "BOOL_OR",
        aliases: &["LOGICAL_OR"],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "MEDIAN",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "MODE",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "FIRST",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "LAST",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "ANY_VALUE",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "APPROX_DISTINCT",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "APPROX_COUNT_DISTINCT",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "BIT_AND",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "BIT_OR",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "BIT_XOR",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "STRING_AGG",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "GROUP_CONCAT",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "LISTAGG",
        aliases: &[],
        parse_kind: TypedParseKind::AggregateLike,
    },
    TypedFunctionSpec {
        canonical_name: "ROW_NUMBER",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "RANK",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "DENSE_RANK",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "PERCENT_RANK",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "CUME_DIST",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "NTILE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "LEAD",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "LAG",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "FIRST_VALUE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "LAST_VALUE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "NTH_VALUE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_EXTRACT",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_EXTRACT_SCALAR",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_QUERY",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_VALUE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_ARRAY_LENGTH",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_KEYS",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_TYPE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TO_JSON",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "PARSE_JSON",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_OBJECT",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_ARRAY",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_ARRAYAGG",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_OBJECTAGG",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "JSON_TABLE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "TRANSLATE",
        aliases: &[],
        parse_kind: TypedParseKind::Variadic,
    },
    TypedFunctionSpec {
        canonical_name: "IF",
        aliases: &["IIF", "IFF"],
        parse_kind: TypedParseKind::Conditional,
    },
];

pub(crate) const TYPED_DISPATCH_GROUP_SPECS: &[TypedDispatchGroupSpec] = &[
    TypedDispatchGroupSpec {
        name: "COUNT",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "LIST",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "MAP",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "ARRAY",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "SUM",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "AVG",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "MIN",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "MAX",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "ARRAY_AGG",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "ARRAY_CONCAT_AGG",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "STDDEV",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "STDDEV_POP",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "STDDEV_SAMP",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "VARIANCE",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "VAR_POP",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "VAR_SAMP",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "BOOL_AND",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "BOOL_OR",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "MEDIAN",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "MODE",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "FIRST",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "LAST",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "ANY_VALUE",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "APPROX_DISTINCT",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "APPROX_COUNT_DISTINCT",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "BIT_AND",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "BIT_OR",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "BIT_XOR",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "STRING_AGG",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "GROUP_CONCAT",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "LISTAGG",
        group: TypedDispatchGroup::Aggregate,
    },
    TypedDispatchGroupSpec {
        name: "ROW_NUMBER",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "RANK",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "DENSE_RANK",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "PERCENT_RANK",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "CUME_DIST",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "NTILE",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "LEAD",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "LAG",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "FIRST_VALUE",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "LAST_VALUE",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "NTH_VALUE",
        group: TypedDispatchGroup::Window,
    },
    TypedDispatchGroupSpec {
        name: "JSON_EXTRACT",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_EXTRACT_SCALAR",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_QUERY",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_VALUE",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_ARRAY_LENGTH",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_KEYS",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_TYPE",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "TO_JSON",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "PARSE_JSON",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_OBJECT",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_ARRAY",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_ARRAYAGG",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_OBJECTAGG",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "JSON_TABLE",
        group: TypedDispatchGroup::Json,
    },
    TypedDispatchGroupSpec {
        name: "TRANSLATE",
        group: TypedDispatchGroup::TranslateTeradata,
    },
];
