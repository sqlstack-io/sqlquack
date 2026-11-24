//! Traversal: the scope of each query in a statement.

use std::collections::HashMap;

use crate::expressions::queries::models::Select;
use crate::expressions::tree::models::Expression;
use crate::traversal::models::{QueryScope, QueryScopes, ScopedRelation};
use crate::traversal::types::ExpressionWalk;

impl Expression {
    /// SQLQuack: each SELECT's relations, columns and the queries around it that it can see.
    pub fn query_scopes(&self) -> QueryScopes<'_> {
        let mut scopes: Vec<QueryScope<'_>> = Vec::new();
        let mut closed: HashMap<*const Expression, Option<usize>> = HashMap::new();
        let mut pending: Vec<(&Self, Option<usize>)> = vec![(self, None)];
        while let Some((node, within)) = pending.pop() {
            let mut current: Option<usize> = match closed.get(&std::ptr::from_ref(node)) {
                Some(&outer) => outer,
                None => within,
            };
            match node {
                Self::Select(select) => {
                    for query in closed_queries(select) {
                        closed.insert(std::ptr::from_ref(query), current);
                    }
                    scopes.push(QueryScope {
                        select,
                        outer: current,
                        relations: relations(select),
                        columns: Vec::new(),
                    });
                    current = Some(scopes.len() - 1);
                }
                Self::Column(column) => {
                    if let Some(scope) = current.and_then(|index| scopes.get_mut(index)) {
                        scope.columns.push(column);
                    }
                }
                _ => {}
            }
            pending.extend(
                node.children()
                    .into_iter()
                    .rev()
                    .map(|child| (child, current)),
            );
        }
        QueryScopes { scopes }
    }
}

/// Every FROM item and joined relation of `select`, with nested joins opened.
fn items(select: &Select) -> Vec<&Expression> {
    let mut pending: Vec<&Expression> = select
        .from
        .iter()
        .flat_map(|from| &from.expressions)
        .chain(select.joins.iter().map(|join| &join.this))
        .collect();
    pending.reverse();
    let mut found: Vec<&Expression> = Vec::new();
    while let Some(item) = pending.pop() {
        match item {
            Expression::Annotated(node) => pending.push(&node.this),
            Expression::JoinedTable(node) if node.alias.is_none() => {
                pending.extend(node.joins.iter().rev().map(|join| &join.this));
                pending.push(&node.left);
            }
            _ => found.push(item),
        }
    }
    found
}

/// The queries inside `select` that cannot see it: its CTE bodies and derived tables not LATERAL.
fn closed_queries(select: &Select) -> Vec<&Expression> {
    let mut found: Vec<&Expression> = select
        .with
        .iter()
        .flat_map(|with| &with.ctes)
        .map(|cte| &cte.this)
        .collect();
    let mut pending: Vec<&Expression> = items(select);
    while let Some(item) = pending.pop() {
        match item {
            Expression::Subquery(node) if !node.lateral => found.push(&node.this),
            Expression::Alias(node) => pending.push(&node.this),
            Expression::Pivot(node) => pending.push(&node.this),
            Expression::JoinedTable(node) => {
                pending.push(&node.left);
                pending.extend(node.joins.iter().map(|join| &join.this));
            }
            _ => {}
        }
    }
    found
}

/// The relation names `select` declares.
fn relations(select: &Select) -> Vec<ScopedRelation<'_>> {
    items(select).into_iter().filter_map(relation).collect()
}

/// The name of FROM item `item`.
fn relation(item: &Expression) -> Option<ScopedRelation<'_>> {
    let alias = match item {
        Expression::Table(node) => node.alias.as_ref(),
        Expression::Subquery(node) => node.alias.as_ref(),
        Expression::Alias(node) => Some(&node.alias),
        Expression::JoinedTable(node) => node.alias.as_ref(),
        Expression::Pivot(node) => node.alias.as_ref(),
        Expression::Unpivot(node) => node.alias.as_ref(),
        Expression::Values(node) => node.alias.as_ref(),
        _ => None,
    };
    let (name, quoted): (&str, bool) = match (alias, item) {
        (Some(alias), _) => (&alias.name, alias.quoted),
        (None, Expression::Table(node)) => (&node.name.name, node.name.quoted),
        (None, Expression::Lateral(node)) => (node.alias.as_deref()?, false),
        (None, Expression::Function(node)) if node.qualified_name.is_empty() => {
            (&node.name, node.quoted)
        }
        (None, Expression::Pivot(node)) => match &node.this {
            Expression::Table(table) => {
                let written = table.alias.as_ref().unwrap_or(&table.name);
                (&written.name, written.quoted)
            }
            _ => return None,
        },
        _ => return None,
    };
    Some(ScopedRelation {
        name,
        quoted,
        alias,
        item,
    })
}
