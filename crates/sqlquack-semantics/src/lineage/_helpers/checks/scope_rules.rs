//! A SELECT's scope rules: structure, aggregate and window placement, grouping and ordering.

use std::collections::{HashMap, HashSet};

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Column, Identifier, Literal};
use sqlquack_syntax::expressions::queries::models::{OrderBy, Select};

use crate::lineage::_helpers::checks::coercion::Engine;
use crate::lineage::_helpers::checks::issues::{in_scope, issue, rule_location};
use crate::lineage::_helpers::tracing::expressions::is_aggregate;
use crate::lineage::_helpers::tracing::resolution::key;
use crate::lineage::constants::SCOPE_PHASE;
use crate::lineage::models::{LineageContext, LineageScope, NameFinding, ScopeSource};

const COLUMNS_FUNCTION: &str = "columns";
const NOT_GROUPED: &str = "Column must be grouped or used in an aggregate function";
const MISPLACED_AGGREGATE: &str =
    "Aggregate function is not allowed in this clause or inside another aggregate";
const MISPLACED_WINDOW: &str =
    "Window function is not allowed in this clause or inside another aggregate/window function";

fn rule(code: &'static str, message: String, at: &Expression, select: &Expression) -> NameFinding {
    issue((code, SCOPE_PHASE), message, rule_location(at), select)
}

/// A clause the placement rules read: expression, allowed aggregates and windows, and aliases.
type Clause<'s, 'm> = (
    &'s Expression,
    (bool, bool),
    &'m HashMap<String, &'s Expression>,
);

/// What the rules read of a SELECT's level: its scope and the engine.
#[derive(Clone, Copy)]
pub(crate) struct RuleLevel<'s, 'a> {
    pub(crate) scope: &'s LineageScope<'a>,
    pub(crate) context: LineageContext<'a>,
}

impl RuleLevel<'_, '_> {
    fn key(&self, name: &Identifier) -> String {
        key(name, self.context)
    }

    /// Whether some FROM item's columns are not all known.
    fn open(&self) -> bool {
        self.scope
            .sources
            .iter()
            .any(|source| source.query.open || source.query.columns.is_empty())
    }

    fn input_names(&self) -> HashSet<String> {
        let mut names: HashSet<String> = HashSet::new();
        for source in &self.scope.sources {
            names.extend(source.query.columns.iter().map(|column| column.key.clone()));
        }
        names
    }
}

/// The scope-rule findings of the SELECT `node`, in polyglot's order.
pub(crate) fn select_rules(
    (node, select): (&Expression, &Select),
    level: RuleLevel<'_, '_>,
) -> Vec<NameFinding> {
    let mut findings: Vec<NameFinding> = structure_rules((node, select), level);
    findings.extend(order_positions((node, select), level));
    findings.extend(star_modifiers((node, select), level));
    let aliases: HashMap<String, &Expression> = output_aliases(select, level);
    let empty: HashMap<String, &Expression> = HashMap::new();
    let usable: &HashMap<String, &Expression> = match level.open() {
        true => &empty,
        false => &aliases,
    };
    let mut seen: HashMap<String, &Expression> = HashMap::new();
    for projection in &select.expressions {
        findings.extend(placement(projection, (true, true), (&seen, level), node));
        if let Expression::Alias(alias) = projection {
            let alias_key: String = level.key(&alias.alias);
            if aliases.contains_key(&alias_key) && !level.open() {
                seen.insert(alias_key, &alias.this);
            }
        }
    }
    let clauses: Vec<Clause<'_, '_>> = {
        let mut clauses: Vec<Clause<'_, '_>> = Vec::new();
        clauses.extend(
            select
                .prewhere
                .iter()
                .map(|clause| (clause, (false, false), usable)),
        );
        clauses.extend(
            select
                .where_clause
                .iter()
                .map(|clause| (&clause.this, (false, false), usable)),
        );
        clauses.extend(
            select
                .joins
                .iter()
                .filter_map(|join| join.on.as_ref())
                .map(|on| (on, (false, false), &empty)),
        );
        for group in select
            .group_by
            .iter()
            .flat_map(|group| group.expressions.iter())
        {
            clauses.push((
                ordinal_projection(group, select).unwrap_or(group),
                (false, false),
                usable,
            ));
        }
        clauses.extend(
            select
                .having
                .iter()
                .map(|clause| (&clause.this, (true, false), usable)),
        );
        clauses.extend(
            select
                .qualify
                .iter()
                .map(|clause| (&clause.this, (true, true), usable)),
        );
        for order in select
            .order_by
            .iter()
            .filter(|order| !order_by_all(order, level))
        {
            clauses.extend(
                order
                    .expressions
                    .iter()
                    .map(|ordered| (&ordered.this, (true, true), usable)),
            );
        }
        clauses
    };
    for (clause, allowed, aliases) in clauses {
        findings.extend(placement(clause, allowed, (aliases, level), node));
    }
    findings.extend(grouping((node, select), &aliases, level));
    findings.extend(distinct_order((node, select), level));
    findings
}

