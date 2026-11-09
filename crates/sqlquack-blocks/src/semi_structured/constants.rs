//! Building block identifiers and the patterns the JSON guard uses.

/// JSON text read as a VARIANT, raising where DuckDB might read it differently: `duckdb.json_guarded`.
pub const JSON_GUARDED: &str = "duckdb.json_guarded";
/// A path into a VARIANT: `duckdb.json_path`.
pub const JSON_PATH: &str = "duckdb.json_path";
/// The text a VARIANT is hashed by: `duckdb.json_hash_key`.
pub const JSON_HASH_KEY: &str = "duckdb.json_hash_key";
/// A VARIANT as Snowflake writes it as text: `duckdb.json_text`.
pub const JSON_TEXT: &str = "duckdb.json_text";
/// Snowflake's TYPEOF of a VARIANT: `duckdb.json_type_name`.
pub const JSON_TYPE_NAME: &str = "duckdb.json_type_name";
/// A VARIANT cast to a scalar Snowflake type: `duckdb.json_scalar`.
pub const JSON_SCALAR: &str = "duckdb.json_scalar";
/// A VARIANT cast to TIMESTAMP_TZ as a UTC wall time: `duckdb.json_zoned_wall`.
pub const JSON_ZONED_WALL: &str = "duckdb.json_zoned_wall";
/// A key matched without regard to ASCII case: `duckdb.json_get_ignore_case`.
pub const JSON_GET_IGNORE_CASE: &str = "duckdb.json_get_ignore_case";
/// SQL NULL as a VARIANT: `duckdb.json_null`.
pub const JSON_NULL: &str = "duckdb.json_null";
/// FLATTEN's rows for a VARIANT: `duckdb.json_flatten`.
pub const JSON_FLATTEN: &str = "duckdb.json_flatten";
/// FLATTEN with RECURSIVE => TRUE: `duckdb.json_flatten_recursive`.
pub const JSON_FLATTEN_RECURSIVE: &str = "duckdb.json_flatten_recursive";
/// An object key FLATTEN writes in a path as it is: a name of letters, Latin-1 letters included.
pub const PLAIN_KEY_PATTERN: &str = "[A-Za-z\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{FF}_][A-Za-z\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{FF}0-9_]*";
/// OBJECT_DELETE of literal keys: `duckdb.json_object_delete`.
pub const JSON_OBJECT_DELETE: &str = "duckdb.json_object_delete";
/// OBJECT_INSERT of one key: `duckdb.json_object_insert`.
pub const JSON_OBJECT_INSERT: &str = "duckdb.json_object_insert";
/// OBJECT_CONSTRUCT and OBJECT_CONSTRUCT_KEEP_NULL: `duckdb.json_object_construct`.
pub const JSON_OBJECT_CONSTRUCT: &str = "duckdb.json_object_construct";
/// A VARIANT result column, written as Snowflake returns it: `duckdb.json_output`.
pub const JSON_OUTPUT: &str = "duckdb.json_output";
/// The message of the error a VARIANT that is not an object raises where one is needed.
pub const NOT_AN_OBJECT_MESSAGE: &str = "Failed to cast variant value to OBJECT";
/// The message of the error inserting a key an object already holds raises.
pub const KEY_EXISTS_MESSAGE: &str = "Duplicate field key";
/// IS_OBJECT, IS_ARRAY and IS_BOOLEAN: `duckdb.json_is_kind`.
pub const JSON_IS_KIND: &str = "duckdb.json_is_kind";
/// ARRAY_CONSTRUCT: `duckdb.json_array_construct`.
pub const JSON_ARRAY_CONSTRUCT: &str = "duckdb.json_array_construct";
/// ARRAY_EXCEPT of arrays of strings: `duckdb.json_array_except`.
pub const JSON_ARRAY_EXCEPT: &str = "duckdb.json_array_except";
/// ARRAY_CONSTRUCT_COMPACT: `duckdb.json_array_construct_compact`.
pub const JSON_ARRAY_CONSTRUCT_COMPACT: &str = "duckdb.json_array_construct_compact";
/// ARRAY_CONTAINS: `duckdb.json_array_contains`.
pub const JSON_ARRAY_CONTAINS: &str = "duckdb.json_array_contains";
/// A JSON number written as a plain decimal that is not whole, which equals no integer.
pub const JSON_FRACTION_PATTERN: &str = r"-?[0-9]+\.[0-9]*[1-9]";
/// ARRAY_TO_STRING: `duckdb.json_array_to_string`.
pub const JSON_ARRAY_TO_STRING: &str = "duckdb.json_array_to_string";
/// The message of the error ARRAY_TO_STRING raises for a value other than an array.
pub const NOT_AN_ARRAY_ARGUMENT_MESSAGE: &str = "Left argument of string is not an array";
/// The message of the error ARRAY_TO_STRING raises for a JSON null element.
pub const ARRAY_ELEMENT_NOT_TEXT_MESSAGE: &str =
    "Failed to cast variant value from array to string";
