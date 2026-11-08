//! The lexer's character classes and messages, as SQL Server 2022 reports them.

/// Operators of two characters, longest first where one prefixes another.
pub const TWO_CHARACTER_OPERATORS: [&str; 15] = [
    "<=", ">=", "<>", "!=", "!<", "!>", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "::",
];

/// Symbols that start a `money` literal (the engine's currency symbols).
pub const CURRENCY_SYMBOLS: &str = "$\u{a2}\u{a3}\u{a4}\u{a5}\u{9f2}\u{9f3}\u{e3f}\u{17db}\u{fdfc}\u{fe69}\u{ff04}\u{ffe0}\u{ffe1}\u{ffe5}\u{ffe6}";

/// The first and last of the contiguous currency block (`U+20A0` to `U+20B1`) that also start `money`.
pub const CURRENCY_BLOCK: (char, char) = ('\u{20a0}', '\u{20b1}');

/// Error 105, for a string or delimited identifier that never closes, before its text.
pub const UNCLOSED_QUOTATION: &str = "Unclosed quotation mark after the character string";

/// Error 113, for a `/*` comment that never closes.
pub const MISSING_COMMENT_END: &str = "Missing end comment mark '*/'.";

/// The engine's error number for an unclosed quotation mark.
pub const UNCLOSED_QUOTATION_CODE: u32 = 105;

/// The engine's error number for a missing end comment mark.
pub const MISSING_COMMENT_END_CODE: u32 = 113;