/// The projection a GROUP BY ordinal names.
fn ordinal_projection<'s>(group: &'s Expression, select: &'s Select) -> Option<&'s Expression> {
    let Expression::Literal(literal) = group else {
        return None;
    };
    let Literal::Number(number) = literal.as_ref() else {
        return None;
    };
    match number.parse::<usize>() {
        Ok(position) => select.expressions.get(position.checked_sub(1)?),
        Err(_) => None,
    }
}

/// Whether `order` is DuckDB or Snowflake ORDER BY ALL.
fn order_by_all(order: &OrderBy, _level: RuleLevel<'_, '_>) -> bool {
    if order.siblings {
        return false;
    }
    let [ordered] = order.expressions.as_slice() else {
        return false;
    };
    let name: &Identifier = match &ordered.this {
        Expression::Column(column) if column.table.is_none() && !column.join_mark => &column.name,
        Expression::Identifier(identifier) => identifier,
        _ => return false,
    };
    !name.quoted && name.name.eq_ignore_ascii_case("ALL")
}

/// The output aliases later clauses may read in place of the expressions they name.
fn output_aliases<'s>(
    select: &'s Select,
    level: RuleLevel<'_, '_>,
) -> HashMap<String, &'s Expression> {
    let inputs: HashSet<String> = level.input_names();
    let mut aliases: HashMap<String, &Expression> = HashMap::new();
    let mut duplicate: HashSet<String> = HashSet::new();
    for projection in &select.expressions {
        let Expression::Alias(alias) = projection else {
            continue;
        };
        let alias_key: String = level.key(&alias.alias);
        let passthrough: bool = matches!(&alias.this, Expression::Column(column)
            if column.table.is_none() && level.key(&column.name) == alias_key);
        if passthrough || inputs.contains(&alias_key) || duplicate.contains(&alias_key) {
            continue;
        }
        if aliases.insert(alias_key.clone(), &alias.this).is_some() {
            aliases.remove(&alias_key);
            duplicate.insert(alias_key);
        }
    }
    aliases
}

