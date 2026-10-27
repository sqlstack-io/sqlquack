//! Traversal: walk.

use crate::expressions::tree::models::Expression;
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
        crate::traversal::main::for_each_child::for_each_child(self, |_, child| result.push(child));
        result
    }

    fn tree_depth(&self) -> usize {
        let mut max_depth = 0usize;
        let mut stack = vec![(self, 0usize)];
        while let Some((node, depth)) = stack.pop() {
            max_depth = max_depth.max(depth);
            crate::traversal::main::for_each_child::for_each_child(node, |_, child| {
                stack.push((child, depth + 1));
            });
        }
        max_depth
    }
}
