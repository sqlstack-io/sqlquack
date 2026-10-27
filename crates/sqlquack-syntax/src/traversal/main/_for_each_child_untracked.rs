//! `for_each_child_untracked`.

use crate::expressions::tree::models::Expression;
use crate::traversal::types::AstNode;

pub(in crate::traversal) fn for_each_child_untracked<'ast>(
    expression: &'ast Expression,
    mut visitor: impl FnMut(&'ast Expression),
) {
    expression.visit_syntax_untracked(&mut visitor, &mut |_| {});
}