/// Duplicate names, unknown named windows, QUALIFY without a window, and bad LIMIT or OFFSET.
fn structure_rules(
    (node, select): (&Expression, &Select),
    level: RuleLevel<'_, '_>,
) -> Vec<NameFinding> {
    let engine: Engine = Engine::of(level.context);
    let mut findings: Vec<NameFinding> = Vec::new();
    if let Some(with) = &select.with {
        let mut names: HashSet<String> = HashSet::new();
        for cte in &with.ctes {
            if engine == Engine::Snowflake
                && !cte.columns.is_empty()
                && output_width(&cte.this).is_some_and(|width| width != cte.columns.len())
            {
                findings.push(rule(
                    "E223",
                    "CTE column alias count does not match its output column count".to_owned(),
                    &cte.this,
                    node,
                ));
            }
            if !names.insert(level.key(&cte.alias)) && engine == Engine::DuckDB {
                findings.push(rule(
                    "E233",
                    format!("Duplicate CTE name '{}'", cte.alias.name),
                    node,
                    node,
                ));
            }
        }
    }
    let mut names: HashSet<String> = HashSet::new();
    for item in from_items(select) {
        if let Some(name) = item_name(item)
            && !names.insert(level.key(name))
        {
            findings.push(rule(
                "E233",
                format!("Duplicate table alias '{}'", name.name),
                item,
                node,
            ));
        }
    }
    let windows: HashSet<String> = select
        .windows
        .iter()
        .flatten()
        .map(|window| level.key(&window.name))
        .collect();
    let mut has_window: bool = false;
    for expression in in_scope(node) {
        if let Expression::WindowFunction(window) = expression {
            has_window = true;
            if let Some(name) = &window.over.window_name
                && !windows.contains(&level.key(name))
            {
                findings.push(rule(
                    "E232",
                    format!("Unknown named window '{}'", name.name),
                    expression,
                    node,
                ));
            }
        }
    }
    if select.qualify.is_some() && !has_window {
        findings.push(rule(
            "E232",
            "QUALIFY requires a window function in the same query scope".to_owned(),
            node,
            node,
        ));
    }
    let bounds = select
        .limit
        .iter()
        .filter(|limit| !limit.percent)
        .map(|limit| &limit.this)
        .chain(select.offset.iter().map(|offset| &offset.this));
    for value in bounds {
        if invalid_bound(value) {
            findings.push(rule(
                "E234",
                "LIMIT/OFFSET must be a non-negative constant numeric expression".to_owned(),
                value,
                node,
            ));
        }
    }
    findings
}

fn from_items(select: &Select) -> Vec<&Expression> {
    let mut items: Vec<&Expression> = Vec::new();
    if let Some(from) = &select.from {
        items.extend(from.expressions.iter());
    }
    items.extend(select.joins.iter().map(|join| &join.this));
    items
}

/// The name a FROM item is known by: its alias, else its table's name.
fn item_name(item: &Expression) -> Option<&Identifier> {
    match item {
        Expression::Table(table) => Some(table.alias.as_ref().unwrap_or(&table.name)),
        Expression::Subquery(subquery) => subquery.alias.as_ref(),
        Expression::Alias(alias) => Some(&alias.alias),
        _ => None,
    }
}

/// Whether a LIMIT or OFFSET is negative, a column, or text that is not a number.
fn invalid_bound(value: &Expression) -> bool {
    match value {
        Expression::Neg(_) => negative_constant(value, false).unwrap_or(false),
        Expression::Column(_) => true,
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::Number(number) => number
                .parse::<f64>()
                .is_ok_and(|number| number < f64::default()),
            Literal::String(text) => text
                .parse::<f64>()
                .map_or(true, |number| number < f64::default()),
            _ => false,
        },
        _ => false,
    }
}

fn negative_constant(value: &Expression, negative: bool) -> Option<bool> {
    match value {
        Expression::Neg(unary) => negative_constant(&unary.this, !negative),
        Expression::Paren(paren) => negative_constant(&paren.this, negative),
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::Number(number) => {
                let number: &str = number.trim();
                let sign: bool = number.starts_with('-') ^ negative;
                let mantissa: &str = number
                    .trim_start_matches(['-', '+'])
                    .split(['e', 'E'])
                    .next()?;
                if !mantissa
                    .chars()
                    .all(|character| character.is_ascii_digit() || character == '.')
                {
                    return None;
                }
                Some(
                    sign && mantissa
                        .chars()
                        .any(|character| matches!(character, '1'..='9')),
                )
            }
            _ => None,
        },
        _ => None,
    }
}

/// Whether a projection calls COLUMNS, whose width only the engine knows.
fn reads_columns(projection: &Expression) -> bool {
    projection.dfs().any(columns_call)
}

fn columns_call(node: &Expression) -> bool {
    matches!(node, Expression::Function(function) if function.name.eq_ignore_ascii_case(COLUMNS_FUNCTION))
}

