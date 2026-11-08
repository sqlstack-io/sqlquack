//! The characters ClickHouse's lexer treats specially beyond ASCII.

/// Characters `skipWhitespacesUTF8` skips as blanks beyond ASCII ones, the byte-order mark included.
pub const UNICODE_BLANKS: [char; 24] = [
    '\u{0085}', '\u{00A0}', '\u{180E}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}',
    '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{200B}', '\u{200C}',
    '\u{200D}', '\u{2028}', '\u{2029}', '\u{202F}', '\u{205F}', '\u{2060}', '\u{3000}', '\u{FEFF}',
];

/// The mathematical minus sign, which lexes as `-`.
pub const UNICODE_MINUS: char = '\u{2212}';

/// The opening and closing quotes of a `‘string’`.
pub const UNICODE_SINGLE_QUOTES: (char, char) = ('\u{2018}', '\u{2019}');
/// The opening and closing quotes of a `“name”`.
pub const UNICODE_DOUBLE_QUOTES: (char, char) = ('\u{201C}', '\u{201D}');

/// The fewest bytes a `‘...’` or `“...”` token may span from its start for the lexer to try it.
pub const UNICODE_QUOTED_MIN_BYTES: usize = 6;

/// How many digits a `\xHH` escape has.
pub const HEX_ESCAPE_DIGITS: usize = 2;
/// The base of a `\xHH` escape's digits.
pub const HEX_RADIX: u32 = 16;
