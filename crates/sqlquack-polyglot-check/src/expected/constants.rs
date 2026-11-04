//! How rendered parse results begin.

/// polyglot's rendered error starts with this.
pub const ERROR_PREFIX: &str = "err:";
/// SQLQuack's rendered tree starts with this.
pub const OK_PREFIX: &str = "ok:";
/// How a rendered tree of one opaque command begins after the prefix.
pub const COMMAND_TREE: &str = "[{\"command\":";
/// The field naming an identifier in a serialised tree.
pub const NAME_KEY: &str = "name";
/// The field recording whether an identifier was quoted.
pub const QUOTED_KEY: &str = "quoted";
