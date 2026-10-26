//! Census defaults and the placeholders that stand in for sqlbuild references.

/// Rule counts at which the convertible share is reported.
pub const COVERAGE_POINTS: &[usize] = &[10, 25, 50, 75, 100, 150, 200, 250];

/// Constructs listed in the text report unless asked otherwise.
pub const DEFAULT_TEXT_LIMIT: usize = 50;

/// Stack for each census worker. Polyglot's trees and their JSON form are deep for generated SQL.
pub const WORKER_STACK_BYTES: usize = 256 * 1024 * 1024;

/// The table that stands in for sqlbuild model, source, seed and fixture references.
pub const RELATION_PLACEHOLDER: &str = "__sqlquack_relation";

/// The table function that stands in for sqlbuild table-function references.
pub const TABLE_FUNCTION_PLACEHOLDER: &str = "__sqlquack_table_fn";

/// The column that stands in for sqlbuild cursor bounds.
pub const CURSOR_PLACEHOLDER: &str = "__sqlquack_cursor";

/// sqlbuild reference calls replaced by a relation.
pub const RELATION_CALLS: &[&str] = &[
    "__ref",
    "__source",
    "__seed",
    "__dbt_ref",
    "__empty_fixture",
];

/// sqlbuild cursor intrinsics replaced by a column.
pub const CURSOR_CALLS: &[&str] = &["__cursor_start", "__cursor_end"];

/// When set, census writes each converted file's known divergences there as `<path>.json`.
pub const GUARDS_DIR_VARIABLE: &str = "SQLQUACK_GUARDS_DIR";
