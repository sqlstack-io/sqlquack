//! How rendered parse results begin.

/// polyglot's rendered error starts with this.
pub const ERROR_PREFIX: &str = "err:";
/// SQLQuack's rendered tree starts with this.
pub const OK_PREFIX: &str = "ok:";
/// The field naming an identifier in a serialised tree.
pub const NAME_KEY: &str = "name";
/// The field recording whether an identifier was quoted.
pub const QUOTED_KEY: &str = "quoted";