/// The JSON kind ARRAY, which a cast to ARRAY wraps other values in.
pub const ARRAY_KIND: &str = "ARRAY";
/// ARRAY_SIZE: `duckdb.json_array_size`.
pub const JSON_ARRAY_SIZE: &str = "duckdb.json_array_size";
/// SPLIT: `duckdb.json_split`.
pub const JSON_SPLIT: &str = "duckdb.json_split";
/// OBJECT_KEYS: `duckdb.json_object_keys`.
pub const JSON_OBJECT_KEYS: &str = "duckdb.json_object_keys";
/// OBJECT_PICK: `duckdb.json_object_pick`.
pub const JSON_OBJECT_PICK: &str = "duckdb.json_object_pick";
/// A VARIANT cast to OBJECT or ARRAY: `duckdb.json_kind_cast`.
pub const JSON_KIND_CAST: &str = "duckdb.json_kind_cast";
/// A typed value put in a VARIANT: `duckdb.json_typed_text`.
pub const JSON_TYPED_TEXT: &str = "duckdb.json_typed_text";
/// A NUMBER with a scale put in a VARIANT: `duckdb.json_decimal`.
pub const JSON_DECIMAL: &str = "duckdb.json_decimal";
/// IS_DATE-style checks of a VARIANT: `duckdb.json_is_typed`.
pub const JSON_IS_TYPED: &str = "duckdb.json_is_typed";
/// The divergence typed values in a VARIANT rely on.
pub const VARIANT_TYPED_VALUES: &str = "variant_typed_values";
/// The VARIANT texts of the typed values SQLQuack stores: a date, and timestamps to milliseconds.
pub const TYPED_TEXT_PATTERN: &str =
    "[0-9]{4}-[0-9]{2}-[0-9]{2}( [0-9]{2}:[0-9]{2}:[0-9]{2}\\.[0-9]{3}( Z)?)?";
/// The divergence the guards rely on.
pub const JSON_FIDELITY: &str = "json_fidelity";
/// The divergence a read of a column another model wrote from a VARIANT-held value relies on.
pub const VARIANT_CHAIN_TYPE: &str = "variant_chain_type";
/// The assumption that a source table's VARIANT holds JSON values, no typed leaf.
pub const SOURCE_VARIANT_JSON: &str = "source_variant_json";
/// A JSON string, as RE2 matches it.
pub const JSON_STRING_PATTERN: &str = r#""(?:[^"\\]|\\.)*""#;
/// The JSON literals spelled with letters.
pub const JSON_WORD_PATTERN: &str = r"\b(?:true|false|null)\b";
/// Outside strings: a letter, a trailing comma, a zero fraction, six leading zeros or 39 digits.
pub const JSON_UNSAFE_TOKEN_PATTERN: &str =
    r"[A-Za-z]|,\s*[\]}]|[0-9]\.0*(?:[^0-9]|$)|\.0{6}|[0-9]{39}";
