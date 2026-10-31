//! The FROM items of a query level, in order, each with the columns it offers.

use std::collections::BTreeSet;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::operators::models::Lateral;
use sqlquack_syntax::expressions::queries::models::{
    Join, JoinKind, Pivot, Select, Unpivot, Values,
};

use crate::lineage::_helpers::expressions::expression_lineage;
use crate::lineage::_helpers::lineages::{
    combined, constant, direct, everything_read, merged, unresolved,
};
use crate::lineage::_helpers::queries::{empty_scope, renamed, resolved_query};
use crate::lineage::_helpers::resolution::{key, plain_key};
use crate::lineage::constants::{
    DUCKDB_PROFILE, DUCKDB_VALUES_COLUMN, IDENTIFIER_FUNCTION, SNOWFLAKE_VALUES_COLUMN,
    TABLE_FUNCTION_COLUMNS, TABLE_WRAPPER,
};
use crate::lineage::models::{
    FromItems, Lineage, LineageContext, LineageScope, NamedQuery, ResolvedQuery, ScopeSource,
    ScopedColumn, SourceColumn, TransformKind,
};

/// The FROM items of `select` and its joins; a later item may read earlier ones (LATERAL).
pub(crate) fn from_sources(
    select: &Select,
    ctes: &[NamedQuery],
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> FromItems {
    let mut items: Vec<(&Expression, Option<&Join>)> = Vec::new();
    for item in select.from.iter().flat_map(|from| from.expressions.iter()) {
        items.extend(flattened(item, None));
    }
    for join in &select.joins {
        items.extend(flattened(&join.this, Some(join)));
    }
    let mut from = FromItems::default();
    for (item, join) in items {
        let scope = LineageScope {
            sources: from.sources.clone(),
            merged: from.merged.clone(),
            ..empty_scope(ctes, outer)
        };
        let added: Vec<ScopeSource> = item_sources(item, &scope, context);
        if let Some(join) = join {
            let (merged, rows) = joined(join, (&scope, &added), context);
            from.merged = merged;
            from.rows.extend(rows);
        }
        from.sources.extend(added);
    }
    from
}

/// A FROM item, with a parenthesised join's tables and joins listed in order.
fn flattened<'a>(
    item: &'a Expression,
    join: Option<&'a Join>,
) -> Vec<(&'a Expression, Option<&'a Join>)> {
    match item {
        Expression::JoinedTable(joined) if joined.alias.is_none() => {
            let mut items = flattened(&joined.left, join);
            for inner in &joined.joins {
                items.extend(flattened(&inner.this, Some(inner)));
            }
            items
        }
        Expression::Paren(paren) if matches!(paren.this, Expression::JoinedTable(_)) => {
            flattened(&paren.this, join)
        }
        _ => vec![(item, join)],
    }
}

