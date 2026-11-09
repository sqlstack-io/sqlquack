//! What a column reference, USING key or UNPIVOT operand resolves to, as a finding when nothing.

use regex::Regex;
use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Column, Identifier, Literal};
use sqlquack_syntax::expressions::queries::models::Select;

use crate::dialects::main::dialect_rules::dialect_rules;
use crate::lineage::_helpers::checks::scope_rules::output_width;
use crate::lineage::_helpers::tracing::queries::empty_scope;
use crate::lineage::_helpers::tracing::resolution::key;
use crate::lineage::_helpers::tracing::reuse::holds_unpivot;
use crate::lineage::_helpers::tracing::sources::item_sources;
use crate::lineage::constants::{
    ALIAS_COUNT_CODE, AMBIGUOUS_COLUMN_CODE, COLUMNS_NO_MATCH, CONNECT_BY_LEVEL, NAME_PHASE,
    UNKNOWN_COLUMN_CODE, UNKNOWN_QUALIFIER_CODE,
};
use crate::lineage::constants::{
    METADATA_PREFIX, POSITIONAL_PREFIX, SEQUENCE_COLUMNS, SINGLE_JOIN_SOURCES, SQL_STAR,
};
use crate::lineage::models::NameFinding;
use crate::lineage::models::{LineageContext, LineageScope, NamedQuery, ScopeSource};

/// The level a clause is read in: its scope, the aliases it may read, CONNECT BY and filter joins.
#[derive(Clone, Copy)]
pub(crate) struct Level<'s, 'a> {
    pub(crate) scope: &'s LineageScope<'a>,
    pub(crate) outputs: &'s [String],
    pub(crate) connect: bool,
    /// Whether a SEMI or ANTI join filters the level, its right side hidden from the outputs.
    pub(crate) filtering_join: bool,
}

/// What a column reference resolves to, as a finding when it resolves to nothing.
pub(crate) fn column_finding(
    column: &Column,
    (level, bound): (&Level<'_, '_>, &[String]),
    context: LineageContext<'_>,
) -> Option<NameFinding> {
    let name: &Identifier = &column.name;
    if name.name.is_empty() || name.name == SQL_STAR {
        return None;
    }
    let wanted: String = key(name, context);
    let span = name.span.or(column.span);
    let finding = |code: &'static str, message: String| NameFinding {
        code,
        message,
        span,
        written: name.name.clone(),
        phase: NAME_PHASE,
        ..NameFinding::default()
    };
    let written: String = match dialect_rules(context.profile).findings_keep_written_case() {
        true => name.name.clone(),
        false => name.name.to_lowercase(),
    };
    if pseudo_column(name, column.table.is_some(), context) {
        return None;
    }
    if let Some(qualifier) = &column.table {
        let field_of_bound: bool = bound.iter().any(|name| {
            context
                .profile
                .same_identifier(name, &key(qualifier, context))
        });
        if field_of_bound {
            return None;
        }
        return qualified_finding((qualifier, &wanted, &written), (level, bound), context).map(
            |(code, message, at)| NameFinding {
                span: at.or(span),
                ..finding(code, message)
            },
        );
    }
    if bound
        .iter()
        .any(|bound| context.profile.same_identifier(bound, &wanted))
        || connect_by_level(name, level, context)
    {
        return None;
    }
    let mut scope: Option<&LineageScope<'_>> = Some(level.scope);
    while let Some(current) = scope {
        if current
            .merged
            .iter()
            .any(|merged| context.profile.same_identifier(&merged.key, &wanted))
        {
            return None;
        }
        let matches: usize = offering_count(current, &wanted, context);
        if matches > 1
            && !(level.filtering_join && std::ptr::eq(current, level.scope))
            && !single_join_prefers_left(current, context)
        {
            return Some(finding(
                AMBIGUOUS_COLUMN_CODE,
                format!(
                    "Ambiguous unqualified column '{written}' found in {matches} referenced tables"
                ),
            ));
        }
        if matches >= 1
            || current
                .sources
                .iter()
                .any(|source| unjudged(source) || source.query.faulted)
        {
            return None;
        }
        scope = current.outer;
    }
    if level
        .outputs
        .iter()
        .any(|output| context.profile.same_identifier(output, &wanted))
        || (level.scope.sources.is_empty() && level.scope.outer.is_none())
        || keyword_function(column, level.scope, context)
    {
        return None;
    }
    Some(finding(
        UNKNOWN_COLUMN_CODE,
        unknown_message(&written, level.scope, context),
    ))
}

