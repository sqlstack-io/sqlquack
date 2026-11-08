//! The outputs of a query, set operation or CTE, with their lineage.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal, Star};
use sqlquack_syntax::expressions::queries::models::{Cte, OrderBy, Select, Union, With};

use crate::lineage::_helpers::tracing::expressions::{expression_lineage, is_aggregate};
use crate::lineage::_helpers::tracing::lineages::{combined, set_merged, traced_if, unresolved};
use crate::lineage::_helpers::tracing::postgres_names::postgres_name;
use crate::lineage::_helpers::tracing::resolution::{key, plain_key};
use crate::lineage::_helpers::tracing::reuse::{holds_aggregate, reused, worked};
use crate::lineage::_helpers::tracing::sources::{from_sources, listed, values_query};
use crate::lineage::_helpers::typing::rules::filtered_non_null;
use crate::lineage::constants::{
    DUCKDB_PROFILE, ORDINAL_BASE, POSTGRES_PROFILE, RECURSION_LIMIT, SQL_STAR, SQLITE_PROFILE,
    SUBTOTAL_GROUPINGS, UNNAMED_CHARACTERS,
};
use crate::lineage::models::{
    FromItems, Lineage, LineageContext, LineageScope, NamedQuery, Nullability, ResolvedQuery,
    RowReads, ScopeSource, ScopedColumn, TransformKind,
};

/// The outputs of `query`, with `ctes` in reach and `outer` the enclosing level.
pub(crate) fn resolved_query(
    query: &Expression,
    ctes: &[NamedQuery],
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    let resolve = || {
        worked();
        crate::lineage::_helpers::tracing::stack::deeper(|| {
            resolved_query_level(query, ctes, outer, context)
        })
    };
    match outer {
        None => reused(query, ctes, resolve),
        Some(_) => resolve(),
    }
}

fn resolved_query_level(
    query: &Expression,
    ctes: &[NamedQuery],
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    match query {
        Expression::Select(select) => select_query(select, ctes, outer, context),
        Expression::Union(union) if union.by_name => {
            let ctes = with_ctes(union.with.as_ref(), ctes, outer, context);
            let sides: (&Expression, &Expression) = (&union.left, &union.right);
            set_operation(sides, (&ctes, outer), context, SetSides::ByName)
        }
        Expression::Union(union) => {
            let ctes = with_ctes(union.with.as_ref(), ctes, outer, context);
            let branches: Vec<ResolvedQuery> = union_branches(union)
                .into_iter()
                .map(|branch| resolved_query(branch, &ctes, outer, context))
                .collect();
            let mut query: ResolvedQuery = positional(branches, context);
            if union.limit.is_some() || union.offset.is_some() {
                let order: Vec<&Expression> = order_keys(union.order_by.as_ref());
                for read in output_reads(&order, &query.columns, context) {
                    query.rows.read(&read);
                }
            }
            query
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
        non_null: Vec::new(),
        traced: RefCell::default(),
    }
}

/// `query`'s outputs renamed by a column alias list; a DuckDB VALUES list all renamed is judged.
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
        .enumerate()
        .map(|(position, column)| match aliases.get(position) {
            Some(alias) => ScopedColumn {
                name: alias.name.clone(),
                key: key(alias, context),
                lineage: column.lineage,
            },
            None => column,
        })
        .collect();
    let from_values: bool = query.from_values
        && !(context.profile.name() == DUCKDB_PROFILE && aliases.len() >= columns.len());
    ResolvedQuery {
        columns,
        stored_relation: None,
        from_values,
        ..query
    }
}

/// Which sides of a set operation its rows and values come from.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SetSides {
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
    let mut rows: RowReads = left.rows.clone();
    rows.absorb(&right.rows);
    if matches!(sides, SetSides::Left | SetSides::Intersect) {
        for column in &right.columns {
            rows.read(&column.lineage);
        }
    }
    let query: ResolvedQuery = match sides {
        SetSides::Left => left_typed_by_both(left, &right, context),
        SetSides::ByName => by_name(left, right, context),
        SetSides::Intersect => positional(vec![left, right], context),
    };
    ResolvedQuery { rows, ..query }
}

