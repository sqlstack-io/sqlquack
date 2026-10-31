//! What a column name in a query level resolves to.

use sqlquack_syntax::expressions::names::models::Identifier;

use crate::lineage::_helpers::lineages::{combined, constant, direct, traced_if, unresolved};
use crate::lineage::constants::POSITIONAL_COLUMN_BASE;
use crate::lineage::models::{
    Lineage, LineageContext, LineageScope, ScopeSource, ScopedColumn, TransformKind,
};

/// The resolved key of an identifier.
pub(crate) fn key(identifier: &Identifier, context: LineageContext<'_>) -> String {
    context
        .profile
        .resolved_identifier(&identifier.name, identifier.quoted)
}

/// The resolved key of a name written unquoted, as schemas and generated names are.
pub(crate) fn plain_key(name: &str, context: LineageContext<'_>) -> String {
    context.profile.resolved_identifier(name, false)
}

/// The lineage of the column `name`, qualified by `table` when written so.
pub(crate) fn column_lineage(
    table: Option<&Identifier>,
    name: &Identifier,
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Lineage {
    let column = key(name, context);
    let qualifier = table.map(|table| key(table, context));
    let found = match &qualifier {
        Some(qualifier) => qualified(qualifier, (&column, name), scope, context),
        None => unqualified((&column, name), scope, context),
    };
    found.unwrap_or_else(|| match scope.outer {
        Some(outer) => column_lineage(table, name, outer, context),
        None => unresolved(),
    })
}

fn qualified(
    qualifier: &str,
    (column, written): (&str, &Identifier),
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Option<Lineage> {
    let source = scope.sources.iter().find(|source| {
        source
            .key
            .as_deref()
            .is_some_and(|key| context.profile.same_identifier(key, qualifier))
    })?;
    if let Some(position) = positional(written) {
        return Some(positional_column(source, position));
    }
    Some(offered(source, (column, written), context).unwrap_or_else(unresolved))
}

/// The 0-based position `$n` names, when an unquoted name is one.
fn positional(written: &Identifier) -> Option<usize> {
    let digits: &str = written.name.strip_prefix('$').filter(|_| !written.quoted)?;
    match digits.parse::<usize>() {
        Ok(number) => number.checked_sub(POSITIONAL_COLUMN_BASE),
        Err(_) => None,
    }
}

/// The column at `position` of `source`, untraced when its columns there are not known.
pub(crate) fn positional_column(source: &ScopeSource, position: usize) -> Lineage {
    match source.query.columns.get(position) {
        Some(column) if !source.query.open => column.lineage.clone(),
        _ => unresolved(),
    }
}

/// An unqualified name, by precedence; a select alias beside open relations is ambiguous.
fn unqualified(
    (column, written): (&str, &Identifier),
    scope: &LineageScope<'_>,
    context: LineageContext<'_>,
) -> Option<Lineage> {
    let same = |name: &str| context.profile.same_identifier(name, column);
    if scope.lambda_names.iter().any(|name| same(name)) {
        return Some(constant());
    }
    if let Some(merged) = scope.merged.iter().find(|merged| same(&merged.key)) {
        return Some(merged.lineage.clone());
    }
    let known = scope
        .sources
        .iter()
        .find_map(|source| known(source, column, context));
    if known.is_some() {
        return known;
    }
    let open: Vec<&ScopeSource> = scope
        .sources
        .iter()
        .filter(|source| source.query.open)
        .collect();
    let alias: Option<&ScopedColumn> = scope.aliases.iter().find(|alias| same(&alias.key));
    match (open.as_slice(), alias) {
        ([], Some(alias)) => Some(alias.lineage.clone()),
        ([only], None) => Some(opened(only, written)),
        ([], None) => None,
        _ => Some(unresolved()),
    }
}

/// The lineage of `column` read from `source`, when the source offers it.
fn offered(
    source: &ScopeSource,
    (column, written): (&str, &Identifier),
    context: LineageContext<'_>,
) -> Option<Lineage> {
    known(source, column, context).or_else(|| source.query.open.then(|| opened(source, written)))
}

fn known(source: &ScopeSource, column: &str, context: LineageContext<'_>) -> Option<Lineage> {
    source
        .query
        .columns
        .iter()
        .find(|offered: &&ScopedColumn| context.profile.same_identifier(&offered.key, column))
        .map(|offered| offered.lineage.clone())
}

/// A column read from an open source: its one relation's, with what that relation also reads.
fn opened(source: &ScopeSource, written: &Identifier) -> Lineage {
    let Some(relation) = &source.query.open_relation else {
        return unresolved();
    };
    let column: Lineage = direct(relation, &written.name, written.quoted);
    match &source.query.open_reads {
        Some(reads) if reads.sources.is_empty() => traced_if(column, reads.resolved),
        Some(reads) => combined(vec![column, reads.clone()], TransformKind::Expression),
        None => column,
    }
}
