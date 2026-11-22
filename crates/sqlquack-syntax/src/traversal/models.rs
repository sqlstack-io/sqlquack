//! Data models of traversal.

use std::collections::VecDeque;

use crate::expressions::tree::models::Expression;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildPathSegment {
    Field(&'static str),
    Index(usize),
}

/// Pre-order depth-first iterator over an expression tree.
#[derive(Debug)]
pub struct DfsIter<'a> {
    pub(crate) stack: Vec<&'a Expression>,
}

impl<'a> DfsIter<'a> {
    /// Create a new DFS iterator starting from the given expression
    pub fn new(root: &'a Expression) -> Self {
        Self { stack: vec![root] }
    }
}

impl<'a> Iterator for DfsIter<'a> {
    type Item = &'a Expression;

    fn next(&mut self) -> Option<Self::Item> {
        let expr = self.stack.pop()?;

        let child_start = self.stack.len();
        crate::traversal::main::_for_each_child_untracked::for_each_child_untracked(
            expr,
            |child| self.stack.push(child),
        );
        self.stack[child_start..].reverse();

        Some(expr)
    }
}

/// Level-order breadth-first iterator over an expression tree.
#[derive(Debug)]
pub struct BfsIter<'a> {
    pub(crate) queue: VecDeque<&'a Expression>,
}

impl<'a> BfsIter<'a> {
    /// Create a new BFS iterator starting from the given expression
    pub fn new(root: &'a Expression) -> Self {
        let mut queue: VecDeque<&Expression> = VecDeque::new();
        queue.push_back(root);
        Self { queue }
    }
}

impl<'a> Iterator for BfsIter<'a> {
    type Item = &'a Expression;

    fn next(&mut self) -> Option<Self::Item> {
        let expr = self.queue.pop_front()?;

        crate::traversal::main::_for_each_child_untracked::for_each_child_untracked(
            expr,
            |child| self.queue.push_back(child),
        );

        Some(expr)
    }
}

/// SQLQuack: which set operator a node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetOperator {
    Union,
    Intersect,
    /// EXCEPT, or MINUS.
    Except,
}

/// SQLQuack: one set operation, read the same for each operator.
#[derive(Debug, Clone, Copy)]
pub struct SetOperation<'a> {
    pub operator: SetOperator,
    pub left: &'a Expression,
    pub right: &'a Expression,
    /// Columns pair by name (BY NAME or CORRESPONDING), so the sides' counts may differ.
    pub by_name: bool,
    pub all: bool,
    /// DISTINCT is written.
    pub distinct: bool,
    /// The operator keyword, in a located parse.
    pub operator_span: crate::tokens::models::SourceSpan,
}

/// SQLQuack: what a FROM item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FromItemKind {
    /// A named relation.
    Relation,
    /// A query or VALUES in parentheses, which cannot read the items before it.
    Derived,
    /// LATERAL is written.
    Lateral,
    /// A function in FROM, or Snowflake's TABLE(...) around a call.
    TableFunction,
    /// UNNEST, which reads the items before it only when its argument names a column.
    Unnest { correlated: bool },
    /// An array path through an earlier item, such as `t.arr`.
    ArrayPath,
}

/// SQLQuack: a FROM item written after a comma.
#[derive(Debug, Clone, Copy)]
pub struct CommaItem<'a> {
    pub item: &'a Expression,
    pub kind: FromItemKind,
    /// The comma before the item, in a located parse.
    pub comma: crate::tokens::models::SourceSpan,
}

impl CommaItem<'_> {
    /// Whether the item may read the items before it, so its comma is no cartesian join.
    pub fn may_read_earlier(&self) -> bool {
        matches!(
            self.kind,
            FromItemKind::Lateral
                | FromItemKind::TableFunction
                | FromItemKind::ArrayPath
                | FromItemKind::Unnest { correlated: true }
        )
    }
}
