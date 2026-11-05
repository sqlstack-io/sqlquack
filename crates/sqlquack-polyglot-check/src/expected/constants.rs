//! How rendered parse results begin.

/// polyglot's rendered error starts with this.
pub const ERROR_PREFIX: &str = "err:";
/// SQLQuack's rendered tree starts with this.
pub const OK_PREFIX: &str = "ok:";
/// How a rendered tree of one opaque command begins after the prefix.
pub const COMMAND_TREE: &str = "[{\"command\":";
/// How a rendered tree of one opaque raw statement begins after the prefix.
pub const RAW_TREE: &str = "[{\"raw\":";
/// The field naming an identifier in a serialised tree.
pub const NAME_KEY: &str = "name";
/// The field holding the operand of a wrapper node such as unary plus.
pub const THIS_KEY: &str = "this";
/// The field recording whether an identifier was quoted.
pub const QUOTED_KEY: &str = "quoted";