/// EXCEPT's columns: values and rows from the left, each typed as both branches combine.
fn left_typed_by_both(
    mut left: ResolvedQuery,
    right: &ResolvedQuery,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    for (column, other) in left.columns.iter_mut().zip(&right.columns) {
        column.lineage.data_type = set_merged(
            vec![column.lineage.clone(), other.lineage.clone()],
            context.profile,
        )
        .data_type;
    }
    left
}

/// The branches of a chain of positional UNIONs, in order, so they merge in one pass.
fn union_branches(union: &Union) -> Vec<&Expression> {
    let mut branches: Vec<&Expression> = vec![&union.right];
    let mut left: &Expression = &union.left;
    while let Expression::Union(inner) = left {
        if inner.by_name || inner.with.is_some() {
            break;
        }
        branches.push(&inner.right);
        left = &inner.left;
    }
    branches.push(left);
    branches.reverse();
    branches
}

/// Queries' columns merged by position: the first query's names, every branch's values and rows.
fn positional(queries: Vec<ResolvedQuery>, context: LineageContext<'_>) -> ResolvedQuery {
    let open: bool = queries.iter().any(|query| query.open);
    let mut rows: RowReads = RowReads::default();
    let mut branches: Vec<std::vec::IntoIter<ScopedColumn>> = Vec::with_capacity(queries.len());
    for query in queries {
        rows.absorb(&query.rows);
        branches.push(query.columns.into_iter());
    }
    let Some((first, others)) = branches.split_first_mut() else {
        return ResolvedQuery::default();
    };
    let mut columns: Vec<ScopedColumn> = Vec::new();
    for column in first {
        let mut parts: Vec<Lineage> = vec![column.lineage];
        let mut complete: bool = true;
        for other in others.iter_mut() {
            match other.next() {
                Some(other) => parts.push(other.lineage),
                None => complete = false,
            }
        }
        columns.push(ScopedColumn {
            lineage: traced_if(set_merged(parts, context.profile), complete),
            ..column
        });
    }
    ResolvedQuery {
        columns,
        open,
        rows,
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
    let mut matched: Vec<bool> = vec![false; columns.len()];
    for mut other in right.columns {
        match columns
            .iter()
            .position(|column| context.profile.same_identifier(&column.key, &other.key))
        {
            Some(position) => {
                let column: &mut ScopedColumn = &mut columns[position];
                column.lineage =
                    set_merged(vec![column.lineage.clone(), other.lineage], context.profile);
                matched[position] = true;
            }
            None => {
                if !left.open {
                    other.lineage.nullability = Nullability::Nullable;
                }
                columns.push(other);
            }
        }
    }
    if !right.open {
        for (column, _) in columns
            .iter_mut()
            .zip(&matched)
            .filter(|(_, matched)| !**matched)
        {
            column.lineage.nullability = Nullability::Nullable;
        }
    }
    ResolvedQuery {
        columns,
        open,
        ..ResolvedQuery::default()
    }
}

/// The CTEs in reach inside a query with `with`: the enclosing ones, then its own in order.
pub(crate) fn with_ctes<'a>(
    with: Option<&With>,
    ctes: &'a [NamedQuery],
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> Cow<'a, [NamedQuery]> {
    with_ctes_visiting(with, (ctes, outer), context, &mut |_, _| false)
}

