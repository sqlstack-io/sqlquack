use crate::expressions::names::models::TableRef;
use crate::expressions::queries::models::{From, Select, Where};
use crate::expressions::tree::models::Expression;
use crate::traversal::main::_for_each_child_untracked::for_each_child_untracked;
use crate::traversal::main::for_each_child::for_each_child;

/// `SELECT a, b FROM t WHERE e`, built by hand: four children.
pub(crate) fn select_from_where() -> Expression {
    let mut select = Select::new();
    select.expressions = vec![Expression::column("a"), Expression::column("b")];
    select.from = Some(From {
        expressions: vec![Expression::Table(Box::new(TableRef::new("t")))],
    });
    select.where_clause = Some(Where {
        this: Expression::column("e"),
    });
    Expression::Select(Box::new(select))
}

pub(crate) fn tracked_and_untracked_children(
    expression: &Expression,
) -> (Vec<*const Expression>, Vec<*const Expression>, usize) {
    let mut tracked: Vec<*const Expression> = Vec::new();
    let mut paths = 0;
    for_each_child(expression, |_, child| {
        tracked.push(child);
        paths += 1;
    });
    let mut untracked: Vec<*const Expression> = Vec::new();
    for_each_child_untracked(expression, |child| untracked.push(child));
    (tracked, untracked, paths)
}
