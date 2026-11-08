//! Walking a query level by level: each level's clauses read in its scope, inner queries after.

use std::borrow::Cow;
use std::collections::HashMap;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::queries::models::{JoinKind, Select, With};

use crate::lineage::_helpers::checks::coercion::Engine;
use crate::lineage::_helpers::checks::issues::in_scope;
use crate::lineage::_helpers::checks::node_checks::{NodeLevel, node_findings};
use crate::lineage::_helpers::checks::scope_rules::{RuleLevel, select_rules, set_order};
use crate::lineage::_helpers::checks::sets::{Branches, set_findings};
use crate::lineage::_helpers::findings::checks::{
    Level, alias_count_findings, column_finding, columns_pattern_finding, unpivot_findings,
    using_findings,
};
use crate::lineage::_helpers::tracing::expressions::date_part_argument;
use crate::lineage::_helpers::tracing::queries::{
    level_scope, output_key, unannotated, with_ctes_visiting,
};
use crate::lineage::_helpers::tracing::resolution::key;
use crate::lineage::_helpers::tracing::reuse::{Reuse, reusing};
use crate::lineage::constants::{CLICKHOUSE_PROFILE, COLUMNS_FUNCTION, SQLSERVER_PROFILE};
use crate::lineage::models::NameFinding;
use crate::lineage::models::{LineageContext, LineageScope, NamedQuery};

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
            select_findings((query, select), (ctes, outer, aliases), context)
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
    context.profile.name() != CLICKHOUSE_PROFILE
}

/// The CTEs `with` adds to `ctes`, each read body's findings, and under PostgreSQL each one faulted.
fn visited_ctes<'a>(
    (owner, with): (&Expression, Option<&With>),
    (ctes, outer, aliases): (&'a [NamedQuery], Option<&LineageScope<'_>>, &[String]),
    context: LineageContext<'_>,
) -> (Cow<'a, [NamedQuery]>, Vec<NameFinding>) {
    let mut found: Vec<NameFinding> = Vec::new();
    let postgres: bool = Engine::of(context) == Engine::Postgres;
    let unread: Vec<String> = match (Engine::of(context), with) {
        (Engine::DuckDB, Some(with)) => unread_ctes(owner, with, context),
        _ => Vec::new(),
    };
    let reach = with_ctes_visiting(with, (ctes, outer), context, &mut |cte, reach| {
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
    (node, select): (&Expression, &Select),
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
        for checked in in_scope(node) {
            nested.extend(node_findings(checked, level));
        }
    }
    let mut own: Vec<NameFinding> = alias_count_findings(select, &scope, context);
    own.extend(using_findings(select, &scope, context));
    let postgres: bool = Engine::of(context) == Engine::Postgres;
    let sqlserver: bool = context.profile.name() == SQLSERVER_PROFILE;
    for item in from_items(select) {
        let reads: Option<&LineageScope<'_>> = lateral(item).then_some(&scope);
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
    let mut outputs: Vec<String> = aliases.to_vec();
    outputs.extend(aliased_outputs(select, context));
    let level = Level {
        scope: &scope,
        outputs: &outputs,
        connect: select.connect.is_some(),
        filtering_join: select.joins.iter().any(|join| {
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
    let unnamed: [String; 0] = [];
    for projection in &select.expressions {
        let own_name: String = output_key(unannotated(projection), context);
        let mut others: Vec<String> = aliases.to_vec();
        others.extend(
            aliased_outputs(select, context)
                .into_iter()
                .filter(|alias| !context.profile.same_identifier(alias, &own_name)),
        );
        let (found, inner) = clause_findings(
            projection,
            &Level {
                outputs: if postgres || sqlserver {
                    &unnamed
                } else {
                    &others
                },
                ..level
            },
            context,
        );
        own.extend(found);
        nested.extend(inner);
    }
    let own_outputs: Vec<String> = aliased_outputs(select, context);
    for clause in clauses(select) {
        let outputs: &[String] = match (postgres, sqlserver) {
            (true, _) if output_name_place(select, clause) => &own_outputs,
            (_, true) if ordered_name_place(select, clause) => &own_outputs,
            (true, _) | (_, true) => &unnamed,
            _ => level.outputs,
        };
        let (found, inner) = clause_findings(clause, &Level { outputs, ..level }, context);
        own.extend(found);
        nested.extend(inner);
    }
    own.sort_by_key(|finding| finding.span.map(|span| span.start));
    own.extend(nested);
    own
}

/// The FROM items and joined items of `select`, in order.
fn from_items(select: &Select) -> Vec<&Expression> {
    let mut items: Vec<&Expression> = Vec::new();
    if let Some(from) = &select.from {
        items.extend(from.expressions.iter());
    }
    items.extend(select.joins.iter().map(|join| &join.this));
    items
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

/// Where PostgreSQL reads output names: GROUP BY items and ORDER BY keys that are a bare name.
fn output_name_place(select: &Select, clause: &Expression) -> bool {
    select
        .group_by
        .iter()
        .flat_map(|group| &group.expressions)
        .any(|candidate| std::ptr::eq(candidate, clause))
        || ordered_name_place(select, clause)
}

/// Where SQL Server reads output names: ORDER BY keys that are a bare name.
fn ordered_name_place(select: &Select, clause: &Expression) -> bool {
    let mut bare: &Expression = unannotated(clause);
    while let Expression::Paren(paren) = bare {
        bare = unannotated(&paren.this);
    }
    matches!(bare, Expression::Column(column) if column.table.is_none())
        && select
            .order_by
            .iter()
            .flat_map(|order| &order.expressions)
            .any(|ordered| std::ptr::eq(&ordered.this, clause))
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
    for join in &select.joins {
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
            Expression::Select(_)
            | Expression::Union(_)
            | Expression::Intersect(_)
            | Expression::Except(_) => nested.extend(query_findings(
                node,
                (level.scope.ctes, Some(level.scope), level.outputs),
                context,
            )),
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
