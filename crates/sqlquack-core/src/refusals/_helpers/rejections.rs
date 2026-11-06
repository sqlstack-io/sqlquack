//! The constructs Snowflake parses but then rejects, recognised one node at a time.

use std::collections::HashSet;

use sqlquack_syntax::expressions::functions::models::Function;
use sqlquack_syntax::expressions::names::models::{Column, Identifier, Star};
use sqlquack_syntax::expressions::queries::models::Select;
use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::diagnostics::constants::{
    DUPLICATE_ALIAS, LATERAL_FUNCTION_JOIN_CONDITION, PIVOT_CONSUMED_COLUMN, TOO_MANY_ARGUMENTS,
};
use crate::refusals::constants::SNOWFLAKE_FUNCTION_ARITIES;

/// The registry name of the rejection a node causes, with the reason, if it causes one.
pub(crate) fn rejection(node: &Expression) -> Option<(&'static str, String)> {
    match node {
        Expression::Intersect(operation) if operation.all => Some(set_operation_all("INTERSECT")),
        Expression::Except(operation) if operation.all => {
            Some(set_operation_all("EXCEPT or MINUS"))
        }
        Expression::Select(select) if select.locks.iter().any(|lock| lock.wait.is_some()) => {
            Some((
                "lock_wait",
                "Snowflake does not support WAIT in FOR UPDATE".to_owned(),
            ))
        }
        Expression::Select(select)
            if select.qualify.as_ref().is_some_and(|qualify| {
                !select.expressions.iter().any(has_window) && !has_window(&qualify.this)
            }) =>
        {
            Some((
                "qualify_without_window",
                "QUALIFY is used without a window function in the SELECT list or QUALIFY"
                    .to_owned(),
            ))
        }
        Expression::Select(select) => duplicate_alias(select)
            .map(|alias| {
                (
                    DUPLICATE_ALIAS,
                    format!("the alias `{}` names two FROM items", alias.name),
                )
            })
            .or_else(|| {
                lateral_join_condition(select).map(|_| {
                    (
                        LATERAL_FUNCTION_JOIN_CONDITION,
                        "a lateral table function is joined with an ON condition".to_owned(),
                    )
                })
            })
            .or_else(|| {
                pivot_consumed_column(select).map(|column| {
                    (
                        PIVOT_CONSUMED_COLUMN,
                        format!(
                            "`{}` is aggregated or pivoted by the PIVOT, so its output has no such column",
                            column.name.name
                        ),
                    )
                })
            }),
        Expression::Function(function) => most_arguments(function)
            .filter(|most| function.args.len() > *most)
            .map(|most| {
                (
                    TOO_MANY_ARGUMENTS,
                    format!(
                        "the most arguments {} takes is {most}, and this call passes {}",
                        function.name.to_uppercase(),
                        function.args.len()
                    ),
                )
            }),
        Expression::Star(star) => filtered_out_name(star).map(|name| {
            (
                "star_filter_drops_named_column",
                format!("{name} does not match the star's ILIKE pattern, so REPLACE or RENAME names a column the pattern removed"),
            )
        }),
        _ => None,
    }
}

/// The first alias written again on a later FROM item or join, compared as Snowflake stores it.
pub(crate) fn duplicate_alias(select: &Select) -> Option<&Identifier> {
    let mut seen: HashSet<String> = HashSet::new();
    let items = select.from.iter().flat_map(|from| from.expressions.iter());
    items
        .chain(select.joins.iter().map(|join| &join.this))
        .filter_map(|item| match item {
            Expression::Table(table) => table.alias.as_ref(),
            Expression::Subquery(subquery) => subquery.alias.as_ref(),
            Expression::Alias(alias) => Some(&alias.alias),
            _ => None,
        })
        .find(|alias| !seen.insert(stored_name(alias)))
}

/// The ON condition of the first join of a LATERAL table function, which Snowflake joins only without one.
pub(crate) fn lateral_join_condition(select: &Select) -> Option<&Expression> {
    select
        .joins
        .iter()
        .filter(|join| matches!(join.this, Expression::Lateral(_)))
        .find_map(|join| join.on.as_ref())
}

