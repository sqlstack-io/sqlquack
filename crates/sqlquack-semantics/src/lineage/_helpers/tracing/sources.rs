//! The FROM items of a query level, in order, each with the columns it offers.

use std::rc::Rc;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, TableRef};
use sqlquack_syntax::expressions::operators::models::Lateral;
use sqlquack_syntax::expressions::queries::models::{
    Join, JoinKind, Pivot, Select, Unpivot, Values,
};

use crate::lineage::_helpers::tracing::expressions::expression_lineage;
use crate::lineage::_helpers::tracing::lineages::{combined, constant, direct, merged, unresolved};
use crate::lineage::_helpers::tracing::queries::{empty_scope, renamed, resolved_query};
use crate::lineage::_helpers::tracing::resolution::{key, plain_key};
use crate::lineage::constants::{
    DUCKDB_PROFILE, DUCKDB_VALUES_COLUMN, IDENTIFIER_FUNCTION, ONE_AGGREGATE, OUTER_LEFT_JOINS,
    OUTER_RIGHT_JOINS, SNOWFLAKE_PROFILE, SNOWFLAKE_VALUES_COLUMN, TABLE_FUNCTION_COLUMNS,
    TABLE_WRAPPER,
};
use crate::lineage::models::{
    FromItems, Lineage, LineageContext, LineageScope, NamedQuery, Nullability, ResolvedQuery,
    RowReads, SchemaColumn, ScopeSource, ScopedColumn, TransformKind,
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
        if join.is_some_and(|join| OUTER_RIGHT_JOINS.contains(&join.kind)) {
            for key in &mut from.merged {
                key.lineage.nullability = Nullability::Nullable;
            }
        }
        let scope = LineageScope {
            sources: from.sources.clone(),
            merged: from.merged.clone(),
            ..empty_scope(ctes, outer)
        };
        let mut added: Vec<ScopeSource> = item_sources(item, &scope, context);
        if let Some(join) = join {
            let (merged, rows) = joined(join, (&scope, &added), context);
            from.merged = merged;
            from.rows.absorb(&rows);
            if OUTER_RIGHT_JOINS.contains(&join.kind) {
                from.sources = from.sources.into_iter().map(outer_nullable).collect();
            }
            if OUTER_LEFT_JOINS.contains(&join.kind) || outer_as_of(join.kind, context) {
                added = added.into_iter().map(outer_nullable).collect();
            }
        }
        from.sources.extend(added);
    }
    from
}

/// Snowflake's ASOF JOIN keeps unmatched left rows, NULL-filling the right; DuckDB's is inner.
fn outer_as_of(kind: JoinKind, context: LineageContext<'_>) -> bool {
    kind == JoinKind::AsOf && context.profile.name() == SNOWFLAKE_PROFILE
}

