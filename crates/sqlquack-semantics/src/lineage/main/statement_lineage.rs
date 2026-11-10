//! `statement_lineage`: the column lineage of a parsed query.

use sqlquack_syntax::Expression;

use crate::lineage::_helpers::tracing::reuse::{Analysis, analysing};
use crate::lineage::_helpers::tracing::statement::analysed_lineage;
use crate::lineage::models::{LineageContext, QueryLineage};

/// The lineage of each output column of the parsed query `statement`.
pub fn statement_lineage(statement: &Expression, context: LineageContext<'_>) -> QueryLineage {
    let _analysis: Analysis = analysing();
    analysed_lineage(statement, context)
}