/// Whether a projection holds a star or a COLUMNS call, which hide how many outputs there are.
fn starred_projection(projection: &Expression) -> bool {
    projection.dfs().any(|node| {
        matches!(node, Expression::Star(_) | Expression::BracedWildcard(_)) || columns_call(node)
    })
}

/// How many columns a query outputs, when its projections show it.
pub(crate) fn output_width(query: &Expression) -> Option<usize> {
    match query {
        Expression::Select(select) if select.expressions.iter().any(reads_columns) => None,
        Expression::Select(select)
            if !select.expressions.iter().any(|projection| {
                matches!(
                    projection,
                    Expression::Star(_) | Expression::BracedWildcard(_)
                )
            }) =>
        {
            Some(select.expressions.len())
        }
        Expression::Values(values) => values.expressions.first().map(|row| row.expressions.len()),
        Expression::Union(set) => output_width(&set.left),
        Expression::Intersect(set) => output_width(&set.left),
        Expression::Except(set) => output_width(&set.left),
        Expression::Subquery(subquery) => output_width(&subquery.this),
        Expression::Alias(alias) => output_width(&alias.this),
        Expression::Paren(paren) => output_width(&paren.this),
        _ => None,
    }
}

/// ORDER BY positions outside the output columns, when no star hides how many there are.
fn order_positions(
    (node, select): (&Expression, &Select),
    level: RuleLevel<'_, '_>,
) -> Vec<NameFinding> {
    let starred: bool = select.expressions.iter().any(starred_projection);
    if starred {
        return Vec::new();
    }
    select
        .order_by
        .iter()
        .filter(|order| !order_by_all(order, level))
        .flat_map(|order| order.expressions.iter())
        .filter(|ordered| {
            matches!(&ordered.this, Expression::Literal(literal)
                if matches!(literal.as_ref(), Literal::Number(number)
                    if number.parse::<usize>().is_ok_and(|position| position == 0 || position > select.expressions.len())))
        })
        .map(|ordered| {
            rule(
                "E201",
                "ORDER BY position is outside the output column range".to_owned(),
                &ordered.this,
                node,
            )
        })
        .collect()
}

/// EXCLUDE, EXCEPT and REPLACE names a closed input does not have.
fn star_modifiers(
    (node, select): (&Expression, &Select),
    level: RuleLevel<'_, '_>,
) -> Vec<NameFinding> {
    if level.open() {
        return Vec::new();
    }
    let inputs: HashSet<String> = level.input_names();
    let mut findings: Vec<NameFinding> = Vec::new();
    for projection in &select.expressions {
        let Expression::Star(star) = projection else {
            continue;
        };
        let names = star
            .except
            .iter()
            .flatten()
            .chain(star.replace.iter().flatten().map(|alias| &alias.alias));
        for name in names {
            if !inputs.contains(&level.key(name)) {
                findings.push(rule(
                    "E201",
                    format!("Star modifier references unknown column '{}'", name.name),
                    projection,
                    node,
                ));
            }
        }
    }
    findings
}

/// One pending node of the placement walk and where it is read.
#[derive(Clone, Copy)]
struct Placed<'e> {
    node: &'e Expression,
    inside_aggregate: bool,
    inside_window: bool,
    window_function: bool,
}

fn query_start(node: &Expression) -> bool {
    matches!(
        node,
        Expression::Select(_)
            | Expression::Subquery(_)
            | Expression::Exists(_)
            | Expression::Union(_)
            | Expression::Intersect(_)
            | Expression::Except(_)
    )
}

