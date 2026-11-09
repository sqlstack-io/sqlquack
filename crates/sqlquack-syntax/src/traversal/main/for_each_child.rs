//! `for_each_child`.

use crate::expressions::tree::models::Expression;
use crate::traversal::models::ChildPathSegment;
use crate::traversal::types::AstNode;

pub fn for_each_child<'ast>(
    expression: &'ast Expression,
    mut visitor: impl FnMut(&[ChildPathSegment], &'ast Expression),
) {
    let mut path: Vec<ChildPathSegment> = Vec::new();
    expression.visit_expressions(&mut path, &mut visitor);
}