/// A JSON number without an exponent, outside strings.
pub const JSON_NUMBER_PATTERN: &str = r"-?[0-9]+(?:\.[0-9]+)?";
/// A UTF-16 surrogate escape, which Snowflake may write as invalid UTF-8.
pub const JSON_SURROGATE_PATTERN: &str = r"\\u[dD][89abAB]";
/// The hex digits DuckDB writes in upper case and Snowflake in lower case.
pub const JSON_ESCAPE_HEX_LETTERS: &str = "ABCDEF";
/// Any JSON number, exponent included.
pub const JSON_ANY_NUMBER_PATTERN: &str = r"-?[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?";
/// Digits beyond which a JSON decimal is kept only when DuckDB's double writes it back as written.
pub const JSON_DECIMAL_DIGITS: usize = 15;
/// Nanoseconds in a second, for integers read as epoch seconds.
pub const NANOS_PER_SECOND: i64 = 1_000_000_000;

/// OBJECT_AGG: `duckdb.json_object_agg`.
pub const JSON_OBJECT_AGG: &str = "duckdb.json_object_agg";
/// ARRAY_POSITION: `duckdb.json_array_position`.
pub const JSON_ARRAY_POSITION: &str = "duckdb.json_array_position";
/// ARRAY_CAT: `duckdb.json_array_cat`.
pub const JSON_ARRAY_CAT: &str = "duckdb.json_array_cat";
/// ARRAY_COMPACT: `duckdb.json_array_compact`.
pub const JSON_ARRAY_COMPACT: &str = "duckdb.json_array_compact";
/// The message of the error ARRAY_CAT raises for a first argument other than an array.
pub const ARRAY_CAT_LEFT_MESSAGE: &str = "Left argument of ARRAY_CAT is not an array";
/// The message of the error ARRAY_CAT raises for a second argument other than an array.
pub const ARRAY_CAT_RIGHT_MESSAGE: &str = "Right argument of ARRAY_CAT is not an array";
/// The message of the error ARRAY_COMPACT raises for an argument other than an array.
pub const ARRAY_COMPACT_MESSAGE: &str = "First argument of ARRAY_COMPACT is not an array";
/// GET with a computed index or key: `duckdb.json_get_computed`.
pub const JSON_GET_COMPUTED: &str = "duckdb.json_get_computed";
/// FILTER: `duckdb.json_filter`.
pub const JSON_FILTER: &str = "duckdb.json_filter";
/// TRANSFORM: `duckdb.json_transform`.
pub const JSON_TRANSFORM: &str = "duckdb.json_transform";
/// REDUCE, a left fold of an array's elements: `duckdb.json_reduce`.
pub const JSON_REDUCE: &str = "duckdb.json_reduce";
/// JSON text with Snowflake's escapes: `duckdb.json_escaped`.
pub const JSON_ESCAPED: &str = "duckdb.json_escaped";
/// A VARIANT equal to an integer NUMBER: `duckdb.json_number_equal`.
pub const JSON_NUMBER_EQUAL: &str = "duckdb.json_number_equal";
/// Object keys a quoted JSON path cannot name exactly: the empty key and the wildcard.
pub const PATH_READ_KEYS: &[&str] = &["", "*"];

/// A key step of a DuckDB `json_tree` full key: a bare name, or a quoted one.
pub const KEY_STEP: &str = r#"\.([A-Za-z_][A-Za-z0-9_]*|"([^"\\]|\\.)*")"#;
/// An element step of a DuckDB `json_tree` full key.
pub const ELEMENT_STEP: &str = r"\[[0-9]+\]";
/// A key Snowflake was recorded writing in a path as `['key']`: printable ASCII but a backslash.
pub const BRACKETED_KEY: &str = r"[ -\[\]-~]*";
/// Negative zero with an all-zero fraction, and runs of 20 or more digits, kept as written.
pub const KEPT_NUMBER_PATTERN: &str = r"-0\.0+|-?[0-9]{20,}(?:\.[0-9]+)?";
/// A number of at most 38 digits with an all-zero fraction, and the JSON delimiter after it.
pub const ZERO_FRACTION_PATTERN: &str = r"|(-?[1-9][0-9]{0,18}|0)\.0{1,19}([,\]} \t\n\r]|$)";
