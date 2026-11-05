//! Layout constants, SQLClean's defaults.

/// Spaces per indent step.
pub const INDENT_WIDTH: usize = 4;

/// The width a line fills before a bracket on it is broken over lines.
pub const LINE_WIDTH: usize = 100;

/// A SELECT list of at least this many items puts one item per line.
pub const SELECT_ITEMS_BROKEN: usize = 2;
