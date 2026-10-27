use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::LikeOp;
use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::traversal::main::_for_each_child_untracked::for_each_child_untracked;
use crate::traversal::main::for_each_child::for_each_child;
use crate::traversal::models::ChildPathSegment;
use crate::traversal::types::ExpressionWalk;

pub(crate) fn statement(sql: &str) -> Expression {
    Parser::parse_sql(sql).expect("the query parses").remove(0)
}

pub(crate) fn label(expression: &Expression) -> String {
    format!("{}:{}", expression.variant_name(), expression.name())
}

pub(crate) fn labels<'a>(expressions: impl IntoIterator<Item = &'a Expression>) -> Vec<String> {
    expressions.into_iter().map(label).collect()
}

pub(crate) fn like_with_escape() -> Expression {
    Expression::Like(Box::new(LikeOp {
        negated: false,
        left: Expression::column("name"),
        right: Expression::string("x%"),
        escape: Some(Expression::string("!")),
        quantifier: None,
        inferred_type: None,
        span: SourceSpan::default(),
    }))
}

pub(crate) fn table_with_hint_and_identifier_function() -> Expression {
    let mut table = TableRef::new("events");
    table.hints.push(Expression::column("table_hint"));
    table.identifier_func = Some(Box::new(Expression::identifier("dynamic_table")));
    Expression::Table(Box::new(table))
}

pub(crate) fn walk_summary(expression: &Expression, kind: &str) -> (Option<String>, usize, bool) {
    let is_kind = |node: &Expression| node.variant_name() == kind;
    (
        expression.find(is_kind).map(label),
        expression.find_all(is_kind).len(),
        expression.contains(is_kind),
    )
}

pub(crate) fn child_paths(expression: &Expression) -> Vec<Vec<ChildPathSegment>> {
    let mut paths: Vec<Vec<ChildPathSegment>> = Vec::new();
    for_each_child(expression, |path, _| paths.push(path.to_vec()));
    paths
}

pub(crate) fn tracked_and_untracked_children(
    expression: &Expression,
) -> (Vec<*const Expression>, Vec<*const Expression>) {
    let mut tracked: Vec<*const Expression> = Vec::new();
    for_each_child(expression, |_, child| tracked.push(child));
    let mut untracked: Vec<*const Expression> = Vec::new();
    for_each_child_untracked(expression, |child| untracked.push(child));
    (tracked, untracked)
}