/// `with_ctes`, showing `visit` each CTE with what its body reads, a DuckDB recursive one itself.
pub(crate) fn with_ctes_visiting<'a>(
    with: Option<&With>,
    (ctes, outer): (&'a [NamedQuery], Option<&LineageScope<'_>>),
    context: LineageContext<'_>,
    visit: &mut dyn FnMut(&Cte, &[NamedQuery]) -> bool,
) -> Cow<'a, [NamedQuery]> {
    let Some(with) = with else {
        return Cow::Borrowed(ctes);
    };
    let mut reach: Vec<NamedQuery> = ctes.to_vec();
    for cte in &with.ctes {
        let name = key(&cte.alias, context);
        let reads_own: bool = reads_itself(cte, &name, context);
        let mut query: ResolvedQuery = match with.recursive || reads_own {
            true => recursive_query(cte, (&name, &reach), outer, context),
            false => renamed(
                resolved_query(&cte.this, &reach, outer, context),
                &cte.columns,
                context,
            ),
        };
        let faulted: bool = match reads_own && context.profile.name() == DUCKDB_PROFILE {
            true => {
                let mut own: Vec<NamedQuery> = reach.clone();
                own.retain(|named| named.key != name);
                own.push(NamedQuery {
                    key: name.clone(),
                    query: query.clone(),
                });
                visit(cte, &own)
            }
            false => visit(cte, &reach),
        };
        query.faulted = faulted;
        reach.retain(|named| named.key != name);
        reach.push(NamedQuery { key: name, query });
    }
    Cow::Owned(reach)
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

/// A recursive CTE: its anchor, then each iteration merged in; DuckDB keeps the anchor's types.
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
    let anchor_types: Vec<Option<Rc<str>>> = current
        .columns
        .iter()
        .map(|column| column.lineage.data_type.clone())
        .collect();
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
        let mut joined: ResolvedQuery = positional(vec![current.clone(), next], context);
        if context.profile.name() == DUCKDB_PROFILE {
            for (column, data_type) in joined.columns.iter_mut().zip(&anchor_types) {
                column.lineage.data_type.clone_from(data_type);
            }
        }
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

/// The scope a SELECT's own clauses read: its FROM items, merged keys, windows and filters.
pub(crate) fn level_scope<'a>(
    select: &'a Select,
    ctes: &'a [NamedQuery],
    outer: Option<&'a LineageScope<'a>>,
    context: LineageContext<'_>,
) -> LineageScope<'a> {
    level_scope_and_rows(select, ctes, outer, context).0
}

fn level_scope_and_rows<'a>(
    select: &'a Select,
    ctes: &'a [NamedQuery],
    outer: Option<&'a LineageScope<'a>>,
    context: LineageContext<'_>,
) -> (LineageScope<'a>, RowReads) {
    let FromItems {
        sources,
        merged,
        rows,
    } = from_sources(select, ctes, outer, context);
    let mut scope = LineageScope {
        sources,
        merged,
        ..empty_scope(ctes, outer)
    };
    scope.windows = named_windows(select, &scope, context);
    scope.non_null = filtered_non_null(select);
    scope.traced.get_mut().clear();
    (scope, rows)
}

fn select_query(
    select: &Select,
    ctes: &[NamedQuery],
    outer: Option<&LineageScope<'_>>,
    context: LineageContext<'_>,
) -> ResolvedQuery {
    let ctes = with_ctes(select.with.as_ref(), ctes, outer, context);
    let (mut scope, rows) = level_scope_and_rows(select, &ctes, outer, context);
    let mut query = ResolvedQuery {
        rows,
        ..ResolvedQuery::default()
    };
    for source in &scope.sources {
        query.rows.absorb(&source.query.rows);
    }
    let mut ungrouped: Vec<Lineage> = Vec::new();
    let mut aggregated: Vec<bool> = Vec::new();
    for projection in &select.expressions {
        let projection = unannotated(projection);
        if let Expression::Star(star) = projection {
            let expanded = expanded_star(star, &scope, context);
            query.open |= expanded.open;
            query.open_relation = query.open_relation.or(expanded.open_relation);
            query.open_reads = query.open_reads.or(expanded.open_reads);
            query.outer_nullable |= expanded.outer_nullable;
            ungrouped.extend(expanded.columns.iter().map(|column| column.lineage.clone()));
            aggregated.extend(expanded.columns.iter().map(|_| false));
            query.columns.extend(expanded.columns);
            continue;
        }
        let column = ScopedColumn {
            name: output_name(projection, context),
            key: output_key(projection, context),
            lineage: expression_lineage(projection, &scope, context),
        };
        query.rows.rows_of(&column.lineage);
        if !holds_aggregate(projection) {
            ungrouped.push(column.lineage.clone());
        }
        scope.aliases.push(column.clone());
        scope.traced.get_mut().clear();
        aggregated.push(is_aggregate(unwrapped_projection(projection)));
        query.columns.push(column);
    }
    if subtotals(select) {
        for (column, aggregate) in query.columns.iter_mut().zip(&aggregated) {
            if !aggregate {
                column.lineage.nullability = Nullability::Nullable;
            }
        }
    }
    for filter in row_filters(select) {
        query
            .rows
            .read(&expression_lineage(filter, &scope, context));
    }
    let grouped_all: bool = select
        .group_by
        .as_ref()
        .is_some_and(|group| group.all == Some(true));
    if grouped_all {
        for read in &ungrouped {
            query.rows.read(read);
        }
    }
    let grouping = select
        .group_by
        .iter()
        .flat_map(|group| group.expressions.iter());
    for item in grouping.chain(ordering(select)) {
        let read: Lineage = match ordinal(item) {
            Some(position) => output_at(&query.columns, position),
            None => expression_lineage(item, &scope, context),
        };
        query.rows.read(&read);
    }
    query
}

