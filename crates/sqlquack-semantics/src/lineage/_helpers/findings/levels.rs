//! Walking a query level by level: each level's clauses read in its scope, inner queries after.

use std::collections::HashMap;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::queries::models::{
    JoinKind, NamedWindow, OrderBy, Select, Subquery, With,
};

use crate::dialects::main::dialect_rules::dialect_rules;
use crate::lineage::_helpers::checks::issues::in_scope;
use crate::lineage::_helpers::checks::node_checks::{NodeLevel, node_findings};
use crate::lineage::_helpers::checks::scope_rules::{
    RuleLevel, select_rules, set_order, set_order_by,
};
use crate::lineage::_helpers::checks::sets::{Branches, set_findings};
use crate::lineage::_helpers::findings::checks::{
    Level, alias_count_findings, column_finding, columns_pattern_finding, level_outputs,
    unpivot_findings, using_findings,
};
use crate::lineage::_helpers::tracing::expressions::date_part_argument;
use crate::lineage::_helpers::tracing::queries::{
    after_windows, clauses_within, empty_scope, level_scope, output_key, takes_clauses,
    unannotated, with_ctes_visiting,
};
use crate::lineage::_helpers::tracing::resolution::key;
use crate::lineage::_helpers::tracing::reuse::{Reach, Reuse, reusing};
use crate::lineage::_helpers::tracing::sources::{every_join, from_parts};
use crate::lineage::constants::{COLUMNS_FUNCTION, OUTER_RIGHT_JOINS};
use crate::lineage::models::Engine;
use crate::lineage::models::NameFinding;
use crate::lineage::models::{Lineage, LineageContext, LineageScope, NamedQuery};
use crate::lineage::types::DialectRules;

