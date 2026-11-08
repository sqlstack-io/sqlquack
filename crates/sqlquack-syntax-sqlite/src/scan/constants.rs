//! The tokenizer's character classes and messages (`tokenize.c`, `global.c`).

/// `SQLITE_DIGIT_SEPARATOR`: the character a number may hold between two digits.
pub const DIGIT_SEPARATOR: u8 = b'_';

/// The message for text the tokenizer cannot read, before the text in double quotes.
pub const UNRECOGNIZED_TOKEN: &str = "unrecognized token:";

/// The first byte of a multi-byte UTF-8 character, which identifiers may hold.
pub const FIRST_NON_ASCII: u8 = 0x80;

/// The bytes of a UTF-8 byte order mark, which the tokenizer reads as a space.
pub const BYTE_ORDER_MARK: [u8; 3] = [0xEF, 0xBB, 0xBF];
