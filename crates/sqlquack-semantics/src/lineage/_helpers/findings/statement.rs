//! The findings of one parsed statement, each finding once, in polyglot's order.

use std::collections::HashMap;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;

use crate::lineage::_helpers::findings::levels::query_findings;
use crate::lineage::_helpers::tracing::reuse::{Analysis, Reuse, analysing, reusing};
use crate::lineage::constants::NAME_PHASE;
use crate::lineage::models::{LineageContext, NameFinding};

/// Every finding of `statement`: scope rules, names, then types, each phase in polyglot's order.
pub(crate) fn statement_findings(
    statement: &Expression,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let found: Vec<NameFinding> = {
        let _analysis: Analysis = analysing();
        analysed_findings(statement, context)
    };
    ordered_findings(statement, found)
}

/// Every finding of `statement` within the analysis already running, in the order walked.
pub(crate) fn analysed_findings(
    statement: &Expression,
    context: LineageContext<'_>,
) -> Vec<NameFinding> {
    let _reuse: Reuse = reusing(&[]);
    query_findings(statement, (&[], None, &[]), context)
}

/// `found` once each, scope rules and names first, then each other phase in `statement`'s order.
pub(crate) fn ordered_findings(
    statement: &Expression,
    found: Vec<NameFinding>,
) -> Vec<NameFinding> {
    let mut distinct: Vec<NameFinding> = Vec::new();
    for finding in found {
        if !distinct.contains(&finding) {
            distinct.push(finding);
        }
    }
    let order: HashMap<usize, usize> = statement
        .dfs()
        .enumerate()
        .map(|(position, node)| (std::ptr::from_ref(node) as usize, position))
        .collect();
    let mut keyed: Vec<((u8, usize), usize, NameFinding)> = distinct
        .into_iter()
        .enumerate()
        .map(|(emitted, finding)| {
            let place: usize = match finding.phase {
                NAME_PHASE => 0,
                _ => order.get(&finding.node).copied().unwrap_or(usize::MAX),
            };
            ((finding.phase, place), emitted, finding)
        })
        .collect();
    keyed.sort_by_key(|(key, emitted, _)| (*key, *emitted));
    keyed.into_iter().map(|(_, _, finding)| finding).collect()
}
