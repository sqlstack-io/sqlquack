//! The findings of one parsed statement, each finding once.

use sqlquack_syntax::Expression;

use crate::lineage::_helpers::findings::levels::query_findings;
use crate::lineage::models::{LineageContext, NameFinding};

/// Every unresolved column reference in `statement`: each level's own, then its inner queries'.
pub(crate) fn statement_findings(
    statement: &Expression,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let mut distinct: Vec<NameFinding> = Vec::new();
    for finding in query_findings(statement, (&[], None, &[]), context) {
        if !distinct.contains(&finding) {
            distinct.push(finding);
        }
    }
    distinct
}