/// The findings of `query` and its inner queries; `aliases` are enclosing levels' output aliases.
pub(crate) fn query_findings(
    query: &Expression,
    (ctes, outer, aliases): (&[NamedQuery], Option<&LineageScope<'_>>, &[String]),
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    crate::lineage::_helpers::tracing::stack::deeper(|| {
        query_findings_level(query, (ctes, outer, aliases), context)
    })
}

fn query_findings_level(
    query: &Expression,
    (ctes, outer, aliases): (&[NamedQuery], Option<&LineageScope<'_>>, &[String]),
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let branches = |with: Option<&With>, left: &Expression, right: &Expression| {
        let (ctes, mut found) = visited_ctes((query, with), (ctes, outer, aliases), context);
        let _reuse: Reuse = reusing(&ctes);
        let mut branches: Vec<NameFinding> = query_findings(left, (&ctes, outer, aliases), context);
        branches.extend(query_findings(right, (&ctes, outer, aliases), context));
        branches.append(&mut found);
        if typed_rules(context) {
            branches.extend(set_order(query, context));
            branches.extend(set_findings(
                query,
                Branches { ctes: &ctes, outer },
                context,
            ));
        }
        branches
    };
    match query {
        Expression::Select(select) => {
            select_findings((query, select, &[]), (ctes, outer, aliases), context)
        }
        Expression::Subquery(subquery) if takes_clauses(subquery) => {
            let (inner, after) = clauses_within(query);
            let orders: Vec<&OrderBy> = after
                .iter()
                .filter_map(|node| node.order_by.as_ref())
                .collect();
            match inner {
                Expression::Select(select) => {
                    select_findings((inner, select, &after), (ctes, outer, aliases), context)
                }
                _ => {
                    let mut found: Vec<NameFinding> =
                        query_findings(inner, (ctes, outer, aliases), context);
                    if typed_rules(context) {
                        for order in orders {
                            found.extend(set_order_by(inner, Some(order), context));
                        }
                    }
                    found
                }
            }
        }
        Expression::Union(union) => branches(union.with.as_ref(), &union.left, &union.right),
        Expression::Intersect(intersect) => {
            branches(intersect.with.as_ref(), &intersect.left, &intersect.right)
        }
        Expression::Except(except) => branches(except.with.as_ref(), &except.left, &except.right),
        Expression::Subquery(subquery) => {
            query_findings(&subquery.this, (ctes, outer, aliases), context)
        }
        Expression::Paren(paren) => query_findings(&paren.this, (ctes, outer, aliases), context),
        Expression::Annotated(annotated) => {
            query_findings(&annotated.this, (ctes, outer, aliases), context)
        }
        _ => Vec::new(),
    }
}

/// Whether the profile takes the scope, type and set-operation rules; ClickHouse keeps names only.
fn typed_rules(context: LineageContext<'_>) -> bool {
    !dialect_rules(context.profile).checks_names_only()
}

/// The CTEs `with` adds to `ctes`, each read body's findings, and under PostgreSQL each one faulted.
fn visited_ctes<'a>(
    (owner, with): (&Expression, Option<&With>),
    (ctes, outer, aliases): (&'a [NamedQuery], Option<&LineageScope<'_>>, &[String]),
    context: LineageContext<'_>,
) -> (Reach<'a>, Vec<NameFinding>) {
    let mut found: Vec<NameFinding> = Vec::new();
    let postgres: bool = Engine::of(context) == Engine::Postgres;
    let unread: Vec<String> = match (Engine::of(context), with) {
        (Engine::DuckDB, Some(with)) => unread_ctes(owner, with, context),
        _ => Vec::new(),
    };
    let reach = with_ctes_visiting(with, (ctes, outer), context, |cte, reach| {
        let body: Vec<NameFinding> = query_findings(&cte.this, (reach, outer, aliases), context);
        let faulted: bool =
            postgres && (!body.is_empty() || reads_faulted(&cte.this, reach, context));
        if !unread.contains(&key(&cte.alias, context)) {
            found.extend(body);
        }
        faulted
    });
    (reach, found)
}

/// The CTEs of `with` that `owner` never reads, nor any CTE it reads: DuckDB does not bind them.
fn unread_ctes(owner: &Expression, with: &With, context: LineageContext<'_>) -> Vec<String> {
    let names: Vec<String> = with
        .ctes
        .iter()
        .map(|cte| key(&cte.alias, context))
        .collect();
    let mut indices: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, name) in names.iter().enumerate() {
        indices
            .entry(name.to_ascii_lowercase())
            .or_default()
            .push(index);
    }
    let reads = |query: &Expression| -> Vec<usize> {
        query
            .dfs()
            .filter_map(|node| match node {
                Expression::Table(table) if table.schema.is_none() && table.catalog.is_none() => {
                    indices.get(&key(&table.name, context).to_ascii_lowercase())
                }
                _ => None,
            })
            .flatten()
            .copied()
            .collect()
    };
    let mut outside: Vec<isize> = vec![0; names.len()];
    for index in reads(owner) {
        outside[index] += 1;
    }
    let bodies: Vec<Vec<usize>> = with.ctes.iter().map(|cte| reads(&cte.this)).collect();
    for index in bodies.iter().flatten() {
        outside[*index] -= 1;
    }
    let mut read: Vec<bool> = outside.iter().map(|count| *count > 0).collect();
    let mut pending: Vec<usize> = (0..names.len()).filter(|index| read[*index]).collect();
    while let Some(index) = pending.pop() {
        for &next in &bodies[index] {
            if !read[next] {
                read[next] = true;
                pending.push(next);
            }
        }
    }
    names
        .into_iter()
        .zip(read)
        .filter_map(|(name, read)| (!read).then_some(name))
        .collect()
}

/// The parts before the `index`th FROM part, then `outer`; none when no part comes before it.
fn left_scope<'a>(
    scope: &LineageScope<'_>,
    index: usize,
    (ctes, outer): (&'a [NamedQuery], Option<&'a LineageScope<'a>>),
) -> Option<LineageScope<'a>> {
    let start: usize = scope
        .starts
        .get(index)
        .map_or(scope.sources.len(), |start| *start);
    (start > 0).then(|| LineageScope {
        sources: scope.sources[..start].to_vec(),
        merged: scope.merged.clone(),
        ..empty_scope(ctes, outer)
    })
}

