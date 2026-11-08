//! The lexer's operator tokens and blanks.

/// Operators of more than one character, longest first so the longest match wins.
pub const MULTI_CHAR_OPERATORS: [&str; 15] = [
    "<=>", "?::", "<<", "<=", "<>", ">=", "!=", "!<", "!>", "==", "||", "|>", "::", "->", "=>",
];

/// Characters `WS` skips beyond ASCII blanks.
pub const UNICODE_BLANKS: [char; 18] = [
    '\u{00A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}',
    '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}', '\u{2028}', '\u{202F}', '\u{205F}',
    '\u{3000}', '\u{000B}',
];

/// The message for an unterminated `/* ... */` comment.
pub const UNCLOSED_COMMENT: &str = "Unclosed bracketed comment";
