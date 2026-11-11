//! The column lineage of one parsed statement, within an analysis already running.

use std::ops::Range;
use std::rc::Rc;

use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::Span;

use crate::lineage::_helpers::tracing::queries::{projects_star, resolved_query};
use crate::lineage::models::{LineageContext, OutputLineage, QueryLineage};

/// The lineage of `statement` within the analysis already running.
pub(crate) fn analysed_lineage(
    statement: &Expression,
    context: LineageContext<'_>,
) -> QueryLineage {
    let resolved = resolved_query(statement, &[], None, context);
    let columns = Rc::unwrap_or_clone(resolved.columns);
    let mut ranges = byte_ranges(
        context.sql,
        columns.iter().map(|column| column.expression).collect(),
    )
    .into_iter();
    QueryLineage {
        outputs: columns
            .into_iter()
            .map(|column| OutputLineage {
                name: column.name,
                key: column.key,
                kind: column.lineage.kind,
                sources: column.lineage.sources.iter().cloned().collect(),
                resolved: column.lineage.resolved,
                data_type: column.lineage.data_type.as_deref().map(str::to_owned),
                nullability: column.lineage.nullability,
                expression_range: ranges.next().flatten(),
            })
            .collect(),
        open_star: resolved.open,
        projects_star: projects_star(statement),
        row_sources: resolved.rows.columns.into_iter().collect(),
        rows_resolved: !resolved.rows.lost,
    }
}

/// `spans`, Unicode character ranges in `sql`, as UTF-8 byte ranges, in one pass over `sql`.
fn byte_ranges(sql: Option<&str>, spans: Vec<Option<Span>>) -> Vec<Option<Range<usize>>> {
    let Some(sql) = sql else {
        return vec![None; spans.len()];
    };
    let mut offsets: Vec<usize> = spans
        .iter()
        .flatten()
        .flat_map(|span| [span.start, span.end])
        .collect();
    offsets.sort_unstable();
    offsets.dedup();
    let mut bytes: Vec<(usize, usize)> = Vec::with_capacity(offsets.len());
    let mut wanted = offsets.iter().copied().peekable();
    for (position, (byte, _)) in sql
        .char_indices()
        .chain(std::iter::once((sql.len(), ' ')))
        .enumerate()
    {
        while wanted.next_if_eq(&position).is_some() {
            bytes.push((position, byte));
        }
        if wanted.peek().is_none() {
            break;
        }
    }
    let byte = |offset: usize| -> Option<usize> {
        let index = bytes.partition_point(|&(position, _)| position < offset);
        bytes
            .get(index)
            .filter(|&&(position, _)| position == offset)
            .map(|&(_, byte)| byte)
    };
    spans
        .into_iter()
        .map(|span| {
            let span = span?;
            let (start, end) = (byte(span.start)?, byte(span.end)?);
            (start < end).then_some(start..end)
        })
        .collect()
}