/// The merged keys after a join, and the columns its conditions and keys decide rows by.
fn joined(
    join: &Join,
    (left, right): (&LineageScope<'_>, &[ScopeSource]),
    context: LineageContext<'_>,
) -> (Vec<ScopedColumn>, BTreeSet<SourceColumn>) {
    let mut merged_keys: Vec<ScopedColumn> = left.merged.clone();
    let mut rows: BTreeSet<SourceColumn> = BTreeSet::new();
    let both = LineageScope {
        sources: left.sources.iter().chain(right).cloned().collect(),
        merged: left.merged.clone(),
        ..empty_scope(left.ctes, left.outer)
    };
    for condition in join.on.iter().chain(join.match_condition.iter()) {
        let read: Lineage = expression_lineage(condition, &both, context);
        rows.extend(everything_read(&read));
    }
    let names: Vec<String> = match natural(join.kind) {
        true => common_names(left, right, context),
        false => join.using.iter().map(|name| key(name, context)).collect(),
    };
    for name in names {
        let left_side: Lineage = side_column(&left.merged, &left.sources, &name, context);
        let right_side: Lineage = side_column(&[], right, &name, context);
        rows.extend(everything_read(&left_side));
        rows.extend(everything_read(&right_side));
        let lineage: Lineage = match join.kind {
            JoinKind::Right | JoinKind::NaturalRight => right_side,
            JoinKind::Full | JoinKind::Outer | JoinKind::NaturalFull => {
                combined(vec![left_side, right_side], TransformKind::Expression)
            }
            _ => left_side,
        };
        let column = ScopedColumn {
            name: name.clone(),
            key: name,
            lineage,
        };
        match merged_keys
            .iter_mut()
            .find(|merged| context.profile.same_identifier(&merged.key, &column.key))
        {
            Some(merged) => *merged = column,
            None => merged_keys.push(column),
        }
    }
    (merged_keys, rows)
}

fn natural(kind: JoinKind) -> bool {
    matches!(
        kind,
        JoinKind::Natural | JoinKind::NaturalLeft | JoinKind::NaturalRight | JoinKind::NaturalFull
    )
}

/// The column names both sides of a NATURAL join know.
fn common_names(
    left: &LineageScope<'_>,
    right: &[ScopeSource],
    context: LineageContext<'_>,
) -> Vec<String> {
    let left_names: Vec<String> = left
        .merged
        .iter()
        .chain(
            left.sources
                .iter()
                .flat_map(|source| source.query.columns.iter()),
        )
        .map(|column| column.key.clone())
        .collect();
    let mut names: Vec<String> = Vec::new();
    for column in right.iter().flat_map(|source| source.query.columns.iter()) {
        if listed(&left_names, &column.key, context) && !listed(&names, &column.key, context) {
            names.push(column.key.clone());
        }
    }
    names
}

/// A join key's lineage on one side: a key merged earlier, else the first column so named.
fn side_column(
    merged: &[ScopedColumn],
    sources: &[ScopeSource],
    name: &str,
    context: LineageContext<'_>,
) -> Lineage {
    merged
        .iter()
        .chain(
            sources
                .iter()
                .flat_map(|source| source.query.columns.iter()),
        )
        .find(|column| context.profile.same_identifier(&column.key, name))
        .map(|column| column.lineage.clone())
        .unwrap_or_else(unresolved)
}

/// The sources one FROM item adds, reading earlier items through `scope`.
fn item_sources(
    item: &Expression,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Vec<ScopeSource> {
    match item {
        Expression::Table(table) => vec![table_source(table, scope, context)],
        Expression::Subquery(subquery) => {
            let outer = if subquery.lateral {
                Some(scope)
            } else {
                scope.outer
            };
            vec![ScopeSource {
                key: subquery.alias.as_ref().map(|alias| key(alias, context)),
                query: renamed(
                    resolved_query(&subquery.this, scope.ctes, outer, context),
                    &subquery.column_aliases,
                    context,
                ),
            }]
        }
        Expression::Alias(alias) => {
            let inner = item_sources(&alias.this, scope, context);
            inner
                .into_iter()
                .map(|source| ScopeSource {
                    key: Some(key(&alias.alias, context)),
                    query: renamed(source.query, &alias.column_aliases, context),
                })
                .collect()
        }
        Expression::Lateral(lateral) => {
            let alias: Option<String> = lateral_key(lateral, context);
            item_sources(&lateral.this, scope, context)
                .into_iter()
                .map(|source| ScopeSource {
                    key: alias.clone().or(source.key),
                    query: source.query,
                })
                .collect()
        }
        Expression::Function(function) if built_in_columns(&function.name).is_some() => {
            let columns = built_in_columns(&function.name).unwrap_or_default();
            vec![table_function_source(
                columns,
                &function.args,
                scope,
                context,
            )]
        }
        Expression::Function(function) if function.name.eq_ignore_ascii_case(TABLE_WRAPPER) => {
            function
                .args
                .iter()
                .flat_map(|argument| item_sources(argument, scope, context))
                .collect()
        }
        Expression::Function(function)
            if function.name.eq_ignore_ascii_case(IDENTIFIER_FUNCTION) =>
        {
            vec![opaque_source()]
        }
        Expression::Function(function) => {
            let arguments: Vec<Lineage> = function
                .args
                .iter()
                .map(|argument| expression_lineage(argument, scope, context))
                .collect();
            vec![ScopeSource {
                key: None,
                query: ResolvedQuery {
                    open: true,
                    open_relation: Some(plain_key(&function.name, context)),
                    open_reads: (!arguments.is_empty())
                        .then(|| combined(arguments, TransformKind::Expression)),
                    ..ResolvedQuery::default()
                },
            }]
        }
        Expression::Values(values) => vec![values_source(values, scope, context)],
        Expression::JoinedTable(joined) => {
            let mut sources = item_sources(&joined.left, scope, context);
            for join in &joined.joins {
                sources.extend(item_sources(&join.this, scope, context));
            }
            let alias: Option<String> = joined.alias.as_ref().map(|alias| key(alias, context));
            sources
                .into_iter()
                .map(|source| ScopeSource {
                    key: alias.clone().or(source.key),
                    query: source.query,
                })
                .collect()
        }
        Expression::Paren(paren) => item_sources(&paren.this, scope, context),
        Expression::Annotated(annotated) => item_sources(&annotated.this, scope, context),
        Expression::Pivot(pivot) if !pivot.unpivot => pivot_sources(pivot, scope, context),
        Expression::Unpivot(unpivot) => vec![unpivot_source(unpivot, scope, context)],
        _ => vec![opaque_source()],
    }
}

/// A FROM item lineage does not trace: its columns are unknown and untraced.
fn opaque_source() -> ScopeSource {
    ScopeSource {
        key: None,
        query: ResolvedQuery {
            open: true,
            ..ResolvedQuery::default()
        },
    }
}

fn table_source(
    table: &TableRef,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> ScopeSource {
    let name = key(&table.name, context);
    let source_key = Some(
        table
            .alias
            .as_ref()
            .map_or_else(|| name.clone(), |alias| key(alias, context)),
    );
    let unqualified = table.schema.is_none() && table.catalog.is_none();
    let cte = scope
        .ctes
        .iter()
        .rev()
        .find(|cte| unqualified && context.profile.same_identifier(&cte.key, &name));
    let query = match cte {
        Some(cte) => cte.query.clone(),
        None => base_relation(&relation_name(table, context), context),
    };
    ScopeSource {
        key: source_key,
        query: renamed(query, &table.column_aliases, context),
    }
}

/// The relation's name, its parts resolved and joined by dots.
fn relation_name(table: &TableRef, context: LineageContext<'_>) -> String {
    [
        table.catalog.as_ref(),
        table.schema.as_ref(),
        Some(&table.name),
    ]
    .into_iter()
    .flatten()
    .map(|part: &Identifier| key(part, context))
    .collect::<Vec<String>>()
    .join(".")
}

/// A stored relation: its schema columns, or open when the schema does not know it.
fn base_relation(relation: &str, context: LineageContext<'_>) -> ResolvedQuery {
    match context.schema.columns(relation) {
        Some(columns) => ResolvedQuery {
            columns: columns
                .iter()
                .map(|column| ScopedColumn {
                    name: column.name.clone(),
                    key: context
                        .profile
                        .resolved_identifier(&column.name, column.quoted),
                    lineage: direct(relation, &column.name, column.quoted),
                })
                .collect(),
            ..ResolvedQuery::default()
        },
        None => ResolvedQuery {
            open: true,
            open_relation: Some(relation.to_owned()),
            ..ResolvedQuery::default()
        },
    }
}

/// The columns of a built-in table function, when `name` is one.
fn built_in_columns(name: &str) -> Option<&'static [&'static str]> {
    TABLE_FUNCTION_COLUMNS
        .iter()
        .find(|(function, _)| name.eq_ignore_ascii_case(function))
        .map(|(_, columns)| *columns)
}

/// A built-in table function: each of its columns derives from everything its arguments read.
fn table_function_source(
    columns: &[&str],
    arguments: &[Expression],
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> ScopeSource {
    let input = combined(
        arguments
            .iter()
            .map(|argument| expression_lineage(argument, scope, context))
            .collect(),
        TransformKind::Expression,
    );
    ScopeSource {
        key: None,
        query: ResolvedQuery {
            columns: columns
                .iter()
                .map(|column| ScopedColumn {
                    name: (*column).to_owned(),
                    key: plain_key(column, context),
                    lineage: input.clone(),
                })
                .collect(),
            ..ResolvedQuery::default()
        },
    }
}

/// A VALUES list as a FROM item.
fn values_source(
    values: &Values,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> ScopeSource {
    ScopeSource {
        key: values.alias.as_ref().map(|alias| key(alias, context)),
        query: values_query(values, scope, context),
    }
}

/// A VALUES list: its n-th column reads the n-th value of every row.
pub(crate) fn values_query(
    values: &Values,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    let width = values
        .expressions
        .iter()
        .map(|row| row.expressions.len())
        .max()
        .unwrap_or(0);
    let generated = |position: usize| match context.profile.name() {
        DUCKDB_PROFILE => format!("{DUCKDB_VALUES_COLUMN}{position}"),
        _ => format!("{SNOWFLAKE_VALUES_COLUMN}{}", position + 1),
    };
    let columns: Vec<ScopedColumn> = (0..width)
        .map(|position| {
            let name = generated(position);
            ScopedColumn {
                key: plain_key(&name, context),
                name,
                lineage: values_column(values, position, scope, context),
            }
        })
        .collect();
    let query = ResolvedQuery {
        columns,
        ..ResolvedQuery::default()
    };
    renamed(query, &values.column_aliases, context)
}

/// PIVOT: the input's other columns pass through; each pivoted column aggregates its values.
fn pivot_sources(
    pivot: &Pivot,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Vec<ScopeSource> {
    let input = LineageScope {
        sources: item_sources(&pivot.this, scope, context),
        ..empty_scope(scope.ctes, scope.outer)
    };
    let used: Vec<String> = pivot
        .expressions
        .iter()
        .chain(pivot.fields.iter())
        .flat_map(|expression| expression.dfs())
        .filter_map(|node| match node {
            Expression::Column(column) => Some(key(&column.name, context)),
            _ => None,
        })
        .collect();
    let aggregated = combined(
        pivot
            .expressions
            .iter()
            .map(|aggregate| expression_lineage(aggregate, &input, context))
            .collect(),
        TransformKind::Aggregation,
    );
    let values: Vec<String> = pivot
        .fields
        .iter()
        .flat_map(|field| match field {
            Expression::In(within) => within.expressions.iter().collect::<Vec<&Expression>>(),
            _ => Vec::new(),
        })
        .map(pivoted_name)
        .collect();
    let passed: Vec<ScopedColumn> = passed_columns(&input.sources, &used, context);
    let dynamic = values.is_empty() || values.iter().any(String::is_empty);
    let mut rows: BTreeSet<SourceColumn> = BTreeSet::new();
    for field in &pivot.fields {
        let pivoted: &Expression = match field {
            Expression::In(within) => &within.this,
            other => other,
        };
        rows.extend(everything_read(&expression_lineage(
            pivoted, &input, context,
        )));
    }
    for source in &input.sources {
        rows.extend(source.query.rows.iter().cloned());
    }
    let pivoted = values.into_iter().map(|name| ScopedColumn {
        key: context.profile.resolved_identifier(&name, true),
        name,
        lineage: aggregated.clone(),
    });
    let query = ResolvedQuery {
        columns: match dynamic {
            true => passed,
            false => passed.into_iter().chain(pivoted).collect(),
        },
        open: dynamic || input.sources.iter().any(|source| source.query.open),
        rows,
        ..ResolvedQuery::default()
    };
    vec![ScopeSource {
        key: pivot.alias.as_ref().map(|alias| key(alias, context)),
        query: renamed(query, &pivot.alias_columns, context),
    }]
}

/// The column a pivoted value names: a string literal's text, quoted as Snowflake names it.
fn pivoted_name(value: &Expression) -> String {
    match value {
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::String(text) => format!("'{text}'"),
            Literal::Number(text) => text.clone(),
            _ => String::new(),
        },
        Expression::Alias(alias) => alias.alias.name.clone(),
        _ => String::new(),
    }
}

/// UNPIVOT: the name column is constant and the value column reads every unpivoted column.
fn unpivot_source(
    unpivot: &Unpivot,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> ScopeSource {
    let input = LineageScope {
        sources: item_sources(&unpivot.this, scope, context),
        ..empty_scope(scope.ctes, scope.outer)
    };
    let unpivoted: Vec<String> = unpivot
        .columns
        .iter()
        .flat_map(|column| column.dfs())
        .filter_map(|node| match node {
            Expression::Column(column) => Some(key(&column.name, context)),
            _ => None,
        })
        .collect();
    let value = merged(
        unpivot
            .columns
            .iter()
            .map(|column| expression_lineage(column, &input, context))
            .collect(),
    );
    let passed: Vec<ScopedColumn> = passed_columns(&input.sources, &unpivoted, context);
    let generated = [
        (&unpivot.name_column, constant()),
        (&unpivot.value_column, value),
    ]
    .into_iter()
    .map(|(name, lineage)| ScopedColumn {
        name: name.name.clone(),
        key: key(name, context),
        lineage,
    });
    let query = ResolvedQuery {
        columns: passed.into_iter().chain(generated).collect(),
        open: input.sources.iter().any(|source| source.query.open),
        rows: input
            .sources
            .iter()
            .flat_map(|source| source.query.rows.iter().cloned())
            .collect(),
        ..ResolvedQuery::default()
    };
    ScopeSource {
        key: unpivot.alias.as_ref().map(|alias| key(alias, context)),
        query: renamed(query, &unpivot.alias_columns, context),
    }
}

/// The key a LATERAL item's alias gives it, when it has one.
fn lateral_key(lateral: &Lateral, context: LineageContext<'_>) -> Option<String> {
    let alias: &String = lateral.alias.as_ref()?;
    Some(
        context
            .profile
            .resolved_identifier(alias, lateral.alias_quoted),
    )
}

/// The n-th column of a VALUES list: it reads the n-th value of every row.
fn values_column(
    values: &Values,
    position: usize,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Lineage {
    let mut read: Vec<Lineage> = Vec::with_capacity(values.expressions.len());
    for row in &values.expressions {
        if let Some(value) = row.expressions.get(position) {
            read.push(expression_lineage(value, scope, context));
        }
    }
    merged(read)
}

/// The columns of `sources` a PIVOT or UNPIVOT passes through: those it does not consume.
fn passed_columns(
    sources: &[ScopeSource],
    consumed: &[String],
    context: LineageContext<'_>,
) -> Vec<ScopedColumn> {
    let mut passed: Vec<ScopedColumn> = Vec::new();
    for column in sources
        .iter()
        .flat_map(|source| source.query.columns.iter())
    {
        if !listed(consumed, &column.key, context) {
            passed.push(column.clone());
        }
    }
    passed
}

/// Whether `key` is one of `names`.
pub(crate) fn listed(names: &[String], key: &str, context: LineageContext<'_>) -> bool {
    names
        .iter()
        .any(|name| context.profile.same_identifier(name, key))
}
