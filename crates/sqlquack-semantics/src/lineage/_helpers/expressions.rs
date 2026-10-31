//! The lineage of a value expression in a query level.

use std::collections::BTreeSet;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Identifier, Star};
use sqlquack_syntax::expressions::operators::models::Parameter;

use crate::lineage::_helpers::lineages::{
    combined, constant, everything_read, traced_if, unresolved,
};
use crate::lineage::_helpers::queries::{expanded_star, resolved_query};
use crate::lineage::_helpers::resolution::{column_lineage, key, positional_column};
use crate::lineage::constants::{AGGREGATE_FUNCTIONS, POSITIONAL_COLUMN_BASE};
use crate::lineage::models::{
    Lineage, LineageContext, LineageScope, ResolvedQuery, SourceColumn, TransformKind,
};

/// The lineage of `expression` read in `scope`.
pub(crate) fn expression_lineage(
    expression: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Lineage {
    match expression {
        Expression::Column(column) => {
            column_lineage(column.table.as_ref(), &column.name, scope, context)
        }
        Expression::Alias(alias) => expression_lineage(&alias.this, scope, context),
        Expression::Paren(paren) => expression_lineage(&paren.this, scope, context),
        Expression::Annotated(annotated) => expression_lineage(&annotated.this, scope, context),
        Expression::Star(star) => star_lineage(star, scope, context),
        Expression::Parameter(parameter) => positional_lineage(parameter, scope),
        Expression::ConnectByRoot(root) => match &root.this {
            Expression::Var(var) => {
                column_lineage(None, &Identifier::new(var.this.clone()), scope, context)
            }
            other => expression_lineage(other, scope, context),
        },
        Expression::Cast(cast) | Expression::TryCast(cast) | Expression::SafeCast(cast) => {
            combined(
                vec![expression_lineage(&cast.this, scope, context)],
                TransformKind::Cast,
            )
        }
        Expression::Literal(_)
        | Expression::Null(_)
        | Expression::Boolean(_)
        | Expression::Identifier(_)
        | Expression::Var(_) => constant(),
        Expression::Exists(exists) => {
            let query: ResolvedQuery =
                resolved_query(&exists.this, scope.ctes, Some(scope), context);
            Lineage {
                rows: query_reads(query),
                ..unresolved()
            }
        }
        Expression::Subquery(subquery) => query_value(&subquery.this, scope, context),
        Expression::Select(_)
        | Expression::Union(_)
        | Expression::Intersect(_)
        | Expression::Except(_) => query_value(expression, scope, context),
        Expression::Lambda(lambda) => {
            let names: Vec<String> = lambda
                .parameters
                .iter()
                .map(|parameter| key(parameter, context))
                .collect();
            let inner = LineageScope {
                sources: Vec::new(),
                ctes: scope.ctes,
                outer: Some(scope),
                aliases: Vec::new(),
                lambda_names: names,
                merged: Vec::new(),
                windows: Vec::new(),
            };
            expression_lineage(&lambda.body, &inner, context)
        }
        Expression::WindowFunction(window) => {
            let mut parts: Vec<Lineage> = vec![children_lineage(
                expression,
                scope,
                context,
                TransformKind::Expression,
            )];
            if let Some(name) = &window.over.window_name {
                let name: String = key(name, context);
                let named: Option<&Lineage> = scope
                    .windows
                    .iter()
                    .find(|(window, _)| context.profile.same_identifier(window, &name))
                    .map(|(_, lineage)| lineage);
                parts.push(named.cloned().unwrap_or_else(unresolved));
            }
            let floor: TransformKind = parts[0].kind.max(TransformKind::Expression);
            combined(parts, floor)
        }
        _ if is_aggregate(expression) => {
            children_lineage(expression, scope, context, TransformKind::Aggregation)
        }
        _ => children_lineage(expression, scope, context, TransformKind::Expression),
    }
}

fn children_lineage(
    expression: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
    floor: TransformKind,
) -> Lineage {
    combined(
        expression
            .children()
            .into_iter()
            .map(|child| match (child, floor) {
                (Expression::Star(_), TransformKind::Aggregation) => constant(),
                _ => expression_lineage(child, scope, context),
            })
            .collect(),
        floor,
    )
}

/// `$n`: the n-th column of the level's one FROM item, untraced when that is not known.
fn positional_lineage(parameter: &Parameter, scope: &LineageScope<'_>) -> Lineage {
    let (Some(index), [only]) = (parameter.index, scope.sources.as_slice()) else {
        return unresolved();
    };
    let position: Option<usize> = match usize::try_from(index) {
        Ok(index) => index.checked_sub(POSITIONAL_COLUMN_BASE),
        Err(_) => None,
    };
    position.map_or_else(unresolved, |position| positional_column(only, position))
}

/// A query used as a value: what its outputs read, its filters as row dependencies.
fn query_value(
    query: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Lineage {
    let resolved = resolved_query(query, scope.ctes, Some(scope), context);
    let open: bool = resolved.open;
    let rows: BTreeSet<SourceColumn> = resolved.rows.clone();
    let value: Lineage = combined(
        resolved
            .columns
            .into_iter()
            .map(|column| column.lineage)
            .collect(),
        TransformKind::Expression,
    );
    let mut value: Lineage = traced_if(value, !open);
    value.rows.extend(rows);
    value
}

/// Every column a query reads, values and row dependencies together.
fn query_reads(query: ResolvedQuery) -> BTreeSet<SourceColumn> {
    let mut read: BTreeSet<SourceColumn> = query.rows;
    for column in &query.columns {
        read.extend(everything_read(&column.lineage));
    }
    read
}

fn is_aggregate(expression: &Expression) -> bool {
    match expression {
        Expression::AggregateFunction(_)
        | Expression::Count(_)
        | Expression::Sum(_)
        | Expression::Avg(_)
        | Expression::Min(_)
        | Expression::Max(_)
        | Expression::ArrayAgg(_)
        | Expression::CountIf(_) => true,
        Expression::Function(function) => AGGREGATE_FUNCTIONS
            .iter()
            .any(|name| function.name.eq_ignore_ascii_case(name)),
        _ => false,
    }
}

/// A star read as a value, as in `OBJECT_CONSTRUCT(*)`: everything its columns read.
fn star_lineage(star: &Star, scope: &LineageScope<'_>, context: LineageContext<'_>) -> Lineage {
    if star.table.is_none() && scope.sources.is_empty() {
        return constant();
    }
    let expanded = expanded_star(star, scope, context);
    let open: bool = expanded.open;
    let read = combined(
        expanded
            .columns
            .into_iter()
            .map(|column| column.lineage)
            .collect(),
        TransformKind::Expression,
    );
    traced_if(read, !open)
}
