//! The schema helper macros live in and the statement creating it.

/// The schema of every helper macro; converted SQL names it unquoted, as nothing else is printed.
pub const HELPER_SCHEMA: &str = "sqlquack";
/// What opens a call of a helper macro.
pub const HELPER_CALL_PREFIX: &str = "sqlquack.";
/// The parameter holding a raising helper's raise site, its first.
pub const SITE_PARAMETER: &str = "quack_site";
/// JSON text with Snowflake's escapes.
pub const JSON_ESCAPED_V1: &str = "json_escaped_v1";
/// A VARIANT cast to VARCHAR, as Snowflake writes it.
pub const VARIANT_TEXT_V1: &str = "variant_text_v1";
/// A VARIANT as JSON text, as Snowflake writes it.
pub const VARIANT_JSON_TEXT_V1: &str = "variant_json_text_v1";
/// A VARIANT cast to a NUMBER.
pub const VARIANT_NUMBER_V1: &str = "variant_number_v1";
/// A VARIANT cast to FLOAT.
pub const VARIANT_FLOAT_V1: &str = "variant_float_v1";
/// A VARIANT cast to BOOLEAN.
pub const VARIANT_BOOLEAN_V1: &str = "variant_boolean_v1";
/// A VARIANT cast to DATE.
pub const VARIANT_DATE_V1: &str = "variant_date_v1";
/// A VARIANT cast to TIMESTAMP_NTZ.
pub const VARIANT_TIMESTAMP_NTZ_V1: &str = "variant_timestamp_ntz_v1";
/// Text read as a timestamp, raising Snowflake's error for text it does not read.
pub const TIMESTAMP_TEXT_V1: &str = "timestamp_text_v1";
/// Text read as a timestamp, NULL for text Snowflake does not read, as TRY_ forms.
pub const TRY_TIMESTAMP_TEXT_V1: &str = "try_timestamp_text_v1";
/// Text read as a date, raising Snowflake's error for text it does not read.
pub const DATE_TEXT_V1: &str = "date_text_v1";
/// Text read as a date, NULL for text Snowflake does not read, as TRY_ forms.
pub const TRY_DATE_TEXT_V1: &str = "try_date_text_v1";
/// Text read as the UTC wall time of the instant it names.
pub const ZONED_TEXT_V1: &str = "zoned_text_v1";
/// Text read as the UTC wall time of the instant it names, NULL where Snowflake reads none.
pub const TRY_ZONED_TEXT_V1: &str = "try_zoned_text_v1";
/// Text read as an instant whose offset must be +00:00.
pub const UTC_ZONED_TEXT_V1: &str = "utc_zoned_text_v1";
/// Text read as an instant whose offset must be +00:00, NULL where Snowflake reads none.
pub const TRY_UTC_ZONED_TEXT_V1: &str = "try_utc_zoned_text_v1";
/// A time zone name Snowflake was recorded accepting, raising time_zone_spelling for others.
pub const TIME_ZONE_V1: &str = "time_zone_v1";
/// The full read check of JSON text.
pub const JSON_READ_V1: &str = "json_read_v1";
/// The fast read check of JSON text, falling back to the full one.
pub const JSON_CHECKED_V1: &str = "json_checked_v1";
/// The statement creating the schema, before any helper macro.
pub const CREATE_HELPER_SCHEMA: &str = "CREATE SCHEMA IF NOT EXISTS sqlquack";
/// The parameter holding the VARIANT a writing or casting macro reads.
pub const VALUE_PARAMETER: &str = "quack_value";
/// The parameter holding the JSON text a reading or escaping macro reads.
pub const TEXT_PARAMETER: &str = "quack_text";
/// The parameter holding a NULL of the NUMBER type the number macro casts to.
pub const NUMBER_TYPE_PARAMETER: &str = "quack_type";
/// The parameter holding the largest value of that type.
pub const NUMBER_LARGEST_PARAMETER: &str = "quack_largest";
/// The parameter holding whether the read check makes unfinished text NULL, as TRY_PARSE_JSON.
pub const TRYING_PARAMETER: &str = "quack_trying";
/// The parameter listing the paths of objects whose keys the fast read check checks.
pub const OBJECTS_PARAMETER: &str = "quack_objects";
/// The parameter listing the paths of values the fast read check checks at every depth.
pub const VALUES_PARAMETER: &str = "quack_values";
