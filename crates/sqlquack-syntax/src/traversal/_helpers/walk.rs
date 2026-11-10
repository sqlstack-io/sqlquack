//! Traversal: walk.

use crate::expressions::tree::models::Expression;
use crate::traversal::main::_for_each_child_untracked::for_each_child_untracked;
use crate::traversal::models::BfsIter;
use crate::traversal::models::DfsIter;
use crate::traversal::types::ExpressionWalk;

impl ExpressionWalk for Expression {
    fn dfs(&self) -> DfsIter<'_> {
        DfsIter::new(self)
    }

    fn bfs(&self) -> BfsIter<'_> {
        BfsIter::new(self)
    }

    fn find<F>(&self, predicate: F) -> Option<&Expression>
    where
        F: Fn(&Expression) -> bool,
    {
        self.dfs().find(|e| predicate(e))
    }

    fn find_all<F>(&self, predicate: F) -> Vec<&Expression>
    where
        F: Fn(&Expression) -> bool,
    {
        self.dfs().filter(|e| predicate(e)).collect()
    }

    fn contains<F>(&self, predicate: F) -> bool
    where
        F: Fn(&Expression) -> bool,
    {
        self.dfs().any(predicate)
    }

    fn count<F>(&self, predicate: F) -> usize
    where
        F: Fn(&Expression) -> bool,
    {
        self.dfs().filter(|e| predicate(e)).count()
    }

    fn children(&self) -> Vec<&Expression> {
        let mut result: Vec<&Expression> = Vec::new();
        for_each_child_untracked(self, |child| result.push(child));
        result
    }

    fn tree_depth(&self) -> usize {
        let mut max_depth = 0usize;
        let mut stack = vec![(self, 0usize)];
        while let Some((node, depth)) = stack.pop() {
            max_depth = max_depth.max(depth);
            for_each_child_untracked(node, |child| stack.push((child, depth + 1)));
        }
        max_depth
    }
}