/// Whether the level groups by ROLLUP, CUBE or GROUPING SETS, whose subtotal rows NULL the keys.
fn subtotals(select: &Select) -> bool {
    select
        .group_by
        .iter()
        .flat_map(|group| group.expressions.iter())
        .flat_map(|item| item.dfs())
        .any(subtotal_grouping)
}

fn subtotal_grouping(node: &Expression) -> bool {
    match node {
        Expression::Rollup(_) | Expression::Cube(_) | Expression::GroupingSets(_) => true,
        Expression::Function(function) => {
            SUBTOTAL_GROUPINGS.contains(&function.name.to_ascii_uppercase().as_str())
        }
        _ => false,
    }
}

/// A projection without its alias, parentheses or annotations.
fn unwrapped_projection(mut projection: &Expression) -> &Expression {
    loop {
        projection = match projection {
            Expression::Alias(alias) => &alias.this,
            Expression::Paren(paren) => &paren.this,
            Expression::Annotated(annotated) => &annotated.this,
            _ => return projection,
        };
    }
}

/// The ORDER BY keys that decide rows: those of a level with LIMIT, OFFSET, TOP or FETCH.
fn ordering(select: &Select) -> Vec<&Expression> {
    let limited: bool = select.limit.is_some()
        || select.offset.is_some()
        || select.top.is_some()
        || select.fetch.is_some();
    match (limited, &select.order_by) {
        (true, Some(order)) => order
            .expressions
            .iter()
            .map(|ordered| &ordered.this)
            .collect(),
        _ => Vec::new(),
    }
}

/// What ORDER BY keys read: an ordinal or output name reads that output, else nothing traced.
fn output_reads(
    keys: &[&Expression],
    columns: &[ScopedColumn],
    context: LineageContext<'_>,
) -> Vec<Lineage> {
    keys.iter()
        .map(|item| match (ordinal(item), unannotated(item)) {
            (Some(position), _) => output_at(columns, position),
            (None, Expression::Column(column)) if column.table.is_none() => {
                named_output(columns, &key(&column.name, context), context)
            }
            (None, _) => unresolved(),
        })
        .collect()
}

/// The lineage of the output named `name`, untraced when there is none.
fn named_output(columns: &[ScopedColumn], name: &str, context: LineageContext<'_>) -> Lineage {
    match columns
        .iter()
        .find(|output| context.profile.same_identifier(&output.key, name))
    {
        Some(output) => output.lineage.clone(),
        None => unresolved(),
    }
}

/// The keys of an ORDER BY, when there is one.
fn order_keys(order: Option<&OrderBy>) -> Vec<&Expression> {
    match order {
        Some(order) => order
            .expressions
            .iter()
            .map(|ordered| &ordered.this)
            .collect(),
        None => Vec::new(),
    }
}