/// `source` on the side of an outer join that may be NULL-filled: every column it offers may be NULL.
fn outer_nullable(mut source: ScopeSource) -> ScopeSource {
    source.query.outer_nullable = true;
    for column in &mut source.query.columns {
        column.lineage.nullability = Nullability::Nullable;
    }
    source
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
) -> (Vec<ScopedColumn>, RowReads) {
    let mut merged_keys: Vec<ScopedColumn> = left.merged.clone();
    let mut rows: RowReads = RowReads::default();
    let both = LineageScope {
        sources: left.sources.iter().chain(right).cloned().collect(),
        merged: left.merged.clone(),
        ..empty_scope(left.ctes, left.outer)
    };
    for condition in join.on.iter().chain(join.match_condition.iter()) {
        let read: Lineage = expression_lineage(condition, &both, context);
        rows.read(&read);
    }
    let names: Vec<String> = match natural(join.kind) {
        true => common_names(left, right, context),
        false => join.using.iter().map(|name| key(name, context)).collect(),
    };
    for name in names {
        let left_side: Lineage = side_column(&left.merged, &left.sources, &name, context);
        let right_side: Lineage = side_column(&[], right, &name, context);
        rows.read(&left_side);
        rows.read(&right_side);
        let lineage: Lineage = match join.kind {
            JoinKind::Right | JoinKind::NaturalRight => right_side,
            JoinKind::Full | JoinKind::Outer | JoinKind::NaturalFull => {
                let data_type: Option<Rc<str>> = left_side.data_type.clone().filter(|left| {
                    right_side
                        .data_type
                        .as_ref()
                        .is_some_and(|right| context.profile.same_type(left, right))
                });
                Lineage {
                    data_type,
                    ..combined(vec![left_side, right_side], TransformKind::Expression)
                }
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
pub(crate) fn item_sources(
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
                query: ResolvedQuery {
                    derived: true,
                    ..renamed(
                        resolved_query(&subquery.this, scope.ctes, outer, context),
                        &subquery.column_aliases,
                        context,
                    )
                },
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
            let reads: Option<Lineage> =
                (!arguments.is_empty()).then(|| combined(arguments, TransformKind::Expression));
            vec![ScopeSource {
                key: None,
                query: ResolvedQuery {
                    open: true,
                    open_relation: Some(plain_key(&function.name, context)),
                    rows: input_rows(reads.as_ref()),
                    open_reads: reads,
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
    let mut query: ResolvedQuery = renamed(query, &table.column_aliases, context);
    query.written_name = Some(written_name(table));
    ScopeSource {
        key: source_key,
        query,
    }
}

/// The relation's name as written, its parts joined by dots, in lower case.
fn written_name(table: &TableRef) -> String {
    [
        table.catalog.as_ref(),
        table.schema.as_ref(),
        Some(&table.name),
    ]
    .into_iter()
    .flatten()
    .map(|part: &Identifier| part.name.to_lowercase())
    .collect::<Vec<String>>()
    .join(".")
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

/// A stored relation: its schema columns, or open with the columns known so far.
fn base_relation(relation: &str, context: LineageContext<'_>) -> ResolvedQuery {
    match context.schema.columns(relation) {
        Some(columns) => ResolvedQuery {
            columns: stored_columns(relation, columns, context),
            stored_relation: Some(relation.to_owned()),
            ..ResolvedQuery::default()
        },
        None => ResolvedQuery {
            columns: stored_columns(
                relation,
                context.schema.open_columns(relation).unwrap_or_default(),
                context,
            ),
            open: true,
            open_relation: Some(relation.to_owned()),
            ..ResolvedQuery::default()
        },
    }
}

/// The columns a stored relation offers, typed as the schema lists them.
fn stored_columns(
    relation: &str,
    columns: &[SchemaColumn],
    context: LineageContext<'_>,
) -> Vec<ScopedColumn> {
    columns
        .iter()
        .map(|column| ScopedColumn {
            name: column.name.clone(),
            key: context
                .profile
                .resolved_identifier(&column.name, column.quoted),
            lineage: Lineage {
                data_type: column.data_type.as_deref().map(Rc::from),
                nullability: column.nullability,
                ..direct(relation, &column.name, column.quoted)
            },
        })
        .collect()
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
            rows: input_rows(Some(&input)),
            ..ResolvedQuery::default()
        },
    }
}

/// A table function's inputs as the columns deciding its rows: it returns rows per input.
fn input_rows(input: Option<&Lineage>) -> RowReads {
    let mut rows: RowReads = RowReads::default();
    if let Some(input) = input {
        rows.read(input);
    }
    rows
}

/// The row-deciding columns of every source.
fn absorbed(sources: &[ScopeSource]) -> RowReads {
    let mut rows: RowReads = RowReads::default();
    for source in sources {
        rows.absorb(&source.query.rows);
    }
    rows
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
        from_values: true,
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
        .map(|value| pivoted_name(value, pivot.expressions.len(), context))
        .collect();
    let passed: Vec<ScopedColumn> = passed_columns(&input.sources, &used, context);
    let dynamic = values.is_empty() || values.iter().any(String::is_empty);
    let mut rows: RowReads = RowReads::default();
    for field in &pivot.fields {
        let pivoted: &Expression = match field {
            Expression::In(within) => &within.this,
            other => other,
        };
        rows.read(&expression_lineage(pivoted, &input, context));
    }
    for source in &input.sources {
        rows.absorb(&source.query.rows);
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

/// A pivoted value's column: quoted text in Snowflake, bare in DuckDB (unnamed beside several aggregates).
fn pivoted_name(value: &Expression, aggregates: usize, context: LineageContext<'_>) -> String {
    let duckdb: bool = context.profile.name() == DUCKDB_PROFILE;
    if duckdb && aggregates != ONE_AGGREGATE {
        return String::new();
    }
    match value {
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::String(text) if duckdb => text.clone(),
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
        context.profile,
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
        rows: absorbed(&input.sources),
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
    merged(read, context.profile)
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
