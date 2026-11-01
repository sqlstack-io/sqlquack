//! What a column reference, USING key or UNPIVOT operand resolves to, as a finding when nothing.

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::names::models::{Column, Identifier};
use sqlquack_syntax::expressions::queries::models::Select;

use crate::lineage::_helpers::tracing::queries::empty_scope;
use crate::lineage::_helpers::tracing::resolution::key;
use crate::lineage::_helpers::tracing::sources::item_sources;
use crate::lineage::constants::{
    AMBIGUOUS_COLUMN_CODE, CONNECT_BY_LEVEL, UNKNOWN_COLUMN_CODE, UNKNOWN_QUALIFIER_CODE,
};
use crate::lineage::constants::{
    DUCKDB_PROFILE, DUCKDB_ROWID, METADATA_PREFIX, POSITIONAL_PREFIX, SEQUENCE_COLUMNS,
    SNOWFLAKE_PROFILE, SQL_STAR,
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
    };
    let written: String = name.name.to_lowercase();
    if pseudo_column(name, column.table.is_some(), context) {
        return None;
    }
    if let Some(qualifier) = &column.table {
        return qualified_finding((qualifier, &wanted, &written), level, context).map(
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
        let matches: usize = current
            .sources
            .iter()
            .filter(|source| offers(source, &wanted, context))
            .count();
        if matches > 1 && !(level.filtering_join && std::ptr::eq(current, level.scope)) {
            return Some(finding(
                AMBIGUOUS_COLUMN_CODE,
                format!(
                    "Ambiguous unqualified column '{written}' found in {matches} referenced tables"
                ),
            ));
        }
        if matches >= 1 || current.sources.iter().any(unjudged) {
            return None;
        }
        scope = current.outer;
    }
    if level
        .outputs
        .iter()
        .any(|output| context.profile.same_identifier(output, &wanted))
        || (level.scope.sources.is_empty() && level.scope.outer.is_none())
    {
        return None;
    }
    Some(finding(
        UNKNOWN_COLUMN_CODE,
        unknown_message(&written, level.scope),
    ))
}

/// A qualified reference's finding: its qualifier names no FROM item, or a closed one lacks it.
fn qualified_finding(
    (qualifier, wanted, written): (&Identifier, &str, &str),
    level: &Level<'_, '_>,
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
            return (!unjudged(source) && !offers(source, wanted, context)).then(|| {
                (
                    UNKNOWN_COLUMN_CODE,
                    format!(
                        "Unknown column '{written}' in table '{}'",
                        display_name(source, &alias)
                    ),
                    None,
                )
            });
        }
        scope = current.outer;
    }
    if !definitely_no_relation(&alias, level, context) {
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

/// Whether an unknown qualifier names nothing else: every item closed and named, none a struct.
fn definitely_no_relation(alias: &str, level: &Level<'_, '_>, context: LineageContext<'_>) -> bool {
    let mut scope: Option<&LineageScope<'_>> = Some(level.scope);
    while let Some(current) = scope {
        if current.sources.iter().any(|source| {
            unjudged(source) || source.key.is_none() || offers(source, alias, context)
        }) {
            return false;
        }
        scope = current.outer;
    }
    true
}

/// A pseudo-column no relation lists: `METADATA$…`, `$n`, DuckDB's `rowid`, NEXTVAL or CURRVAL.
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
            || (context.profile.name() == DUCKDB_PROFILE
                && name.name.eq_ignore_ascii_case(DUCKDB_ROWID))
            || (qualified && SEQUENCE_COLUMNS.contains(&upper.as_str())))
}

/// Snowflake's CONNECT BY LEVEL, a pseudo-column of a hierarchical query.
fn connect_by_level(name: &Identifier, level: &Level<'_, '_>, context: LineageContext<'_>) -> bool {
    level.connect
        && context.profile.name() == SNOWFLAKE_PROFILE
        && !name.quoted
        && name.name.eq_ignore_ascii_case(CONNECT_BY_LEVEL)
}

/// The words of an unknown column: in the one FROM item, or in none of several.
fn unknown_message(written: &str, scope: &LineageScope<'_>) -> String {
    match scope.sources.as_slice() {
        [only] => format!(
            "Unknown column '{written}' in table '{}'",
            display_name(only, only.key.as_deref().unwrap_or_default())
        ),
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

/// UNPIVOT operands its closed input does not offer.
pub(crate) fn unpivot_findings(
    item: &Expression,
    ctes: &[NamedQuery],
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let mut findings: Vec<NameFinding> = Vec::new();
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
                });
            }
        }
    }
    findings
}

/// Whether `source` offers a column named `wanted`.
fn offers(source: &ScopeSource, wanted: &str, context: LineageContext<'_>) -> bool {
    source
        .query
        .columns
        .iter()
        .any(|offered| context.profile.same_identifier(&offered.key, wanted))
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
