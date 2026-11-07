//! Tree helpers: which nodes open a scope, and the children a scope walk visits.

use sqlquack_syntax::expressions::queries::models::{OrderBy, Select};
use sqlquack_syntax::{Expression, ExpressionWalk};

/// Whether `expression` opens a nested scope: a CTE, a derived table, or a query.
pub(crate) fn opens_scope(expression: &Expression) -> bool {
    match expression {
        Expression::Cte(_)
        | Expression::Select(_)
        | Expression::Union(_)
        | Expression::Intersect(_)
        | Expression::Except(_) => true,
        Expression::Subquery(subquery) => subquery.alias.is_some(),
        _ => false,
    }
}

/// The children a scope walk visits below `expression`, before nested scopes are dropped.
pub(crate) fn scope_children(expression: &Expression) -> Vec<&Expression> {
    match expression {
        Expression::Prepare(prepare) => vec![&prepare.statement],
        Expression::Select(select) => select_children(select),
        Expression::And(operation)
        | Expression::Or(operation)
        | Expression::Add(operation)
        | Expression::Sub(operation)
        | Expression::Mul(operation)
        | Expression::Div(operation)
        | Expression::Mod(operation)
        | Expression::Eq(operation)
        | Expression::Neq(operation)
        | Expression::Lt(operation)
        | Expression::Lte(operation)
        | Expression::Gt(operation)
        | Expression::Gte(operation)
        | Expression::BitwiseAnd(operation)
        | Expression::BitwiseOr(operation)
        | Expression::BitwiseXor(operation)
        | Expression::Concat(operation) => vec![&operation.left, &operation.right],
        Expression::Like(like) | Expression::ILike(like) => {
            let mut children: Vec<&Expression> = vec![&like.left, &like.right];
            children.extend(like.escape.as_ref());
            children
        }
        Expression::Not(unary) | Expression::Neg(unary) | Expression::BitwiseNot(unary) => {
            vec![&unary.this]
        }
        Expression::Function(function) => function.args.iter().collect(),
        Expression::AggregateFunction(function) => function.args.iter().collect(),
        Expression::WindowFunction(window) => {
            let mut children: Vec<&Expression> = vec![&window.this];
            children.extend(window.over.partition_by.iter());
            children.extend(window.over.order_by.iter().map(|ordered| &ordered.this));
            children
        }
        Expression::Alias(alias) => vec![&alias.this],
        Expression::Case(case) => {
            let mut children: Vec<&Expression> = case.operand.iter().collect();
            for (condition, result) in &case.whens {
                children.extend([condition, result]);
            }
            children.extend(case.else_.as_ref());
            children
        }
        Expression::Paren(paren) => vec![&paren.this],
        Expression::Ordered(ordered) => vec![&ordered.this],
        Expression::In(membership) => {
            let mut children: Vec<&Expression> = vec![&membership.this];
            children.extend(membership.expressions.iter());
            children
        }
        Expression::Between(between) => vec![&between.this, &between.low, &between.high],
        Expression::IsNull(test) => vec![&test.this],
        Expression::Cast(cast) => vec![&cast.this],
        Expression::Extract(extract) => vec![&extract.this],
        Expression::Coalesce(coalesce) => coalesce.expressions.iter().collect(),
        Expression::NullIf(nullif) => vec![&nullif.this, &nullif.expression],
        Expression::TryCatch(try_catch) => {
            let mut children: Vec<&Expression> = try_catch.try_body.iter().collect();
            children.extend(try_catch.catch_body.iter().flatten());
            children
        }
        Expression::Table(_)
        | Expression::Column(_)
        | Expression::Literal(_)
        | Expression::Identifier(_)
        | Expression::Subquery(_)
        | Expression::Exists(_) => Vec::new(),
        other => other.children(),
    }
}

/// A SELECT's clause expressions, leaving out the relations that open nested scopes.
fn select_children(select: &Select) -> Vec<&Expression> {
    let mut children: Vec<&Expression> = select.expressions.iter().collect();
    for relation in select.from.iter().flat_map(|from| from.expressions.iter()) {
        children.extend((!opens_scope(relation)).then_some(relation));
    }
    for join in &select.joins {
        children.extend((!opens_scope(&join.this)).then_some(&join.this));
        children.extend(join.on.as_ref());
        children.extend(join.match_condition.as_ref());
    }
    children.extend(select.where_clause.as_ref().map(|clause| &clause.this));
    children.extend(
        select
            .group_by
            .iter()
            .flat_map(|group| group.expressions.iter()),
    );
    children.extend(select.having.as_ref().map(|having| &having.this));
    children.extend(select.qualify.as_ref().map(|qualify| &qualify.this));
    children.extend(select.order_by.iter().flat_map(ordered_terms));
    children.extend(select.limit.as_ref().map(|limit| &limit.this));
    children.extend(select.offset.as_ref().map(|offset| &offset.this));
    children
}

/// The expressions an ORDER BY sorts on.
fn ordered_terms(order: &OrderBy) -> impl Iterator<Item = &Expression> {
    order.expressions.iter().map(|ordered| &ordered.this)
}
