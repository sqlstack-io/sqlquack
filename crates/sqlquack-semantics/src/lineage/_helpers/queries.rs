//! The outputs of a query, set operation or CTE, with their lineage.

use std::collections::BTreeSet;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Identifier, Star};
use sqlquack_syntax::expressions::queries::models::{Cte, Select, With};

use crate::lineage::_helpers::expressions::expression_lineage;
use crate::lineage::_helpers::lineages::{combined, everything_read, merged, traced_if};
use crate::lineage::_helpers::resolution::{key, plain_key};
use crate::lineage::_helpers::sources::{from_sources, listed, values_query};
use crate::lineage::constants::{DUCKDB_PROFILE, RECURSION_LIMIT, UNNAMED_CHARACTERS};
use crate::lineage::models::{
    FromItems, Lineage, LineageContext, LineageScope, NamedQuery, ResolvedQuery, ScopeSource,
    ScopedColumn, SourceColumn, TransformKind,
};

/// The outputs of `query`, with `ctes` in reach and `outer` the enclosing level.
pub(crate) fn resolved_query(
    query: &Expression,
    ctes: &[NamedQuery],
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    match query {
        Expression::Select(select) => select_query(select, ctes, outer, context),
        Expression::Union(union) => {
            let ctes = with_ctes(union.with.as_ref(), ctes, outer, context);
            let sides: SetSides = match union.by_name {
                true => SetSides::ByName,
                false => SetSides::Both,
            };
            set_operation((&union.left, &union.right), (&ctes, outer), context, sides)
        }
        Expression::Intersect(intersect) => {
            let ctes = with_ctes(intersect.with.as_ref(), ctes, outer, context);
            let sides: (&Expression, &Expression) = (&intersect.left, &intersect.right);
            set_operation(sides, (&ctes, outer), context, SetSides::Intersect)
        }
        Expression::Except(except) => {
            let ctes = with_ctes(except.with.as_ref(), ctes, outer, context);
            let sides: (&Expression, &Expression) = (&except.left, &except.right);
            set_operation(sides, (&ctes, outer), context, SetSides::Left)
        }
        Expression::Subquery(subquery) => renamed(
            resolved_query(&subquery.this, ctes, outer, context),
            &subquery.column_aliases,
            context,
        ),
        Expression::Paren(paren) => resolved_query(&paren.this, ctes, outer, context),
        Expression::Annotated(annotated) => resolved_query(&annotated.this, ctes, outer, context),
        Expression::Values(values) => {
            let scope = empty_scope(ctes, outer);
            values_query(values, &scope, context)
        }
        _ => ResolvedQuery {
            open: true,
            ..ResolvedQuery::default()
        },
    }
}

/// A level that reads no FROM item, with `ctes` in reach and `outer` the enclosing level.
pub(crate) fn empty_scope<'a>(
    ctes: &'a [NamedQuery],
    outer: Option<&'a LineageScope<'a>>,
) -> LineageScope<'a> {
    LineageScope {
        sources: Vec::new(),
        ctes,
        outer,
        aliases: Vec::new(),
        lambda_names: Vec::new(),
        merged: Vec::new(),
        windows: Vec::new(),
    }
}

/// `query`'s outputs renamed by a column alias list, when it has one.
pub(crate) fn renamed(
    query: ResolvedQuery,
    aliases: &[Identifier],
    context: LineageContext<'_>,
) -> ResolvedQuery {
    if aliases.is_empty() {
        return query;
    }
    let columns: Vec<ScopedColumn> = query
        .columns
        .into_iter()
        .zip(aliases)
        .map(|(column, alias)| ScopedColumn {
            name: alias.name.clone(),
            key: key(alias, context),
            lineage: column.lineage,
        })
        .collect();
    ResolvedQuery { columns, ..query }
}

/// Which sides of a set operation its rows and values come from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SetSides {
    Both,
    ByName,
    Intersect,
    Left,
}

