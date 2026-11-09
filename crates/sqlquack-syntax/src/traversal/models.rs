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