/// Aggregates and windows where the clause allows none, or inside another.
fn placement(
    expression: &Expression,
    (aggregates, windows): (bool, bool),
    (aliases, level): (&HashMap<String, &Expression>, RuleLevel<'_, '_>),
    select: &Expression,
) -> Vec<NameFinding> {
    let mut findings: Vec<NameFinding> = Vec::new();
    let mut pending: Vec<Placed<'_>> = vec![Placed {
        node: expression,
        inside_aggregate: false,
        inside_window: false,
        window_function: false,
    }];
    let mut expanded: HashSet<(String, bool, bool)> = HashSet::new();
    while let Some(placed) = pending.pop() {
        let node: &Expression = placed.node;
        if query_start(node) {
            continue;
        }
        if let Expression::Column(column) = node
            && column.table.is_none()
        {
            let name: String = level.key(&column.name);
            if let Some(alias) = aliases.get(&name) {
                if expanded.insert((name, placed.inside_aggregate, placed.inside_window)) {
                    pending.push(Placed {
                        node: alias,
                        window_function: false,
                        ..placed
                    });
                }
                continue;
            }
        }
        if let Expression::WindowFunction(window) = node {
            if !windows || placed.inside_window || placed.inside_aggregate {
                findings.push(rule("E232", MISPLACED_WINDOW.to_owned(), node, select));
                continue;
            }
            pending.push(Placed {
                node: &window.this,
                inside_aggregate: false,
                inside_window: true,
                window_function: true,
            });
            for child in node.children() {
                if !std::ptr::eq(child, &window.this) {
                    pending.push(Placed {
                        node: child,
                        inside_aggregate: false,
                        inside_window: true,
                        window_function: false,
                    });
                }
            }
            continue;
        }
        if !placed.window_function
            && matches!(
                node,
                Expression::RowNumber(_)
                    | Expression::Rank(_)
                    | Expression::DenseRank(_)
                    | Expression::NTile(_)
                    | Expression::Lag(_)
                    | Expression::Lead(_)
                    | Expression::FirstValue(_)
                    | Expression::LastValue(_)
            )
        {
            findings.push(rule(
                "E232",
                "Window function requires an OVER clause".to_owned(),
                node,
                select,
            ));
        }
        let aggregate: bool = is_aggregate(node) && !placed.window_function;
        if aggregate && (!aggregates || placed.inside_aggregate) {
            findings.push(rule("E231", MISPLACED_AGGREGATE.to_owned(), node, select));
            continue;
        }
        let mut children: Vec<&Expression> = node.children();
        children.reverse();
        pending.extend(children.into_iter().map(|child| Placed {
            node: child,
            inside_aggregate: placed.inside_aggregate || aggregate,
            inside_window: placed.inside_window,
            window_function: false,
        }));
    }
    findings
}

/// Whether `expression` holds an aggregate of this level, not counting window functions' own.
fn ordinary_aggregate(expression: &Expression) -> bool {
    let mut pending: Vec<&Expression> = vec![expression];
    while let Some(node) = pending.pop() {
        if query_start(node) {
            continue;
        }
        if let Expression::WindowFunction(window) = node {
            pending.extend(window.this.children());
            pending.extend(
                node.children()
                    .into_iter()
                    .filter(|child| !std::ptr::eq(*child, &window.this)),
            );
            continue;
        }
        if is_aggregate(node) {
            return true;
        }
        pending.extend(node.children());
    }
    false
}

/// The group keys of `select`: GROUP BY expressions, ordinals and aliases resolved, sets opened.
fn group_keys<'s>(
    select: &'s Select,
    aliases: &HashMap<String, &'s Expression>,
    level: RuleLevel<'_, '_>,
) -> Vec<&'s Expression> {
    let mut groups: Vec<&Expression> = Vec::new();
    let mut pending: Vec<&Expression> = select
        .group_by
        .iter()
        .flat_map(|group| group.expressions.iter())
        .collect();
    while let Some(mut node) = pending.pop() {
        if let Some(projection) = ordinal_projection(node, select) {
            node = unalias(projection);
        }
        if let Expression::Column(column) = node
            && column.table.is_none()
            && let Some(alias) = aliases.get(&level.key(&column.name))
        {
            node = alias;
        }
        groups.push(node);
        let opens: bool = matches!(
            node,
            Expression::GroupingSets(_)
                | Expression::Cube(_)
                | Expression::Rollup(_)
                | Expression::Tuple(_)
                | Expression::Paren(_)
        ) || type_marker(node)
            || matches!(node, Expression::Function(function) if !function.quoted
                && matches!(function.name.to_ascii_uppercase().as_str(), "GROUPING SETS" | "CUBE" | "ROLLUP"));
        if opens {
            pending.extend(node.children());
        }
    }
    groups
}

