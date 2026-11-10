//! The column lineage of one parsed statement, within an analysis already running.

use sqlquack_syntax::Expression;

use crate::lineage::_helpers::tracing::queries::{projects_star, resolved_query};
use crate::lineage::models::{LineageContext, OutputLineage, QueryLineage};

/// The lineage of `statement` within the analysis already running.
pub(crate) fn analysed_lineage(
    statement: &Expression,
    context: LineageContext<'_>,
) -> QueryLineage {
    let resolved = resolved_query(statement, &[], None, context);
    QueryLineage {
        outputs: resolved
            .columns
            .into_iter()
            .map(|column| OutputLineage {
                name: column.name,
                key: column.key,
                kind: column.lineage.kind,
                sources: column.lineage.sources.iter().cloned().collect(),
                resolved: column.lineage.resolved,
                data_type: column.lineage.data_type.as_deref().map(str::to_owned),
                nullability: column.lineage.nullability,
            })
            .collect(),
        open_star: resolved.open,
        projects_star: projects_star(statement),
        row_sources: resolved.rows.columns.into_iter().collect(),
        rows_resolved: !resolved.rows.lost,
    }
}