/// The key of a derived table in FROM, by its alias; none when `item` is not a derived table.
fn derived_key(item: &Expression, context: LineageContext<'_>) -> Option<Option<String>> {
    match item {
        Expression::Subquery(subquery) => {
            Some(subquery.alias.as_ref().map(|alias| key(alias, context)))
        }
        Expression::Alias(alias) if matches!(alias.this, Expression::Subquery(_)) => {
            Some(Some(key(&alias.alias, context)))
        }
        _ => None,
    }
}

/// Whether `query` names a faulted CTE among `reach`.
fn reads_faulted(query: &Expression, reach: &[NamedQuery], context: LineageContext<'_>) -> bool {
    query.dfs().any(|node| match node {
        Expression::Table(table) if table.schema.is_none() && table.catalog.is_none() => {
            faulted_cte(&key(&table.name, context), reach, context)
        }
        _ => false,
    })
}

/// Whether the CTE `name` resolves to among `reach` is faulted.
fn faulted_cte(name: &str, reach: &[NamedQuery], context: LineageContext<'_>) -> bool {
    reach
        .iter()
        .any(|named| named.query.faulted && context.profile.same_identifier(&named.key, name))
}

fn select_findings(
    (node, select, after): (&Expression, &Select, &[&Subquery]),
    (ctes, outer, aliases): (&[NamedQuery], Option<&LineageScope<'_>>, &[String]),
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let (ctes, mut nested) = visited_ctes(
        (node, select.with.as_ref()),
        (ctes, outer, aliases),
        context,
    );
    let _reuse: Reuse = reusing(&ctes);
    let mut scope: LineageScope<'_> = level_scope(select, &ctes, outer, context);
    let found: Vec<(String, Lineage)> = after_windows(after, &scope, context);
    scope.windows.extend(found);
    let orders: Vec<&OrderBy> = after
        .iter()
        .filter_map(|node| node.order_by.as_ref())
        .collect();
    let level = NodeLevel {
        scope: &scope,
        context,
    };
    if typed_rules(context) {
        nested.extend(select_rules(
            (node, select),
            RuleLevel {
                scope: &scope,
                context,
            },
        ));
        let after_orders = orders.iter().flat_map(|order| order_keys(order));
        let after_windows = after
            .iter()
            .flat_map(|node| node.windows.iter().flatten())
            .flat_map(window_keys);
        let after_keys = after_orders.chain(after_windows);
        for checked in in_scope(node)
            .into_iter()
            .chain(after_keys.flat_map(in_scope))
        {
            nested.extend(node_findings(checked, level));
        }
    }
    let mut own: Vec<NameFinding> = alias_count_findings(select, &scope, context);
    own.extend(using_findings(select, &scope, context));
    let postgres: bool = Engine::of(context) == Engine::Postgres;
    let rules: &dyn DialectRules = dialect_rules(context.profile);
    let anywhere: bool = rules.outputs_read_anywhere();
    let left_reading: bool = rules.derived_tables_read_left_items();
    for (index, (item, join)) in from_parts(select).into_iter().enumerate() {
        let implicit: bool = left_reading
            && derived_key(unannotated(item), context).is_some()
            && join.is_none_or(|join| !OUTER_RIGHT_JOINS.contains(&join.kind));
        let left: Option<LineageScope<'_>> = (lateral(item) || implicit)
            .then(|| left_scope(&scope, index, (&ctes, outer)))
            .flatten();
        let reads: Option<&LineageScope<'_>> = left.as_ref().or(outer);
        let body: Vec<NameFinding> =
            query_findings(unannotated(item), (&ctes, reads, aliases), context);
        if let Some(derived) = derived_key(unannotated(item), context)
            && postgres
            && (!body.is_empty() || reads_faulted(item, &ctes, context))
        {
            scope
                .sources
                .iter_mut()
                .filter(|source| source.query.derived && source.key == derived)
                .for_each(|source| source.query.faulted = true);
            scope.traced.get_mut().clear();
        }
        nested.extend(body);
        own.extend(unpivot_findings(item, &ctes, context));
    }
    let own_outputs: Vec<String> = aliased_outputs(select, context);
    let level = Level {
        scope: &scope,
        enclosing: aliases,
        own: &own_outputs,
        hidden: None,
        connect: select.connect.is_some(),
        filtering_join: every_join(select).into_iter().any(|join| {
            matches!(
                join.kind,
                JoinKind::Semi
                    | JoinKind::Anti
                    | JoinKind::LeftSemi
                    | JoinKind::LeftAnti
                    | JoinKind::RightSemi
                    | JoinKind::RightAnti
            )
        }),
    };
    let unnamed = Level {
        enclosing: &[],
        own: &[],
        ..level
    };
    for projection in &select.expressions {
        let own_name: Option<String> =
            anywhere.then(|| output_key(unannotated(projection), context));
        let projected = match &own_name {
            Some(own_name) => Level {
                hidden: Some(own_name),
                ..level
            },
            None => unnamed,
        };
        let (found, inner) = clause_findings(projection, &projected, context);
        own.extend(found);
        nested.extend(inner);
    }
    let mut read: Vec<&Expression> = clauses(select);
    read.extend(orders.iter().flat_map(|order| order_keys(order)));
    for clause in read {
        let named_place: bool = ordered_name_place(select, &orders, clause)
            || (rules.group_keys_read_outputs() && group_key_place(select, clause));
        let read = match (anywhere, named_place) {
            (true, _) => level,
            (false, true) => Level {
                enclosing: &[],
                ..level
            },
            (false, false) => unnamed,
        };
        let (found, inner) = clause_findings(clause, &read, context);
        own.extend(found);
        nested.extend(inner);
    }
    own.sort_by_key(|finding| finding.span.map(|span| span.start));
    own.extend(nested);
    own
}

/// The names of `select`'s aliased outputs, which later clauses may read.
fn aliased_outputs(select: &Select, context: LineageContext<'_>) -> Vec<String> {
    select
        .expressions
        .iter()
        .filter(|projection| matches!(unannotated(projection), Expression::Alias(_)))
        .map(|projection| output_key(projection, context))
        .collect()
}

/// Whether `clause` is a GROUP BY item of `select`.
fn group_key_place(select: &Select, clause: &Expression) -> bool {
    select
        .group_by
        .iter()
        .flat_map(|group| &group.expressions)
        .any(|candidate| std::ptr::eq(candidate, clause))
}

/// Whether `clause` is a bare-name ORDER BY key of `select` or of the clauses `after` it.
fn ordered_name_place(select: &Select, after: &[&OrderBy], clause: &Expression) -> bool {
    let mut bare: &Expression = unannotated(clause);
    while let Expression::Paren(paren) = bare {
        bare = unannotated(&paren.this);
    }
    matches!(bare, Expression::Column(column) if column.table.is_none())
        && select
            .order_by
            .iter()
            .chain(after.iter().copied())
            .flat_map(|order| &order.expressions)
            .any(|ordered| std::ptr::eq(&ordered.this, clause))
}

/// The partition and order keys of `window`.
fn window_keys(window: &NamedWindow) -> impl Iterator<Item = &Expression> {
    let orders = window.spec.order_by.iter().map(|ordered| &ordered.this);
    window.spec.partition_by.iter().chain(orders)
}

/// The keys of `order`.
fn order_keys(order: &OrderBy) -> impl Iterator<Item = &Expression> {
    order.expressions.iter().map(|ordered| &ordered.this)
}

/// Whether a FROM item is LATERAL, reading the items before it.
fn lateral(item: &Expression) -> bool {
    match unannotated(item) {
        Expression::Lateral(_) => true,
        Expression::Subquery(subquery) => subquery.lateral,
        Expression::Alias(alias) => lateral(&alias.this),
        _ => false,
    }
}

/// Every expression of `select` after its projections: filters, keys and orders.
fn clauses(select: &Select) -> Vec<&Expression> {
    let mut read: Vec<&Expression> = Vec::new();
    for join in every_join(select) {
        read.extend(join.on.iter());
        read.extend(join.match_condition.iter());
    }
    read.extend(select.where_clause.iter().map(|clause| &clause.this));
    if let Some(group) = &select.group_by {
        read.extend(group.expressions.iter());
    }
    read.extend(select.having.iter().map(|clause| &clause.this));
    read.extend(select.qualify.iter().map(|clause| &clause.this));
    if let Some(order) = &select.order_by {
        read.extend(order.expressions.iter().map(|ordered| &ordered.this));
    }
    read
}

/// One clause's findings: its own columns checked here, and its queries' at their own levels.
fn clause_findings(
    clause: &Expression,
    level: &Level<'_, '_>,
    context: LineageContext<'_>,
) -> (Vec<NameFinding>, Vec<NameFinding>) {
    let mut own: Vec<NameFinding> = Vec::new();
    let mut nested: Vec<NameFinding> = Vec::new();
    let mut pending: Vec<&Expression> = vec![clause];
    let mut bound: Vec<String> = Vec::new();
    while let Some(node) = pending.pop() {
        match node {
            Expression::Subquery(subquery) if takes_clauses(subquery) => {
                let aliases: Vec<String> =
                    level_outputs(*level, context).map(str::to_owned).collect();
                nested.extend(query_findings(
                    node,
                    (level.scope.ctes, Some(level.scope), &aliases),
                    context,
                ));
            }
            Expression::Select(_)
            | Expression::Union(_)
            | Expression::Intersect(_)
            | Expression::Except(_) => {
                let aliases: Vec<String> =
                    level_outputs(*level, context).map(str::to_owned).collect();
                nested.extend(query_findings(
                    node,
                    (level.scope.ctes, Some(level.scope), &aliases),
                    context,
                ));
            }
            Expression::Dot(dot) => {
                if !matches!(unannotated(&dot.this), Expression::Column(_)) {
                    pending.push(&dot.this);
                }
            }
            Expression::Comprehension(comprehension) => {
                for variable in [
                    Some(&comprehension.expression),
                    comprehension.position.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    if let Expression::Column(column) = variable.as_ref() {
                        bound.push(key(&column.name, context));
                    }
                }
                pending.push(&comprehension.this);
                pending.extend(comprehension.iterator.as_deref());
                pending.extend(comprehension.condition.as_deref());
            }
            Expression::MethodCall(call) => {
                if !matches!(unannotated(&call.this), Expression::Column(_)) {
                    pending.push(&call.this);
                }
                pending.extend(call.args.iter());
            }
            Expression::Column(column) => {
                own.extend(column_finding(column, (level, &bound), context));
            }
            Expression::Columns(columns) => {
                own.extend(columns_pattern_finding(&columns.this, level, context));
                pending.extend(columns.unpack.as_deref());
            }
            Expression::Function(function)
                if function.name.eq_ignore_ascii_case(COLUMNS_FUNCTION)
                    && function.args.len() == 1 =>
            {
                own.extend(columns_pattern_finding(&function.args[0], level, context));
            }
            Expression::Lambda(lambda) => {
                bound.extend(lambda.parameters.iter().map(|name| key(name, context)));
                pending.push(&lambda.body);
            }
            _ => {
                for child in node.children() {
                    if !date_part_argument(node, child, context) {
                        pending.push(child);
                    }
                }
            }
        }
    }
    (own, nested)
}