fn type_marker(node: &Expression) -> bool {
    matches!(node, Expression::Annotated(annotated)
        if annotated.type_assertion.is_some() || annotated.type_like.is_some())
}

fn unmarked(node: &Expression) -> &Expression {
    match node {
        Expression::Annotated(annotated) if type_marker(node) => unmarked(&annotated.this),
        _ => node,
    }
}

fn unalias(expression: &Expression) -> &Expression {
    match expression {
        Expression::Alias(alias) => &alias.this,
        _ => expression,
    }
}

/// The relation and name a column reference reads, as grouping compares them.
fn column_key(column: &Column, level: RuleLevel<'_, '_>) -> (Option<String>, String) {
    let name: String = level.key(&column.name);
    let table: Option<String> = column
        .table
        .as_ref()
        .map(|table| level.key(table))
        .or_else(|| {
            let offering: Vec<&ScopeSource> = level
                .scope
                .sources
                .iter()
                .filter(|source| offers(source, &name, level))
                .collect();
            match (offering.as_slice(), level.scope.sources.as_slice()) {
                ([only], _) => only.key.clone(),
                ([], [only]) => only.key.clone(),
                _ => None,
            }
        });
    (table, name)
}

/// Whether `source` has a column named `name`.
fn offers(source: &ScopeSource, name: &str, level: RuleLevel<'_, '_>) -> bool {
    source
        .query
        .columns
        .iter()
        .any(|offered| level.context.profile.same_identifier(&offered.key, name))
}

/// Columns an aggregate query reads that are neither grouped nor aggregated.
fn grouping(
    (node, select): (&Expression, &Select),
    aliases: &HashMap<String, &Expression>,
    level: RuleLevel<'_, '_>,
) -> Vec<NameFinding> {
    let aggregate_query: bool = select.group_by.is_some()
        || select.expressions.iter().any(ordinary_aggregate)
        || select
            .having
            .iter()
            .any(|having| ordinary_aggregate(&having.this))
        || select
            .order_by
            .iter()
            .flat_map(|order| order.expressions.iter())
            .any(|ordered| ordinary_aggregate(&ordered.this));
    let group_all: bool = select
        .group_by
        .as_ref()
        .is_some_and(|group| group.all == Some(true) && group.expressions.is_empty());
    if !aggregate_query || group_all {
        return Vec::new();
    }
    let groups: Vec<&Expression> = group_keys(select, aliases, level);
    let unmarked_groups: Vec<&Expression> = groups.iter().map(|group| unmarked(group)).collect();
    let grouped_columns: HashSet<(Option<String>, String)> = groups
        .iter()
        .filter_map(|group| match group {
            Expression::Column(column) => Some(column_key(column, level)),
            _ => None,
        })
        .collect();
    let mut read: Vec<&Expression> = select.expressions.iter().collect();
    read.extend(select.having.iter().map(|having| &having.this));
    read.extend(select.qualify.iter().map(|qualify| &qualify.this));
    for order in select
        .order_by
        .iter()
        .filter(|order| !order_by_all(order, level))
    {
        read.extend(order.expressions.iter().map(|ordered| &ordered.this));
    }
    let mut findings: Vec<NameFinding> = Vec::new();
    for expression in read {
        let mut pending: Vec<&Expression> = vec![unalias(expression)];
        let mut expanded: HashSet<String> = HashSet::new();
        while let Some(current) = pending.pop() {
            if query_start(current)
                || is_aggregate(current)
                || matches!(current, Expression::WithinGroup(group) if is_aggregate(&group.this))
                || unmarked_groups
                    .iter()
                    .any(|group| **group == *unmarked(current))
            {
                continue;
            }
            if let Expression::Column(column) = current {
                if grouped_columns.contains(&column_key(column, level)) {
                    continue;
                }
                if column.table.is_none() {
                    let name: String = level.key(&column.name);
                    if let Some(alias) = aliases.get(&name) {
                        if expanded.insert(name) {
                            pending.push(alias);
                        }
                        continue;
                    }
                }
                findings.push(rule("E230", NOT_GROUPED.to_owned(), current, node));
                break;
            }
            if let Expression::WindowFunction(window) = current {
                pending.extend(window.this.children());
                pending.extend(
                    current
                        .children()
                        .into_iter()
                        .filter(|child| !std::ptr::eq(*child, &window.this)),
                );
            } else {
                let mut children: Vec<&Expression> = current.children();
                children.reverse();
                pending.extend(children);
            }
        }
    }
    findings
}

