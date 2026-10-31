//! `statement_lineage`: the column lineage of a parsed query.

use sqlquack_syntax::Expression;

use crate::lineage::_helpers::queries::resolved_query;
use crate::lineage::models::{LineageContext, OutputLineage, QueryLineage};

/// The lineage of each output column of the parsed query `statement`.
pub fn statement_lineage(statement: &Expression, context: LineageContext<'_>) -> QueryLineage {
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
            })
            .collect(),
        open_star: resolved.open,
        row_sources: resolved.rows.columns.into_iter().collect(),
        rows_resolved: !resolved.rows.lost,
    }
}
