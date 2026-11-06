//! Character classes and limits of PostgreSQL's scanner (scan.l).

/// `op_chars`: characters an operator is made of.
pub const OP_CHARS: &str = "~!@#^&|`?+-*/%<>=";
/// Characters that let an operator end in `+` or `-`.
pub const OP_KEEPS_SIGN: &str = "~!@#^&|`?%";
/// One-character operators the grammar names itself; any other is `Op`.
pub const SINGLE_OPERATORS: &str = "+-*/%^<>=";
/// `NAMEDATALEN`: names and operators are shorter than this many bytes.
pub const NAMEDATALEN: usize = 64;
/// Characters a `UESCAPE` character may not be.
pub const UESCAPE_FORBIDDEN: &str = "0123456789abcdefABCDEF+'\" \t\n\r\x0c";
