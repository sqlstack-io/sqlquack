//! The tokenizer's character classes and messages (`base_tokenizer.cpp`, `parser_tokenizer.cpp`).

/// Characters that are a token on their own (`IsSingleByteOperator`).
pub const SINGLE_BYTE_OPERATORS: [u8; 11] = *b"(){}[],:?$#";

/// Characters that continue an operator (`Tokenizer::CharacterIsOperator`).
pub const OPERATOR_CHARACTERS: [u8; 14] = *b"+-*/%^<>=~!@&|";

/// Characters that let an operator end in `+` or `-` (PostgreSQL's trimming rule).
pub const OPERATOR_SPECIALS: [u8; 10] = *b"~!@#%^&|`?";

/// Letters that, directly before a `'`, open a prefixed string (`CharacterIsSpecialStringCharacter`).
pub const STRING_PREFIXES: [u8; 8] = *b"NnXxEeBb";

/// The message for an unterminated `'` string.
pub const UNTERMINATED_STRING: &str = "unterminated string literal";

/// The message for an unterminated quoted identifier.
pub const UNTERMINATED_IDENTIFIER: &str = "unterminated quoted identifier";

/// The message for `""` or ``` `` ```.
pub const EMPTY_IDENTIFIER: &str = "zero-length delimited identifier";

/// The message for an unterminated `/*` comment, before the comment's text.
pub const UNTERMINATED_COMMENT: &str = "unterminated /* comment at or near";

/// The first byte of a multi-byte UTF-8 character, which a `$tag$` may hold.
pub const FIRST_NON_ASCII: u8 = 0x80;
