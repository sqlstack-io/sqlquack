//! Reserved names shared with callers that rewrite project-specific calls before parsing.

/// The function name callers substitute for project user-defined functions.
pub const PROJECT_FUNCTION_PLACEHOLDER: &str = "__sqlquack_udf";

/// The construct name of a project user-defined function call.
pub const PROJECT_FUNCTION_CONSTRUCT: &str = "<udf>";
/// The name callers substitute for a microbatch cursor bound they cannot render.
pub const CURSOR_PLACEHOLDER: &str = "__sqlquack_cursor";
/// The construct name of an unrendered microbatch cursor bound.
pub const CURSOR_CONSTRUCT: &str = "<cursor>";

/// AST node kinds as SQL readers name them; other kinds read as their name in upper case.
pub const PLAIN_NODE_NAMES: &[(&str, &str)] = &[
    ("add", "+"),
    ("sub", "-"),
    ("mul", "*"),
    ("div", "/"),
    ("mod", "%"),
    ("eq", "="),
    ("neq", "<>"),
    ("lt", "<"),
    ("lte", "<="),
    ("gt", ">"),
    ("gte", ">="),
    ("null_safe_eq", "IS NOT DISTINCT FROM"),
    ("null_safe_neq", "IS DISTINCT FROM"),
    ("i_like", "ILIKE"),
    ("try_cast", "TRY_CAST"),
    ("if_func", "IFF"),
    ("date_diff", "DATEDIFF"),
    ("map_func", "MAP"),
    ("array_func", "ARRAY"),
    ("window", "OVER"),
    ("window_function", "a window function"),
    ("ordered", "ORDER BY"),
    ("column", "a column"),
    ("subquery", "a subquery"),
];
