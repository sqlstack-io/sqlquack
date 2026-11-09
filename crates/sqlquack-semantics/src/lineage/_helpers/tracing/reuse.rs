//! Work the walks would otherwise repeat at every level of a deep tree, done once per analysis.

use std::cell::RefCell;
use std::collections::HashMap;

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;

use crate::lineage::_helpers::tracing::expressions::is_aggregate;
use crate::lineage::models::{NamedQuery, ResolvedQuery};

/// The queries resolved with one CTE list in reach and no enclosing level, by their nodes' addresses.
struct Frame {
    ctes: (usize, usize),
    resolved: HashMap<usize, ResolvedQuery>,
}

/// What one analysis of a tree keeps, by node address: valid while the analysed tree lives.
#[derive(Default)]
struct Kept {
    frames: Vec<Frame>,
    aggregates: HashMap<usize, bool>,
    unpivots: HashMap<usize, bool>,
}

thread_local! {
    /// The analyses running on this thread, innermost last; each keeps only its own tree's work.
    static ANALYSES: RefCell<Vec<Kept>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
thread_local! {
    /// How many queries the walks resolved and expressions they traced, rather than reused.
    static WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Counts one query resolved or one expression traced, for tests that bound the work.
pub(crate) fn worked() {
    #[cfg(test)]
    WORK.with(|work| work.set(work.get() + 1));
}

/// The queries resolved and expressions traced on this thread since the last call.
#[cfg(test)]
pub(crate) fn work_done() -> usize {
    WORK.with(|work| work.replace(0))
}

/// The innermost analysis's kept work, when an analysis runs.
fn with_kept<R>(read: impl FnOnce(&mut Kept) -> R) -> Option<R> {
    ANALYSES.with(|analyses| analyses.borrow_mut().last_mut().map(read))
}

/// One analysis of a tree that outlives it; it keeps nothing for an analysis around it.
pub(crate) struct Analysis;

impl Drop for Analysis {
    fn drop(&mut self) {
        ANALYSES.with(|analyses| analyses.borrow_mut().pop());
    }
}

/// Starts an analysis of one tree, which must outlive the returned guard.
pub(crate) fn analysing() -> Analysis {
    ANALYSES.with(|analyses| analyses.borrow_mut().push(Kept::default()));
    Analysis
}

/// Whether `node` or any node under it is an aggregate, each node judged once per analysis.
pub(crate) fn holds_aggregate(node: &Expression) -> bool {
    holds(node, &|kept| &mut kept.aggregates, &is_aggregate)
}

/// Whether `node` or any node under it is an UNPIVOT, each node judged once per analysis.
pub(crate) fn holds_unpivot(node: &Expression) -> bool {
    holds(node, &|kept| &mut kept.unpivots, &|node| {
        matches!(node, Expression::Unpivot(_))
    })
}

/// Whether `node` or any node under it is `wanted`, kept by node address in `judged`.
fn holds(
    node: &Expression,
    judged: &dyn Fn(&mut Kept) -> &mut HashMap<usize, bool>,
    wanted: &dyn Fn(&Expression) -> bool,
) -> bool {
    let address: usize = std::ptr::from_ref(node) as usize;
    match with_kept(|kept| judged(kept).get(&address).copied()) {
        None => node.dfs().any(wanted),
        Some(Some(holds_it)) => holds_it,
        Some(None) => {
            let holds_it: bool = crate::lineage::_helpers::tracing::stack::deeper(|| {
                wanted(node)
                    || node
                        .children()
                        .into_iter()
                        .any(|child| holds(child, judged, wanted))
            });
            with_kept(|kept| judged(kept).insert(address, holds_it));
            holds_it
        }
    }
}

/// Keeps queries resolved with a CTE list until it drops; declare it after the list it borrows.
pub(crate) struct Reuse {
    pushed: bool,
}

impl Drop for Reuse {
    fn drop(&mut self) {
        if self.pushed {
            with_kept(|kept| kept.frames.pop());
        }
    }
}

/// Every empty list reads alike; a non-empty one is told by its live buffer and length.
fn identity(ctes: &[NamedQuery]) -> (usize, usize) {
    match ctes.is_empty() {
        true => (0, 0),
        false => (ctes.as_ptr() as usize, ctes.len()),
    }
}

/// Starts keeping queries resolved with `ctes`, unless the innermost list kept is already it.
pub(crate) fn reusing(ctes: &[NamedQuery]) -> Reuse {
    let ctes: (usize, usize) = identity(ctes);
    let pushed: bool = with_kept(|kept| {
        let kept_already: bool = kept.frames.last().is_some_and(|frame| frame.ctes == ctes);
        if !kept_already {
            kept.frames.push(Frame {
                ctes,
                resolved: HashMap::new(),
            });
        }
        !kept_already
    })
    .unwrap_or(false);
    Reuse { pushed }
}

/// `query` resolved with `ctes` and no enclosing level: kept from earlier when a walk keeps `ctes`.
pub(crate) fn reused(
    query: &Expression,
    ctes: &[NamedQuery],
    resolve: impl FnOnce() -> ResolvedQuery,
) -> ResolvedQuery {
    let node: usize = std::ptr::from_ref(query) as usize;
    let ctes: (usize, usize) = identity(ctes);
    let frame = |kept: &Kept| kept.frames.iter().rposition(|frame| frame.ctes == ctes);
    let found: Option<ResolvedQuery> = with_kept(|kept| {
        let index: usize = frame(kept)?;
        kept.frames[index].resolved.get(&node).cloned()
    })
    .flatten();
    if let Some(found) = found {
        return found;
    }
    let resolved: ResolvedQuery = resolve();
    with_kept(|kept| {
        if let Some(index) = frame(kept) {
            kept.frames[index].resolved.insert(node, resolved.clone());
        }
    });
    resolved
}
