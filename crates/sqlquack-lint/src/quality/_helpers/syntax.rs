//! Shared syntax helpers for SQL quality Rules.

use sqlquack_syntax::expressions::names::models::Column;
use sqlquack_syntax::expressions::names::models::Star;
use sqlquack_syntax::expressions::queries::models::{Select, Subquery};
use sqlquack_syntax::tokens::models::{Token, TokenType};
use sqlquack_syntax::{Expression, ExpressionWalk};

use crate::quality::constants::MIN_REMOVABLE_OUTPUTS;
use rustc_hash::FxHashMap;

/// The query within `expression`'s parentheses and unaliased subquery nodes.
pub(super) fn query_within(expression: &Expression) -> &Expression {
    match expression {
        Expression::Paren(paren) => query_within(&paren.this),
        Expression::Subquery(subquery) if around(subquery) => query_within(&subquery.this),
        other => other,
    }
}

/// The clause expressions of the unaliased subquery nodes around `expression`'s query.
pub(super) fn clauses_around(expression: &Expression) -> Vec<&Expression> {
    let mut clauses: Vec<&Expression> = Vec::new();
    let mut node: &Expression = expression;
    loop {
        node = match node {
            Expression::Paren(paren) => &paren.this,
            Expression::Subquery(subquery) if around(subquery) => {
                let ordered = subquery
                    .order_by
                    .iter()
                    .flat_map(|order| &order.expressions)
                    .chain(subquery.sort_by.iter().flat_map(|sort| &sort.expressions))
                    .chain(
                        subquery
                            .cluster_by
                            .iter()
                            .flat_map(|cluster| &cluster.expressions),
                    );
                clauses.extend(ordered.map(|ordered| &ordered.this));
                clauses.extend(
                    subquery
                        .distribute_by
                        .iter()
                        .flat_map(|distribute| &distribute.expressions),
                );
                clauses.extend(subquery.limit.iter().map(|limit| &limit.this));
                clauses.extend(subquery.offset.iter().map(|offset| &offset.this));
                &subquery.this
            }
            _ => return clauses,
        };
    }
}

/// Whether `subquery` only wraps a query: unaliased, without a sample or watermark.
fn around(subquery: &Subquery) -> bool {
    subquery.alias.is_none() && subquery.table_sample.is_none() && subquery.watermark.is_none()
}

/// One projection of a select list.
pub(super) enum Projection<'a> {
    Named {
        name: String,
        expression: &'a Expression,
    },
    Star(&'a Star),
    Unnamed,
}

pub(super) fn lower(name: &str) -> String {
    name.to_ascii_lowercase()
}

pub(super) fn projections(select: &Select) -> Vec<Projection<'_>> {
    select
        .expressions
        .iter()
        .map(|expression| match expression {
            Expression::Alias(alias) => Projection::Named {
                name: lower(&alias.alias.name),
                expression: &alias.this,
            },
            Expression::Column(column) => Projection::Named {
                name: lower(&column.name.name),
                expression,
            },
            Expression::Star(star) => Projection::Star(star),
            _ => Projection::Unnamed,
        })
        .collect()
}

pub(super) fn columns(expression: &Expression) -> Vec<&Column> {
    expression
        .dfs()
        .filter_map(|node| match node {
            Expression::Column(column) => Some(column.as_ref()),
            _ => None,
        })
        .collect()
}

pub(super) fn qualifier(column: &Column) -> Option<String> {
    column.table.as_ref().map(|table| lower(&table.name))
}

/// Relations read directly by one select: `(alias, relation name, expression)`.
pub(super) fn sources(select: &Select) -> Vec<(String, Option<String>, &Expression)> {
    let mut found: Vec<(String, Option<String>, &Expression)> = Vec::new();
    let from = select.from.iter().flat_map(|from| from.expressions.iter());
    for expression in from.chain(select.joins.iter().map(|join| &join.this)) {
        match expression {
            Expression::Table(table) => {
                let name: String = lower(&table.name.name);
                let alias: String = table
                    .alias
                    .as_ref()
                    .map_or_else(|| name.clone(), |alias| lower(&alias.name));
                found.push((alias, Some(name), expression));
            }
            Expression::Subquery(subquery) => {
                let alias: String = subquery
                    .alias
                    .as_ref()
                    .map_or_else(String::new, |alias| lower(&alias.name));
                found.push((alias, None, expression));
            }
            _ => found.push((String::new(), None, expression)),
        }
    }
    found
}

/// Token index of the SELECT opening each `name AS ( [( ...] SELECT` CTE body, by lower-cased name.
pub(super) fn cte_select_tokens(tokens: &[Token]) -> FxHashMap<String, usize> {
    let significant: Vec<usize> = (0..tokens.len())
        .filter(|&index| !is_trivia(&tokens[index]))
        .collect();
    let mut found: FxHashMap<String, usize> = FxHashMap::default();
    for (position, window) in significant.windows(3).enumerate() {
        let [name_index, as_index, paren_index] = window else {
            continue;
        };
        if tokens[*as_index].token_type != TokenType::As
            || tokens[*paren_index].token_type != TokenType::LParen
        {
            continue;
        }
        let select_index: Option<&usize> = significant[position + 3..]
            .iter()
            .find(|&&index| tokens[index].token_type != TokenType::LParen);
        if let Some(&select_index) = select_index
            && tokens[select_index].token_type == TokenType::Select
        {
            found
                .entry(lower(&tokens[*name_index].text))
                .or_insert(select_index);
        }
    }
    found
}

/// The comma-inclusive deletion span that removes projection `index` from `spans`.
pub(super) fn projection_removal(spans: &[(usize, usize)], index: usize) -> Option<(usize, usize)> {
    if spans.len() < MIN_REMOVABLE_OUTPUTS {
        return None;
    }
    let (start, end) = spans[index];
    if index + 1 < spans.len() {
        Some((start, spans[index + 1].0))
    } else {
        Some((spans[index - 1].1, end))
    }
}

pub(super) fn is_trivia(token: &Token) -> bool {
    matches!(
        token.token_type,
        TokenType::Space | TokenType::Break | TokenType::LineComment | TokenType::BlockComment
    )
}
