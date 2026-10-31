//! Text building block identifiers.

/// Levenshtein distance counted in characters: `duckdb.character_edit_distance`.
pub const CHARACTER_EDIT_DISTANCE: &str = "duckdb.character_edit_distance";
/// Building block: Jaro-Winkler similarity over lower-cased characters.
pub const CHARACTER_SIMILARITY: &str = "duckdb.character_similarity";
/// Text similarity whose answer no recording shows: case folding outside ASCII, or wide alphabets.
pub const TEXT_SIMILARITY: &str = "text_similarity";
/// How many distinct characters a pair may hold, each encoded as one ASCII byte from 1.
pub const SIMILARITY_ALPHABET: usize = 127;

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

/// TRY_BASE64_DECODE_BINARY's bytes, as upper-case hexadecimal digits: `duckdb.base64_bytes`.
pub const BASE64_BYTES: &str = "duckdb.base64_bytes";

/// Bytes held as hexadecimal digits, read as UTF-8 text: `duckdb.bytes_text`.
pub const BYTES_TEXT: &str = "duckdb.bytes_text";

/// SUBSTR of bytes held as hexadecimal digits, counted in bytes: `duckdb.bytes_substring`.
pub const BYTES_SUBSTRING: &str = "duckdb.bytes_substring";

/// ILIKE's text with letters no recording shows Snowflake folding: `duckdb.folded_text`.
pub const FOLDED_TEXT: &str = "duckdb.folded_text";

/// The divergence of case-insensitive text holding letters whose folding no recording shows.
pub const CASE_FOLDING: &str = "case_folding";

/// The long s, which Snowflake's ILIKE was recorded folding onto s, as DuckDB's lower does not.
pub const LONG_S: char = '\u{17f}';

/// Code point ranges whose case folding differs from DuckDB's lower-casing, or DuckDB's from Unicode's.
pub const UNFOLDED_LETTERS: &[(u32, u32)] = &[
    (0xB5, 0xB5),
    (0x130, 0x130),
    (0x149, 0x149),
    (0x1F0, 0x1F0),
    (0x345, 0x345),
    (0x390, 0x390),
    (0x3B0, 0x3B0),
    (0x3C2, 0x3C2),
    (0x3D0, 0x3D1),
    (0x3D5, 0x3D6),
    (0x3F0, 0x3F1),
    (0x3F5, 0x3F5),
    (0x587, 0x587),
    (0x13A0, 0x13F5),
    (0x13F8, 0x13FD),
    (0x1C80, 0x1C89),
    (0x1E96, 0x1E9B),
    (0x1F50, 0x1F50),
    (0x1F52, 0x1F52),
    (0x1F54, 0x1F54),
    (0x1F56, 0x1F56),
    (0x1F80, 0x1FAF),
    (0x1FB2, 0x1FB4),
    (0x1FB6, 0x1FB7),
    (0x1FBC, 0x1FBC),
    (0x1FBE, 0x1FBE),
    (0x1FC2, 0x1FC4),
    (0x1FC6, 0x1FC7),
    (0x1FCC, 0x1FCC),
    (0x1FD2, 0x1FD3),
    (0x1FD6, 0x1FD7),
    (0x1FE2, 0x1FE4),
    (0x1FE6, 0x1FE7),
    (0x1FF2, 0x1FF4),
    (0x1FF6, 0x1FF7),
    (0x1FFC, 0x1FFC),
    (0xA7CB, 0xA7CC),
    (0xA7DA, 0xA7DA),
    (0xA7DC, 0xA7DC),
    (0xAB70, 0xABBF),
    (0xFB00, 0xFB06),
    (0xFB13, 0xFB17),
    (0x10D50, 0x10D65),
];

/// Base64 Snowflake decodes: whole groups of four, the last padded with `=` to its length.
pub const BASE64_PATTERN: &str = "(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?";

/// The message for bytes that are not UTF-8 read as UTF-8 text.
pub const INVALID_UTF8_MESSAGE: &str = "Invalid UTF8 detected while decoding bytes as text";

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
