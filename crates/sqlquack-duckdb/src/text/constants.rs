//! Text building block identifiers.

/// Levenshtein distance counted in characters: `duckdb.character_edit_distance`.
pub const CHARACTER_EDIT_DISTANCE: &str = "duckdb.character_edit_distance";

/// SPLIT_TO_TABLE's rows for a text: `duckdb.split_to_rows`.
pub const SPLIT_TO_ROWS: &str = "duckdb.split_to_rows";

/// INITCAP's words, capitalised after the given delimiters: `duckdb.title_case`.
pub const TITLE_CASE: &str = "duckdb.title_case";

/// CHR's character for a code point: `duckdb.code_point_text`.
pub const CODE_POINT_TEXT: &str = "duckdb.code_point_text";

/// HEX_ENCODE's hexadecimal digits of the UTF-8 bytes: `duckdb.hex_text`.
pub const HEX_TEXT: &str = "duckdb.hex_text";

/// TRY_HEX_DECODE_STRING's text from hexadecimal digits: `duckdb.hex_decoded_text`.
pub const HEX_DECODED_TEXT: &str = "duckdb.hex_decoded_text";

/// The divergence for REGEXP patterns from a column in a syntax no recording shows read alike.
pub const REGEXP_PATTERN_SYNTAX: &str = "regexp_pattern_syntax";

/// Regular expression syntax RE2 and Snowflake were recorded reading alike, anchors only at the ends.
pub const RECORDED_PATTERN_SYNTAX: &str =
    r"\^?(?:(?:[^\\.\[\](){}*+?|^$]|\.|\)|\[\^?[^\]\[\\:][^\]\[\\:]*\])[*+?]?|\(|\|)*\$?";

/// A REGEXP pattern from a column checked against the recorded syntax: `duckdb.guarded_pattern`.
pub const GUARDED_PATTERN: &str = "duckdb.guarded_pattern";

/// The divergence for text compared with a NUMBER that it spells beyond the NUMBER's scale.
pub const TEXT_NUMBER_SCALE: &str = "text_number_scale";

/// Text compared with a NUMBER, read at its scale or raising: `duckdb.compared_number_text`.
pub const COMPARED_NUMBER_TEXT: &str = "duckdb.compared_number_text";
/// A NUMBER compared with text, raising beyond the recorded widths: `duckdb.compared_number_range`.
pub const COMPARED_NUMBER_RANGE: &str = "duckdb.compared_number_range";
/// The whole digits of a NUMBER recorded comparing with text; 38 overflow as 100046.
pub const TEXT_COMPARED_WHOLE_DIGITS: u8 = 20;
/// The scale from which a NUMBER compared with text was recorded never overflowing.
pub const TEXT_COMPARED_SAFE_SCALE: u8 = 10;

/// Whole pairs of hexadecimal digits of either case, which TRY_HEX_DECODE_STRING decodes.
pub const HEX_PAIRS_PATTERN: &str = "(?:[0-9A-Fa-f]{2})*";

/// The divergence INITCAP raises for text no recording shows it capitalising.
pub const INITCAP_TEXT: &str = "initcap_text";

/// The regular expression class of characters INITCAP is recorded reading.
pub const TITLE_CASE_KNOWN: &str = "\\p{L}\\p{N} _'-";

/// Letters whose title case differs from their upper case, which INITCAP raises for.
pub const TITLE_CASE_DIGRAPHS: &str = "\u{1C4}\u{1C5}\u{1C6}\u{1C7}\u{1C8}\u{1C9}\u{1CA}\u{1CB}\u{1CC}\u{1F1}\u{1F2}\u{1F3}\u{10D0}-\u{10FA}\u{10FD}-\u{10FF}";

/// The largest Unicode code point.
pub const MAX_CODE_POINT: u32 = 0x10_FFFF;

/// The UTF-16 surrogate code points, which no character has.
pub const SURROGATE_CODE_POINTS: (u32, u32) = (0xD800, 0xDFFF);

/// The message of the error CHR raises for a code point no character has.
pub const CHARACTER_CODE_MESSAGE: &str = "Invalid character code in the CHR input";

/// What INITCAP capitalises after without a delimiter argument, as recorded.
pub const INITCAP_DEFAULT_DELIMITERS: &str = " _-";
/// HEX_ENCODE's case argument for lower-case digits.
pub const HEX_LOWER_CASE: &str = "0";
/// HEX_ENCODE's case argument for upper-case digits.
pub const HEX_UPPER_CASE: &str = "1";