/// A bare DuckDB value keyword no FROM item in scope offers, which DuckDB reads as its function.
pub(crate) fn keyword_function(
    column: &Column,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> bool {
    column.table.is_none() && value_keyword(&column.name, scope, context)
}

/// Whether an unqualified name is a dialect value keyword that no FROM item in reach offers.
pub(crate) fn value_keyword(
    name: &Identifier,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> bool {
    if name.quoted
        || !dialect_rules(context.profile)
            .value_keywords()
            .iter()
            .any(|keyword| name.name.eq_ignore_ascii_case(keyword))
    {
        return false;
    }
    let wanted: String = key(name, context);
    let mut current: Option<&LineageScope<'_>> = Some(scope);
    while let Some(level) = current {
        if offering_count(level, &wanted, context) > 0
            || level
                .merged
                .iter()
                .any(|merged| context.profile.same_identifier(&merged.key, &wanted))
        {
            return false;
        }
        current = level.outer;
    }
    true
}

/// Whether the dialect reads a name both sides of the one join offer from the left side.
fn single_join_prefers_left(scope: &LineageScope<'_>, context: LineageContext<'_>) -> bool {
    dialect_rules(context.profile).single_join_prefers_left()
        && scope.sources.len() == SINGLE_JOIN_SOURCES
}

/// A qualified reference's finding: its qualifier names no FROM item, or a closed one lacks it.
fn qualified_finding(
    (qualifier, wanted, written): (&Identifier, &str, &str),
    (level, bound): (&Level<'_, '_>, &[String]),
    context: LineageContext<'_>,
) -> Option<(
    &'static str,
    String,
    Option<sqlquack_syntax::tokens::models::Span>,
)> {
    let alias: String = key(qualifier, context);
    let mut scope: Option<&LineageScope<'_>> = Some(level.scope);
    while let Some(current) = scope {
        if let Some(source) = source_named(current, &alias, context) {
            let by_alias: bool =
                dialect_rules(context.profile).messages_name_unstored_items_by_alias();
            let table: String = match by_alias && source.query.stored_relation.is_none() {
                true => qualifier.name.clone(),
                false => display_name(source, &alias),
            };
            return (!unjudged(source)
                && !source.query.faulted
                && !offers(source, wanted, context))
            .then(|| {
                (
                    UNKNOWN_COLUMN_CODE,
                    format!("Unknown column '{written}' in table '{table}'"),
                    None,
                )
            });
        }
        scope = current.outer;
    }
    let no_relation: bool = match dialect_rules(context.profile).qualifiers_reach_every_name() {
        true => clickhouse_no_relation(&alias, (level, bound), context),
        false => definitely_no_relation(&alias, level, context),
    };
    if !no_relation {
        return None;
    }
    Some((
        UNKNOWN_QUALIFIER_CODE,
        format!(
            "Unknown table or alias '{}' referenced by column '{written}'",
            qualifier.name
        ),
        qualifier.span,
    ))
}

/// Whether a qualifier names nothing: where it names items, only item names and struct columns.
fn definitely_no_relation(alias: &str, level: &Level<'_, '_>, context: LineageContext<'_>) -> bool {
    let by_name: bool = dialect_rules(context.profile).qualifiers_name_items_by_name();
    let mut scope: Option<&LineageScope<'_>> = Some(level.scope);
    while let Some(current) = scope {
        if current.sources.iter().any(|source| {
            let unnamed: bool = source.key.is_none() && !source.query.derived;
            offers(source, alias, context)
                || match by_name {
                    true => source
                        .query
                        .function_name
                        .as_deref()
                        .map_or(unnamed, |name| context.profile.same_identifier(name, alias)),
                    false => unjudged(source) || unnamed,
                }
        }) {
            return false;
        }
        scope = current.outer;
    }
    true
}

/// Whether a ClickHouse qualifier names nothing in reach; wrong for an open relation's subcolumn reads.
fn clickhouse_no_relation(
    alias: &str,
    (level, bound): (&Level<'_, '_>, &[String]),
    context: LineageContext<'_>,
) -> bool {
    let named = |name: &str| context.profile.same_identifier(name, alias);
    if bound.iter().chain(level.outputs).any(|name| named(name)) {
        return false;
    }
    let mut scope: Option<&LineageScope<'_>> = Some(level.scope);
    while let Some(current) = scope {
        let in_scope = current
            .ctes
            .iter()
            .map(|cte| cte.key.as_str())
            .chain(current.aliases.iter().map(|alias| alias.key.as_str()))
            .chain(current.lambda_names.iter().map(String::as_str));
        if in_scope.into_iter().any(named)
            || current
                .sources
                .iter()
                .any(|source| clickhouse_source_names(source, alias, context))
        {
            return false;
        }
        scope = current.outer;
    }
    true
}

/// Whether a FROM item may be what a ClickHouse qualifier names: its table, or a known column it begins.
fn clickhouse_source_names(source: &ScopeSource, alias: &str, context: LineageContext<'_>) -> bool {
    let dotted: String = format!("{alias}.");
    let table: Option<&str> = source
        .query
        .written_name
        .as_deref()
        .and_then(|written| written.rsplit('.').next());
    (source.key.is_none() && !source.query.derived)
        || table.is_some_and(|table| table.eq_ignore_ascii_case(alias))
        || source.query.columns.iter().any(|offered| {
            context.profile.same_identifier(&offered.key, alias) || offered.key.starts_with(&dotted)
        })
}

/// A pseudo-column no relation lists: `METADATA$…`, `$n`, the dialect's own, NEXTVAL or CURRVAL.
fn pseudo_column(name: &Identifier, qualified: bool, context: LineageContext<'_>) -> bool {
    let upper: String = name.name.to_uppercase();
    !name.quoted
        && (upper.starts_with(METADATA_PREFIX)
            || name
                .name
                .strip_prefix(POSITIONAL_PREFIX)
                .is_some_and(|digits| {
                    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
                })
            || dialect_rules(context.profile)
                .pseudo_columns()
                .iter()
                .any(|pseudo| name.name.eq_ignore_ascii_case(pseudo))
            || (qualified && SEQUENCE_COLUMNS.contains(&upper.as_str())))
}

/// CONNECT BY LEVEL, a pseudo-column of a hierarchical query.
fn connect_by_level(name: &Identifier, level: &Level<'_, '_>, context: LineageContext<'_>) -> bool {
    level.connect
        && dialect_rules(context.profile).has_connect_by_level()
        && !name.quoted
        && name.name.eq_ignore_ascii_case(CONNECT_BY_LEVEL)
}

/// A COLUMNS pattern that matches no column of FROM items that list all of theirs.
pub(crate) fn columns_pattern_finding(
    pattern: &Expression,
    level: &Level<'_, '_>,
    context: LineageContext<'_>,
) -> Option<NameFinding> {
    let Expression::Literal(literal) = pattern else {
        return None;
    };
    let (Literal::String(pattern), true) = (
        literal.as_ref(),
        dialect_rules(context.profile).checks_columns_patterns(),
    ) else {
        return None;
    };
    let pattern: Regex = match Regex::new(pattern) {
        Ok(pattern) => pattern,
        Err(_) => return None,
    };
    let sources = &level.scope.sources;
    (!sources.is_empty()
        && !sources.iter().any(unjudged)
        && !sources
            .iter()
            .flat_map(|source| source.query.columns.iter())
            .any(|column| pattern.is_match(&column.name)))
    .then(|| NameFinding {
        code: UNKNOWN_COLUMN_CODE,
        message: COLUMNS_NO_MATCH.to_owned(),
        phase: NAME_PHASE,
        ..NameFinding::default()
    })
}

/// The words of an unknown column: in the one named FROM item, or in none of the items.
fn unknown_message(written: &str, scope: &LineageScope<'_>, context: LineageContext<'_>) -> String {
    match scope.sources.as_slice() {
        [only] if only.key.is_some() || only.query.written_name.is_some() => {
            let alias: &str = only.key.as_deref().unwrap_or_default();
            let by_alias: bool = dialect_rules(context.profile)
                .messages_name_unstored_items_by_alias()
                && only.query.stored_relation.is_none()
                && !alias.is_empty();
            let table: String = match by_alias {
                true => alias.to_owned(),
                false => display_name(only, alias),
            };
            format!("Unknown column '{written}' in table '{table}'")
        }
        _ => format!("Unknown column '{written}' (not found in any referenced table)"),
    }
}

/// A USING key a closed side of its join lacks.
pub(crate) fn using_findings(
    select: &Select,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let at = select.expressions.first().and_then(Expression::source_span);
    let mut findings: Vec<NameFinding> = Vec::new();
    let leading: usize = select.from.iter().map(|from| from.expressions.len()).sum();
    for (position, join) in select.joins.iter().enumerate() {
        let before: usize = leading + position;
        let (left, right) = scope.sources.split_at(before.min(scope.sources.len()));
        let right: &[ScopeSource] = &right[..right.len().min(1)];
        for name in &join.using {
            let wanted: String = key(name, context);
            if lacks(left, &wanted, context) || lacks(right, &wanted, context) {
                findings.push(NameFinding {
                    code: UNKNOWN_COLUMN_CODE,
                    message: format!("JOIN USING column '{}' must exist on both sides", name.name),
                    span: at,
                    written: name.name.clone(),
                    phase: NAME_PHASE,
                    ..NameFinding::default()
                });
            }
        }
    }
    findings
}

/// Whether a closed side of a join offers no column named `wanted`.
fn lacks(side: &[ScopeSource], wanted: &str, context: LineageContext<'_>) -> bool {
    !side.iter().any(unjudged) && !side.iter().any(|source| offers(source, wanted, context))
}

/// UNPIVOT operands its closed input does not offer; an item holding no UNPIVOT is not walked.
pub(crate) fn unpivot_findings(
    item: &Expression,
    ctes: &[NamedQuery],
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let mut findings: Vec<NameFinding> = Vec::new();
    if !holds_unpivot(item) {
        return findings;
    }
    for node in item.dfs() {
        let Expression::Unpivot(unpivot) = node else {
            continue;
        };
        let input: Vec<ScopeSource> =
            item_sources(&unpivot.this, &empty_scope(ctes, None), context);
        if input.iter().any(unjudged) {
            continue;
        }
        for operand in unpivot.columns.iter().flat_map(|operand| operand.dfs()) {
            let Expression::Column(column) = operand else {
                continue;
            };
            let wanted: String = key(&column.name, context);
            if !input.iter().any(|source| offers(source, &wanted, context)) {
                findings.push(NameFinding {
                    code: UNKNOWN_COLUMN_CODE,
                    message: format!("Unknown UNPIVOT input column '{}'", column.name.name),
                    span: column.span.or(column.name.span),
                    written: column.name.name.clone(),
                    phase: NAME_PHASE,
                    ..NameFinding::default()
                });
            }
        }
    }
    findings
}

/// Whether `source` offers a column named `wanted`; a stored table also its system columns.
fn offers(source: &ScopeSource, wanted: &str, context: LineageContext<'_>) -> bool {
    source
        .query
        .columns
        .iter()
        .any(|offered| context.profile.same_identifier(&offered.key, wanted))
        || (source.query.stored_relation.is_some()
            && dialect_rules(context.profile)
                .system_columns()
                .contains(&wanted))
}

/// How many FROM items offer `wanted`, two under one name counted once.
fn offering_count(scope: &LineageScope<'_>, wanted: &str, context: LineageContext<'_>) -> usize {
    let mut names: Vec<&str> = Vec::new();
    let mut keyless: usize = 0;
    for source in scope
        .sources
        .iter()
        .filter(|source| offers(source, wanted, context))
    {
        match source.key.as_deref() {
            Some(name) if !names.contains(&name) => names.push(name),
            Some(_) => {}
            None => keyless += 1,
        }
    }
    names.len() + keyless
}

/// Whether a FROM item's names are not judged: open, VALUES, or with an output of unknown name.
fn unjudged(source: &ScopeSource) -> bool {
    source.query.open
        || source.query.from_values
        || source
            .query
            .columns
            .iter()
            .any(|column| column.name.is_empty())
}

fn source_named<'s>(
    scope: &'s LineageScope<'_>,
    wanted: &str,
    context: LineageContext<'_>,
) -> Option<&'s ScopeSource> {
    scope.sources.iter().find(|source| {
        source
            .key
            .as_deref()
            .is_some_and(|key| context.profile.same_identifier(key, wanted))
    })
}

