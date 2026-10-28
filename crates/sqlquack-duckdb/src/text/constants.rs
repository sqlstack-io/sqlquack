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
