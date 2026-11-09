//! `statement_lineage_and_findings`: a parsed query's lineage and its name findings, analysed once.

use sqlquack_syntax::Expression;

use crate::lineage::_helpers::findings::statement::{analysed_findings, ordered_findings};
use crate::lineage::_helpers::tracing::reuse::{Analysis, analysing};
use crate::lineage::_helpers::tracing::statement::analysed_lineage;
use crate::lineage::models::{LineageContext, NameFinding, QueryLineage};

/// `statement_lineage` and `statement_name_findings` of `statement`, from one analysis.
pub fn statement_lineage_and_findings(
    statement: &Expression,
    context: LineageContext<'_>,
) -> (QueryLineage, Vec<NameFinding>) {
    let _analysis: Analysis = analysing();
    let found: Vec<NameFinding> = analysed_findings(statement, context);
    let lineage: QueryLineage = analysed_lineage(statement, context);
    (lineage, ordered_findings(statement, found))
}
