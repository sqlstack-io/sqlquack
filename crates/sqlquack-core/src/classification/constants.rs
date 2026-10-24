//! Reserved names shared with callers that rewrite project-specific calls before parsing.

/// The function name callers substitute for project user-defined functions.
pub const PROJECT_FUNCTION_PLACEHOLDER: &str = "__sqlquack_udf";

/// The construct name of a project user-defined function call.
pub const PROJECT_FUNCTION_CONSTRUCT: &str = "<udf>";
