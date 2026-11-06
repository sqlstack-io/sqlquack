//! How rendered parse results begin.

/// polyglot's rendered error starts with this.
pub const ERROR_PREFIX: &str = "err:";
/// SQLQuack's rendered tree starts with this.
pub const OK_PREFIX: &str = "ok:";
/// How a rendered tree of one opaque command begins after the prefix.
pub const COMMAND_TREE: &str = "[{\"command\":";
/// How a rendered tree of one statement polyglot keeps as raw SQL begins after the prefix.
pub const RAW_TREE: &str = "[{\"raw\":";
/// The field naming an identifier in a serialised tree.
pub const NAME_KEY: &str = "name";
/// The field holding the operand of a wrapper node such as unary plus.
pub const THIS_KEY: &str = "this";
/// The field recording whether an identifier was quoted.
pub const QUOTED_KEY: &str = "quoted";
/// The field of a clause SQLQuack sets for ALL, beside an empty `expressions`.
pub const ALL_KEY: &str = "all";
/// The field listing a clause's keys.
pub const EXPRESSIONS_KEY: &str = "expressions";
/// The text polyglot gives ALL when it reads ALL as a key.
pub const ALL_TEXT: &str = "ALL";
/// The node polyglot reads a bare word such as ALL into.
pub const VAR_KEY: &str = "var";
