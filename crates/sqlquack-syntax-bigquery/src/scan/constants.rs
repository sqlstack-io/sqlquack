//! The lexer's operator tokens and blanks.

/// Operators of more than one character, longest first; `>>` is absent, as `>` is always one token.
pub const MULTI_CHAR_OPERATORS: [&str; 15] = [
    "?::", "<<", "<=", "<>", ">=", "!=", "||", "|>", "::", "->", "=>", "+=", "-=", "@@", "**",
];

/// `whitespace_character` beyond ASCII blanks; U+1680, U+180E, U+200B and U+FEFF are not blanks.
pub const UNICODE_BLANKS: [char; 15] = [
    '\u{00A0}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}',
    '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{202F}', '\u{205F}', '\u{3000}',
];

/// The message for an unterminated `/* ... */` comment.
pub const UNCLOSED_COMMENT: &str = "Syntax error: Unclosed comment";

/// The message for a number with a name written straight after it, such as `123abc`.
pub const ATTACHED_ALIAS: &str = "Syntax error: Missing whitespace between literal and alias";
