//! `statement_name_findings`: the name findings of a parsed statement.

use sqlquack_syntax::Expression;

use crate::lineage::_helpers::findings::statement::statement_findings;
use crate::lineage::models::{LineageContext, NameFinding};

/// The findings of the parsed query `statement`, in the order `query_name_findings` gives them.
pub fn statement_name_findings(
    statement: &Expression,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    statement_findings(statement, context)
}
