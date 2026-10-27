//! Type aliases and traits of traversal.

use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::DataType;
use crate::tokens::models::Span;
use crate::traversal::models::BfsIter;
use crate::traversal::models::ChildPathSegment;
use crate::traversal::models::DfsIter;

pub(crate) trait AstNode {
    /// Visits expression children and nested syntax types; expression descendants are the caller's.
    fn visit_syntax_untracked<'ast, F, T>(&'ast self, visitor: &mut F, type_visitor: &mut T)
    where
        F: FnMut(&'ast Expression),
        T: FnMut(&'ast DataType);

    fn visit_expressions<'ast, F>(&'ast self, path: &mut Vec<ChildPathSegment>, visitor: &mut F)
    where
        F: FnMut(&[ChildPathSegment], &'ast Expression);

    /// SQLQuack: the node's own recorded source range, when it has a `span` field holding one.
    fn recorded_span(&self) -> Option<Span> {
        None
    }

    /// SQLQuack: records the node's source range in its `span` field unless one is recorded.
    fn fill_span(&mut self, _span: Span) {}
}

/// Traversal and search methods on [`Expression`].
pub trait ExpressionWalk {
    /// A pre-order depth-first iterator over this expression and its descendants.
    fn dfs(&self) -> DfsIter<'_>;

    /// A level-order breadth-first iterator over this expression and its descendants.
    fn bfs(&self) -> BfsIter<'_>;

    /// The first expression matching `predicate`, depth first, this node included.
    fn find<F>(&self, predicate: F) -> Option<&Expression>
    where
        F: Fn(&Expression) -> bool;

    /// Every expression matching `predicate`, depth first.
    fn find_all<F>(&self, predicate: F) -> Vec<&Expression>
    where
        F: Fn(&Expression) -> bool;

    /// Returns `true` if this node or any descendant matches `predicate`.
    fn contains<F>(&self, predicate: F) -> bool
    where
        F: Fn(&Expression) -> bool;

    /// Counts how many nodes (including this one) match `predicate`.
    fn count<F>(&self, predicate: F) -> usize
    where
        F: Fn(&Expression) -> bool;

    /// This node's direct child expressions; leaves have none.
    fn children(&self) -> Vec<&Expression>;

    /// The depth of the tree rooted here; a leaf has depth 0.
    fn tree_depth(&self) -> usize;
}