/// The first column a lone PIVOT aggregates or pivots on that its query still reads by that name.
pub(crate) fn pivot_consumed_column(select: &Select) -> Option<&Column> {
    let [Expression::Pivot(pivot)] = select.from.as_ref()?.expressions.as_slice() else {
        return None;
    };
    let consumed: HashSet<String> = pivot
        .expressions
        .iter()
        .chain(&pivot.fields)
        .flat_map(own_columns)
        .map(|column| stored_name(&column.name))
        .collect();
    let aliases: HashSet<String> = select
        .expressions
        .iter()
        .filter_map(|projection| match projection {
            Expression::Alias(alias) => Some(stored_name(&alias.alias)),
            _ => None,
        })
        .collect();
    let pivot_name = pivot.alias.as_ref().map(stored_name);
    let clauses = select
        .where_clause
        .iter()
        .map(|clause| &clause.this)
        .chain(select.group_by.iter().flat_map(|group| &group.expressions))
        .chain(select.having.iter().map(|clause| &clause.this))
        .chain(select.qualify.iter().map(|clause| &clause.this))
        .chain(
            select
                .order_by
                .iter()
                .flat_map(|order| &order.expressions)
                .map(|ordered| &ordered.this),
        );
    select
        .expressions
        .iter()
        .chain(clauses)
        .flat_map(own_columns)
        .filter(|_| select.joins.is_empty() && !pivot.unpivot)
        .find(|column| {
            let name = stored_name(&column.name);
            consumed.contains(&name)
                && !aliases.contains(&name)
                && column
                    .table
                    .as_ref()
                    .is_none_or(|table| pivot_name.as_ref() == Some(&stored_name(table)))
        })
}

/// The columns an expression reads outside any nested query.
fn own_columns(expression: &Expression) -> Vec<&Column> {
    match expression {
        Expression::Column(column) => vec![column],
        Expression::Select(_)
        | Expression::Subquery(_)
        | Expression::Union(_)
        | Expression::Intersect(_)
        | Expression::Except(_) => Vec::new(),
        _ => expression
            .children()
            .into_iter()
            .flat_map(own_columns)
            .collect(),
    }
}

/// An identifier as Snowflake stores it: as written when quoted, else in upper case.
fn stored_name(identifier: &Identifier) -> String {
    match identifier.quoted {
        true => identifier.name.clone(),
        false => identifier.name.to_uppercase(),
    }
}

/// The most arguments a call without named arguments may pass, when the catalogue bounds it; a malformed limit bounds nothing.
fn most_arguments(function: &Function) -> Option<usize> {
    let named = function
        .args
        .iter()
        .any(|argument| matches!(argument, Expression::NamedArgument(_)));
    let name = function.name.to_uppercase();
    SNOWFLAKE_FUNCTION_ARITIES
        .lines()
        .filter(|_| !named)
        .filter_map(|line| line.split_once(' '))
        .find(|(listed, _)| *listed == name)
        .map(|(_, most)| most.parse().unwrap_or(usize::MAX))
}

/// A REPLACE or RENAME name the ILIKE pattern removes ignoring case; backslashes and non-ASCII skip.
fn filtered_out_name(star: &Star) -> Option<&str> {
    let pattern = star
        .ilike
        .as_deref()
        .filter(|pattern| !pattern.contains('\\'))?;
    let replaced = star.replace.iter().flatten().map(|alias| &alias.alias);
    let renamed = star.rename.iter().flatten().map(|(old, _)| old);
    replaced
        .chain(renamed)
        .map(|identifier| identifier.name.as_str())
        .find(|name| name.is_ascii() && pattern.is_ascii() && !ilike_matches(pattern, name))
}

/// Snowflake ILIKE over ASCII text: `%` is any run, `_` one character, and case is ignored.
fn ilike_matches(pattern: &str, text: &str) -> bool {
    let (pattern, text) = (pattern.as_bytes(), text.as_bytes());
    let (mut p, mut t, mut resume) = (0, 0, None);
    while t < text.len() {
        match pattern.get(p) {
            Some(b'%') => {
                resume = Some((p, t));
                p += 1;
            }
            Some(&wanted) if wanted == b'_' || wanted.eq_ignore_ascii_case(&text[t]) => {
                p += 1;
                t += 1;
            }
            _ => match resume {
                Some((star, from)) => {
                    p = star + 1;
                    t = from + 1;
                    resume = Some((star, from + 1));
                }
                None => return false,
            },
        }
    }
    pattern[p..].iter().all(|&byte| byte == b'%')
}

/// The rejection of a set operation with ALL, which Snowflake supports only for UNION.
fn set_operation_all(operator: &str) -> (&'static str, String) {
    (
        "set_operation_all",
        format!("Snowflake does not support {operator} with ALL"),
    )
}

/// Whether the expression calls a window function outside any nested query.
fn has_window(expression: &Expression) -> bool {
    match expression {
        Expression::WindowFunction(_) => true,
        Expression::Select(_)
        | Expression::Subquery(_)
        | Expression::Union(_)
        | Expression::Intersect(_)
        | Expression::Except(_) => false,
        _ => expression.children().into_iter().any(has_window),
    }
}
