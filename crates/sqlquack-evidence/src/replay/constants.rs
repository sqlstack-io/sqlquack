//! The Python replay's runtime and files.

/// SQLQuack's Python runtime, which registers Python functions and replays through them.
pub const PYTHON_RUNTIME_SOURCE: &str = include_str!("python/sqlquack_runtime.py");
/// The runtime's file in a Python replay's directory.
pub const PYTHON_REPLAY_SCRIPT: &str = "sqlquack_runtime.py";
/// The request a Python replay reads.
pub const PYTHON_REPLAY_REQUEST: &str = "request.json";
/// The database a Python replay stores its rows in.
pub const PYTHON_REPLAY_DATABASE: &str = "result.duckdb";
/// The file a Python replay writes DuckDB's error to.
pub const PYTHON_REPLAY_ERROR: &str = "error.txt";
/// The table holding a Python replay's rows, in order.
pub const PYTHON_REPLAY_RESULT: &str = "result";
/// The parse every override body's `SELECT (<body>)` must match but for its select item.
pub const EXPRESSION_TEMPLATE: &str = "SELECT (NULL)";
/// The serialised select node's list of select items.
pub const SELECT_ITEMS: &str = "select_list";
