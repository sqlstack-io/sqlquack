//! Character classes and limits of PostgreSQL's scanner (scan.l).

/// `op_chars`: characters an operator is made of.
pub const OP_CHARS: &str = "~!@#^&|`?+-*/%<>=";
/// Characters that let an operator end in `+` or `-`.
pub const OP_KEEPS_SIGN: &str = "~!@#^&|`?%";
/// One-character operators the grammar names itself; any other is `Op`.
pub const SINGLE_OPERATORS: &str = "+-*/%^<>=";
/// `NAMEDATALEN`: names and operators are shorter than this many bytes.
pub const NAMEDATALEN: usize = 64;
/// The word that may follow a Unicode-escaped string or identifier.
pub const UESCAPE: &str = "uescape";
/// The first byte value of a multi-byte character, which may start an identifier.
pub const FIRST_NON_ASCII: u8 = 0x80;
/// The largest `int4`: a bigger integer literal is a `numeric` constant.
pub const INT4_MAX: u32 = 2_147_483_647;
/// The base of decimal digits.
pub const DECIMAL_RADIX: u32 = 10;
/// The base of hexadecimal digits.
pub const HEX_RADIX: u32 = 16;
/// Characters a `UESCAPE` character may not be.
pub const UESCAPE_FORBIDDEN: &str = "0123456789abcdefABCDEF+'\" \t\n\r\x0c";
