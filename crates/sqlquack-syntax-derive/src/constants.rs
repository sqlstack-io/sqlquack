//! Names the derive matches in the annotated source.

/// The syntax tree's expression type, whose variant names are also listed.
pub(crate) const EXPRESSION_TYPE: &str = "Expression";

/// The `#[ast(...)]` argument that leaves a field out of traversal.
pub(crate) const SKIP_MARKER: &str = "skip";

/// The field a node keeps its recorded source range in.
pub(crate) const SPAN_FIELD: &str = "span";