/// Snowflake's DISTINCT with ORDER BY a column it does not select.
fn distinct_order(
    (node, select): (&Expression, &Select),
    level: RuleLevel<'_, '_>,
) -> Vec<NameFinding> {
    let ordered = select
        .order_by
        .as_ref()
        .filter(|order| !order_by_all(order, level));
    if !select.distinct
        || ordered.is_none()
        || Engine::of(level.context) != Engine::Snowflake
        || !select
            .expressions
            .iter()
            .all(|projection| matches!(unalias(projection), Expression::Column(_)))
    {
        return Vec::new();
    }
    let mut selected: HashSet<String> = HashSet::new();
    for projection in &select.expressions {
        if let Expression::Alias(alias) = projection {
            selected.insert(level.key(&alias.alias));
        }
        if let Expression::Column(column) = unalias(projection) {
            selected.insert(level.key(&column.name));
        }
    }
    ordered
        .into_iter()
        .flat_map(|order| order.expressions.iter())
        .filter(|ordered| {
            matches!(&ordered.this, Expression::Column(column) if !selected.contains(&level.key(&column.name)))
        })
        .map(|ordered| {
            rule(
                "E201",
                "DISTINCT ORDER BY references a non-selected column".to_owned(),
                &ordered.this,
                node,
            )
        })
        .collect()
}

/// A set operation's ORDER BY column that is not one of its outputs.
pub(crate) fn set_order(query: &Expression, context: LineageContext<'_>) -> Vec<NameFinding> {
    let (order, left) = match query {
        Expression::Union(set) => (set.order_by.as_ref(), &set.left),
        Expression::Intersect(set) => (set.order_by.as_ref(), &set.left),
        Expression::Except(set) => (set.order_by.as_ref(), &set.left),
        _ => return Vec::new(),
    };
    let Some(outputs) = output_names(left, context) else {
        return Vec::new();
    };
    order
        .into_iter()
        .filter(|order| {
            !(order.expressions.len() == 1
                && matches!(&order.expressions[0].this, Expression::Column(column)
                    if column.table.is_none() && !column.name.quoted && column.name.name.eq_ignore_ascii_case("ALL")))
        })
        .flat_map(|order| order.expressions.iter())
        .filter(|ordered| {
            matches!(&ordered.this, Expression::Column(column) if !outputs.contains(&key(&column.name, context)))
        })
        .map(|ordered| {
            issue(
                ("E201", SCOPE_PHASE),
                "Set-operation ORDER BY references a non-output column".to_owned(),
                rule_location(&ordered.this),
                query,
            )
        })
        .collect()
}

/// The output names of a set operation's first branch, when every one is named.
fn output_names(query: &Expression, context: LineageContext<'_>) -> Option<HashSet<String>> {
    match query {
        Expression::Select(select) => select
            .expressions
            .iter()
            .map(|projection| match projection {
                Expression::Alias(alias) => Some(key(&alias.alias, context)),
                Expression::Column(column) => Some(key(&column.name, context)),
                _ => None,
            })
            .collect(),
        Expression::Union(set) => output_names(&set.left, context),
        Expression::Intersect(set) => output_names(&set.left, context),
        Expression::Except(set) => output_names(&set.left, context),
        Expression::Subquery(subquery) => output_names(&subquery.this, context),
        Expression::Paren(paren) => output_names(&paren.this, context),
        _ => None,
    }
}