/// The 0-based output a positional `n` in GROUP BY or ORDER BY names.
fn ordinal(item: &Expression) -> Option<usize> {
    let Expression::Literal(literal) = unannotated(item) else {
        return None;
    };
    let Literal::Number(text) = literal.as_ref() else {
        return None;
    };
    match text.parse::<usize>() {
        Ok(number) => number.checked_sub(ORDINAL_BASE),
        Err(_) => None,
    }
}

/// The lineage of the output at `position`, untraced when there is none.
fn output_at(columns: &[ScopedColumn], position: usize) -> Lineage {
    columns
        .get(position)
        .map_or_else(unresolved, |column| column.lineage.clone())
}

/// The expressions that decide a level's rows: WHERE, HAVING, QUALIFY and CONNECT BY.
fn row_filters(select: &Select) -> Vec<&Expression> {
    let mut filters: Vec<&Expression> = Vec::new();
    filters.extend(select.where_clause.iter().map(|clause| &clause.this));
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
    let named = |level: &LineageScope<'_>| -> bool {
        level.sources.iter().any(|source| {
            matches!((&qualifier, &source.key), (Some(qualifier), Some(key))
                if context.profile.same_identifier(key, qualifier))
        })
    };
    let mut owner: &LineageScope<'_> = scope;
    while qualifier.is_some() && !named(owner) {
        let Some(outer) = owner.outer else {
            owner = scope;
            break;
        };
        owner = outer;
    }
    let scope: &LineageScope<'_> = owner;
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
        stored_relation: None,
        from_values: false,
        written_name: None,
        function_name: None,
        open_reads: only.and_then(|only| only.query.open_reads.clone()),
        rows: RowReads::default(),
        outer_nullable: only.is_some_and(|only| only.query.outer_nullable),
        derived: false,
        faulted: false,
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
        Expression::Literal(literal) if context.profile.name() == DUCKDB_PROFILE => {
            match literal.as_ref() {
                Literal::Number(text) => text.clone(),
                _ => String::new(),
            }
        }
        _ if matches!(context.profile.name(), DUCKDB_PROFILE | SQLITE_PROFILE) => String::new(),
        _ if context.profile.name() == POSTGRES_PROFILE => postgres_name(projection),
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

pub(crate) fn output_key(projection: &Expression, context: LineageContext<'_>) -> String {
    match projection {
        Expression::Alias(alias) => key(&alias.alias, context),
        Expression::Column(column) => key(&column.name, context),
        _ => plain_key(&output_name(projection, context), context),
    }
}

/// The expression a comment annotation wraps; a typed marker is kept, as it types its value.
pub(crate) fn unannotated(expression: &Expression) -> &Expression {
    match expression {
        Expression::Annotated(annotated)
            if annotated.type_assertion.is_none() && annotated.type_like.is_none() =>
        {
            unannotated(&annotated.this)
        }
        _ => expression,
    }
}

/// Whether the outermost level of `query` (a set operation's first branch) projects a star.
pub(crate) fn projects_star(query: &Expression) -> bool {
    match query {
        Expression::Select(select) => select.expressions.iter().any(is_star),
        Expression::Union(set) => projects_star(&set.left),
        Expression::Intersect(set) => projects_star(&set.left),
        Expression::Except(set) => projects_star(&set.left),
        Expression::Subquery(subquery) => projects_star(&subquery.this),
        Expression::Annotated(annotated) => projects_star(&annotated.this),
        _ => false,
    }
}

/// Whether a projection is `*` or `t.*`, past aliases, annotations and parentheses.
fn is_star(projection: &Expression) -> bool {
    match projection {
        Expression::Star(_) => true,
        Expression::Column(column) => column.name.name == SQL_STAR,
        Expression::Alias(alias) => is_star(&alias.this),
        Expression::Annotated(annotated) => is_star(&annotated.this),
        Expression::Paren(paren) => is_star(&paren.this),
        _ => false,
    }
}
