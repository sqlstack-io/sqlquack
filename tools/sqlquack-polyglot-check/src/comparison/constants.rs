//! Serialised tree keys the faithful comparison reads.

/// A plain function call.
pub const FUNCTION_KEY: &str = "function";
/// An aggregate function call.
pub const AGGREGATE_KEY: &str = "aggregate_function";
/// The node kind recorded when polyglot rejects a call SQLQuack keeps.
pub const ERROR_KIND: &str = "error";
/// Prefix that makes a call's text a statement.
pub const SELECT_PREFIX: &str = "SELECT ";
/// A node annotated with comments or a type.
pub const ANNOTATED_KEY: &str = "annotated";
/// The serialised name of a variant path node.
pub const JSON_EXTRACT_KEY: &str = "j_s_o_n_extract";
/// The suffix of every field holding comments.
pub const COMMENTS_SUFFIX: &str = "comments";

/// The field a located parse records a node's source range in.
pub const SPAN_FIELD: &str = "span";
