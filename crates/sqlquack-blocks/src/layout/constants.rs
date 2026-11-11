//! The layout's line width and indentation.

/// Lines longer than this many characters are broken where the formatter can break them.
pub const LINE_WIDTH: usize = 100;

/// Spaces per level of indentation.
pub const INDENT: usize = 4;

/// The characters that open and close a quoted string or identifier, inside which case is kept.
pub const QUOTES: [char; 2] = ['\'', '"'];
