//! Work the walks would otherwise repeat at every level of a deep tree, done once per analysis.

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use sqlquack_syntax::Expression;
use sqlquack_syntax::ExpressionWalk;
use sqlquack_syntax::expressions::queries::models::With;

use crate::lineage::_helpers::tracing::expressions::is_aggregate;
use crate::lineage::models::{FromItems, NamedQuery, ResolvedQuery};

/// The queries resolved with one CTE list in reach and no enclosing level, by their nodes' addresses.
struct Frame {
    ctes: (usize, usize),
    resolved: FxHashMap<usize, ResolvedQuery>,
    /// The FROM items of each SELECT scoped with the list and no enclosing level.
    items: FxHashMap<usize, FromItems>,
}

/// A node's address, with the address and length of the CTE list it was read with.
type Placed = (usize, (usize, usize));

/// Where a kind of kept value lives: for the analysis by placed node, and per frame by node.
type KeptIn<T> = (
    fn(&mut Kept) -> &mut FxHashMap<Placed, T>,
    fn(&mut Frame) -> &mut FxHashMap<usize, T>,
);

/// What one analysis of a tree keeps, by node address: valid while the analysed tree lives.
#[derive(Default)]
struct Kept {
    frames: Vec<Frame>,
    aggregates: FxHashMap<usize, bool>,
    unpivots: FxHashMap<usize, bool>,
    /// Each WITH's CTE list over a settled list, by the WITH's address and that list.
    reaches: FxHashMap<Placed, Rc<[NamedQuery]>>,
    /// The lists this analysis owns until it ends, so no other list can take their addresses.
    settled_lists: FxHashSet<(usize, usize)>,
    /// Queries resolved with a settled list and no enclosing level, for the whole analysis.
    settled: FxHashMap<Placed, ResolvedQuery>,
    /// FROM items scoped with a settled list and no enclosing level, for the whole analysis.
    settled_items: FxHashMap<Placed, FromItems>,
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
    judged: &dyn Fn(&mut Kept) -> &mut FxHashMap<usize, bool>,
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
                resolved: FxHashMap::default(),
                items: FxHashMap::default(),
            });
        }
        !kept_already
    })
    .unwrap_or(false);
    Reuse { pushed }
}

/// Whether `ctes` is the empty list or one this analysis owns: its address names its contents.
fn settled_list(kept: &Kept, ctes: (usize, usize)) -> bool {
    ctes == (0, 0) || kept.settled_lists.contains(&ctes)
}

/// A WITH's CTE list: borrowed when there is no WITH, kept when the analysis owns it.
pub(crate) enum Reach<'a> {
    Borrowed(&'a [NamedQuery]),
    Owned(Vec<NamedQuery>),
    Kept(Rc<[NamedQuery]>),
}

impl Deref for Reach<'_> {
    type Target = [NamedQuery];

    fn deref(&self) -> &[NamedQuery] {
        match self {
            Reach::Borrowed(ctes) => ctes,
            Reach::Owned(ctes) => ctes,
            Reach::Kept(ctes) => ctes,
        }
    }
}

/// The list `with` gave over `ctes` earlier in this analysis, when `ctes` is settled.
pub(crate) fn kept_reach(with: &With, ctes: &[NamedQuery]) -> Option<Rc<[NamedQuery]>> {
    let key = (std::ptr::from_ref(with) as usize, identity(ctes));
    with_kept(|kept| match settled_list(kept, key.1) {
        true => kept.reaches.get(&key).cloned(),
        false => None,
    })
    .flatten()
}

/// `reach`, the list `with` gives over `ctes`, kept for the analysis once `ctes` is settled.
pub(crate) fn keep_reach<'a>(
    with: &With,
    ctes: &[NamedQuery],
    reach: Vec<NamedQuery>,
) -> Reach<'a> {
    let key = (std::ptr::from_ref(with) as usize, identity(ctes));
    if !with_kept(|kept| settled_list(kept, key.1)).unwrap_or(false) {
        return Reach::Owned(reach);
    }
    if let Some(kept) = kept_reach(with, ctes) {
        return Reach::Kept(kept);
    }
    let shared: Rc<[NamedQuery]> = Rc::from(reach);
    with_kept(|kept| {
        kept.settled_lists.insert(identity(&shared));
        kept.reaches.insert(key, Rc::clone(&shared));
    });
    Reach::Kept(shared)
}

/// `query` resolved with `ctes` and no enclosing level, kept while `ctes` is kept or settled.
pub(crate) fn reused(
    query: &Expression,
    ctes: &[NamedQuery],
    resolve: impl FnOnce() -> ResolvedQuery,
) -> ResolvedQuery {
    let node: usize = std::ptr::from_ref(query) as usize;
    kept_or(
        (node, identity(ctes)),
        (|kept| &mut kept.settled, |frame| &mut frame.resolved),
        resolve,
    )
}

/// The FROM items of the SELECT at `select` scoped with `ctes` and no enclosing level, kept alike.
pub(crate) fn reused_items(
    select: usize,
    ctes: &[NamedQuery],
    scope: impl FnOnce() -> FromItems,
) -> FromItems {
    kept_or(
        (select, identity(ctes)),
        (|kept| &mut kept.settled_items, |frame| &mut frame.items),
        scope,
    )
}

/// The value kept for `(node, ctes)` in the settled map or the innermost frame for `ctes`, else built.
fn kept_or<T: Clone>(
    (node, ctes): Placed,
    (settled, framed): KeptIn<T>,
    build: impl FnOnce() -> T,
) -> T {
    let settles: Option<bool> = with_kept(|kept| settled_list(kept, ctes));
    if settles == Some(true) {
        if let Some(found) = with_kept(|kept| settled(kept).get(&(node, ctes)).cloned()).flatten() {
            return found;
        }
        let built: T = build();
        with_kept(|kept| settled(kept).insert((node, ctes), built.clone()));
        return built;
    }
    let frame = |kept: &Kept| kept.frames.iter().rposition(|frame| frame.ctes == ctes);
    let found: Option<T> = with_kept(|kept| {
        let index: usize = frame(kept)?;
        framed(&mut kept.frames[index]).get(&node).cloned()
    })
    .flatten();
    if let Some(found) = found {
        return found;
    }
    let built: T = build();
    with_kept(|kept| {
        if let Some(index) = frame(kept) {
            framed(&mut kept.frames[index]).insert(node, built.clone());
        }
    });
    built
}