fn set_operation(
    (left, right): (&Expression, &Expression),
    (ctes, outer): (&[NamedQuery], Option<&LineageScope<'_>>),
    context: LineageContext<'_>,
    sides: SetSides,
) -> ResolvedQuery {
    let left = resolved_query(left, ctes, outer, context);
    let right = resolved_query(right, ctes, outer, context);
    let mut rows: BTreeSet<SourceColumn> = left.rows.union(&right.rows).cloned().collect();
    if matches!(sides, SetSides::Left | SetSides::Intersect) {
        for column in &right.columns {
            rows.extend(everything_read(&column.lineage));
        }
    }
    let query: ResolvedQuery = match sides {
        SetSides::Left => left,
        SetSides::ByName => by_name(left, right, context),
        SetSides::Both | SetSides::Intersect => positional(left, right),
    };
    ResolvedQuery { rows, ..query }
}

/// Two queries' columns merged by position.
fn positional(left: ResolvedQuery, right: ResolvedQuery) -> ResolvedQuery {
    let open: bool = left.open || right.open;
    let mut right_columns = right.columns.into_iter();
    let columns: Vec<ScopedColumn> = left
        .columns
        .into_iter()
        .map(|column| match right_columns.next() {
            Some(other) => ScopedColumn {
                lineage: merged(vec![column.lineage, other.lineage]),
                ..column
            },
            None => ScopedColumn {
                lineage: traced_if(column.lineage, false),
                ..column
            },
        })
        .collect();
    ResolvedQuery {
        columns,
        open,
        ..ResolvedQuery::default()
    }
}

/// A set operation matching columns by name: the left's, then the right's it lacks.
fn by_name(
    left: ResolvedQuery,
    right: ResolvedQuery,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    let open = left.open || right.open;
    let mut columns: Vec<ScopedColumn> = left.columns;
    for other in right.columns {
        match columns
            .iter_mut()
            .find(|column| context.profile.same_identifier(&column.key, &other.key))
        {
            Some(column) => {
                column.lineage = merged(vec![column.lineage.clone(), other.lineage]);
            }
            None => columns.push(other),
        }
    }
    ResolvedQuery {
        columns,
        open,
        ..ResolvedQuery::default()
    }
}

/// The CTEs in reach inside a query with `with`: the enclosing ones, then its own in order.
fn with_ctes(
    with: Option<&With>,
    ctes: &[NamedQuery],
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> Vec<NamedQuery> {
    let mut reach: Vec<NamedQuery> = ctes.to_vec();
    let recursive: bool = with.is_some_and(|with| with.recursive);
    for cte in with.map(|with| with.ctes.as_slice()).unwrap_or_default() {
        let name = key(&cte.alias, context);
        let query: ResolvedQuery = match recursive || reads_itself(cte, &name, context) {
            true => recursive_query(cte, (&name, &reach), outer, context),
            false => renamed(
                resolved_query(&cte.this, &reach, outer, context),
                &cte.columns,
                context,
            ),
        };
        reach.retain(|named| named.key != name);
        reach.push(NamedQuery { key: name, query });
    }
    reach
}

/// Whether a CTE's body reads the CTE itself, as Snowflake allows without RECURSIVE.
fn reads_itself(cte: &Cte, name: &str, context: LineageContext<'_>) -> bool {
    cte.this.dfs().any(|node| match node {
        Expression::Table(table) => {
            table.schema.is_none()
                && table.catalog.is_none()
                && context
                    .profile
                    .same_identifier(&key(&table.name, context), name)
        }
        _ => false,
    })
}

/// A recursive CTE: its anchor, then each iteration merged in until nothing changes.
fn recursive_query(
    cte: &Cte,
    (name, reach): (&str, &[NamedQuery]),
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    let anchor: Option<&Expression> = match &cte.this {
        Expression::Union(union) => Some(&union.left),
        _ => None,
    };
    let Some(anchor) = anchor else {
        return untraced(resolved_query(&cte.this, reach, outer, context));
    };
    let mut current: ResolvedQuery = renamed(
        resolved_query(anchor, reach, outer, context),
        &cte.columns,
        context,
    );
    for _ in 0..RECURSION_LIMIT {
        let mut iteration: Vec<NamedQuery> = reach.to_vec();
        iteration.push(NamedQuery {
            key: name.to_owned(),
            query: current.clone(),
        });
        let next: ResolvedQuery = renamed(
            resolved_query(&cte.this, &iteration, outer, context),
            &cte.columns,
            context,
        );
        let rows: BTreeSet<SourceColumn> = current.rows.union(&next.rows).cloned().collect();
        let joined: ResolvedQuery = ResolvedQuery {
            rows,
            ..positional(current.clone(), next)
        };
        if same_lineage(&joined, &current) {
            return joined;
        }
        current = joined;
    }
    untraced(current)
}

fn same_lineage(left: &ResolvedQuery, right: &ResolvedQuery) -> bool {
    left.rows == right.rows
        && left.columns.len() == right.columns.len()
        && left
            .columns
            .iter()
            .zip(&right.columns)
            .all(|(left, right)| left.lineage == right.lineage)
}

/// `query` with every column marked untraced.
fn untraced(query: ResolvedQuery) -> ResolvedQuery {
    let columns: Vec<ScopedColumn> = query
        .columns
        .into_iter()
        .map(|column| ScopedColumn {
            lineage: traced_if(column.lineage, false),
            ..column
        })
        .collect();
    ResolvedQuery { columns, ..query }
}

fn select_query(
    select: &Select,
    ctes: &[NamedQuery],
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    let ctes = with_ctes(select.with.as_ref(), ctes, outer, context);
    let FromItems {
        sources,
        merged,
        rows,
    } = from_sources(select, &ctes, outer, context);
    let mut scope = LineageScope {
        sources,
        merged,
        ..empty_scope(&ctes, outer)
    };
    scope.windows = named_windows(select, &scope, context);
    let mut query = ResolvedQuery {
        rows,
        ..ResolvedQuery::default()
    };
    for source in &scope.sources {
        query.rows.extend(source.query.rows.iter().cloned());
    }
    for projection in &select.expressions {
        let projection = unannotated(projection);
        if let Expression::Star(star) = projection {
            let expanded = expanded_star(star, &scope, context);
            query.open |= expanded.open;
            query.open_relation = query.open_relation.or(expanded.open_relation);
            query.open_reads = query.open_reads.or(expanded.open_reads);
            query.columns.extend(expanded.columns);
            continue;
        }
        let column = ScopedColumn {
            name: output_name(projection, context),
            key: output_key(projection, context),
            lineage: expression_lineage(projection, &scope, context),
        };
        query.rows.extend(column.lineage.rows.iter().cloned());
        scope.aliases.push(column.clone());
        query.columns.push(column);
    }
    for filter in row_filters(select) {
        query.rows.extend(everything_read(&expression_lineage(
            filter, &scope, context,
        )));
    }
    query
}

/// The expressions that decide a level's rows: WHERE, GROUP BY, HAVING, QUALIFY and CONNECT BY.
fn row_filters(select: &Select) -> Vec<&Expression> {
    let mut filters: Vec<&Expression> = Vec::new();
    filters.extend(select.where_clause.iter().map(|clause| &clause.this));
    filters.extend(
        select
            .group_by
            .iter()
            .flat_map(|group| group.expressions.iter()),
    );
    filters.extend(select.having.iter().map(|clause| &clause.this));
    filters.extend(select.qualify.iter().map(|clause| &clause.this));
    if let Some(connect) = &select.connect {
        filters.extend(connect.start.iter());
        filters.push(&connect.connect);
    }
    filters
}

/// What each named window's partitions and orders read.
fn named_windows(
    select: &Select,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Vec<(String, Lineage)> {
    let mut windows: Vec<(String, Lineage)> = Vec::new();
    for window in select.windows.iter().flatten() {
        let mut read: Vec<Lineage> = Vec::new();
        for partition in &window.spec.partition_by {
            read.push(expression_lineage(partition, scope, context));
        }
        for order in &window.spec.order_by {
            read.push(expression_lineage(&order.this, scope, context));
        }
        windows.push((
            key(&window.name, context),
            combined(read, TransformKind::Expression),
        ));
    }
    windows
}

/// The columns a star selects (a bare star's merged join keys once, first), EXCLUDE etc. applied.
pub(crate) fn expanded_star(
    star: &Star,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    let qualifier = star.table.as_ref().map(|table| key(table, context));
    let selected: Vec<&ScopeSource> = scope
        .sources
        .iter()
        .filter(|source| match (&qualifier, &source.key) {
            (None, _) => true,
            (Some(qualifier), Some(key)) => context.profile.same_identifier(key, qualifier),
            (Some(_), None) => false,
        })
        .collect();
    let excluded: Vec<String> = star
        .except
        .iter()
        .flatten()
        .map(|name| key(name, context))
        .collect();
    let keys: Vec<&ScopedColumn> = match qualifier {
        None => scope.merged.iter().collect(),
        Some(_) => Vec::new(),
    };
    let merged_keys: Vec<String> = keys.iter().map(|column| column.key.clone()).collect();
    let open: Vec<&ScopeSource> = selected
        .iter()
        .copied()
        .filter(|source| source.query.open)
        .collect();
    let mut columns: Vec<ScopedColumn> = Vec::new();
    let offered = keys.into_iter().chain(
        selected
            .iter()
            .flat_map(|source| source.query.columns.iter())
            .filter(|column| !listed(&merged_keys, &column.key, context)),
    );
    for column in offered {
        if !listed(&excluded, &column.key, context) {
            columns.push(star_column(column, star, scope, context));
        }
    }
    let only: Option<&ScopeSource> = match open.as_slice() {
        [only] => Some(only),
        _ => None,
    };
    ResolvedQuery {
        columns,
        open: !open.is_empty(),
        open_relation: only.and_then(|only| only.query.open_relation.clone()),
        open_reads: only.and_then(|only| only.query.open_reads.clone()),
        rows: BTreeSet::new(),
    }
}

fn star_column(
    column: &ScopedColumn,
    star: &Star,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> ScopedColumn {
    let same = |name: &Identifier| {
        context
            .profile
            .same_identifier(&key(name, context), &column.key)
    };
    if let Some(replacement) = star
        .replace
        .iter()
        .flatten()
        .find(|alias| same(&alias.alias))
    {
        return ScopedColumn {
            lineage: expression_lineage(&replacement.this, scope, context),
            ..column.clone()
        };
    }
    match star.rename.iter().flatten().find(|(old, _)| same(old)) {
        Some((_, new)) => ScopedColumn {
            name: new.name.clone(),
            key: key(new, context),
            lineage: column.lineage.clone(),
        },
        None => column.clone(),
    }
}

/// An output's name: its alias, column, DuckDB struct field or Snowflake's generated name.
fn output_name(projection: &Expression, context: LineageContext<'_>) -> String {
    match projection {
        Expression::Alias(alias) => alias.alias.name.clone(),
        Expression::Column(column) => column.name.name.clone(),
        Expression::Dot(dot) if context.profile.name() == DUCKDB_PROFILE => dot.field.name.clone(),
        _ if context.profile.name() == DUCKDB_PROFILE => String::new(),
        _ => snowflake_name(projection, context).unwrap_or_default(),
    }
}

/// Snowflake's name for unaliased text, `AMOUNT::INT`; none where spacing or quotes may differ.
fn snowflake_name(projection: &Expression, context: LineageContext<'_>) -> Option<String> {
    let span = projection.source_span()?;
    let text: String = context
        .sql?
        .chars()
        .skip(span.start)
        .take(span.end.checked_sub(span.start)?)
        .collect();
    let plain = !text.is_empty()
        && !text
            .chars()
            .any(|character| character.is_whitespace() || UNNAMED_CHARACTERS.contains(&character));
    plain.then(|| text.to_uppercase())
}

fn output_key(projection: &Expression, context: LineageContext<'_>) -> String {
    match projection {
        Expression::Alias(alias) => key(&alias.alias, context),
        Expression::Column(column) => key(&column.name, context),
        _ => plain_key(&output_name(projection, context), context),
    }
}

/// The expression a comment annotation wraps.
pub(crate) fn unannotated(expression: &Expression) -> &Expression {
    match expression {
        Expression::Annotated(annotated) => unannotated(&annotated.this),
        _ => expression,
    }
}