/// How a finding names a FROM item: the table it names as written, else its alias, in lower case.
fn display_name(source: &ScopeSource, alias: &str) -> String {
    source
        .query
        .written_name
        .clone()
        .unwrap_or_else(|| alias.to_lowercase())
}

/// FROM items with more column aliases than their relation has columns, as polyglot words it.
pub(crate) fn alias_count_findings(
    select: &Select,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let items = select
        .from
        .iter()
        .flat_map(|from| from.expressions.iter())
        .chain(select.joins.iter().map(|join| &join.this));
    let mut findings: Vec<NameFinding> = Vec::new();
    for item in items {
        let (relation, aliases, width) = match item {
            Expression::Subquery(subquery) => (
                "Derived-table",
                &subquery.column_aliases,
                output_width(&subquery.this),
            ),
            Expression::Alias(alias) => (
                "Derived-table",
                &alias.column_aliases,
                output_width(&alias.this),
            ),
            Expression::Table(table) => {
                let name: String = key(table.alias.as_ref().unwrap_or(&table.name), context);
                let width: Option<usize> = scope
                    .sources
                    .iter()
                    .find(|source| source.key.as_deref() == Some(name.as_str()))
                    .filter(|source| !source.query.open && !source.query.columns.is_empty())
                    .map(|source| source.query.columns.len());
                ("Table", &table.column_aliases, width)
            }
            _ => continue,
        };
        if !aliases.is_empty() && width.is_some_and(|width| aliases.len() > width) {
            findings.push(NameFinding {
                code: ALIAS_COUNT_CODE,
                message: format!("{relation} column alias count exceeds its output column count"),
                span: aliases.last().and_then(|alias| alias.span),
                phase: NAME_PHASE,
                ..NameFinding::default()
            });
        }
    }
    findings
}
