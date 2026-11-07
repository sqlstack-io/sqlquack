//! Where a node of the converted tree is in the source: its own range, or the ranges inside it.

use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, HashMap};

use serde_json::Value;
use sqlquack_core::failures::constants::{RAISE_ID_MARKER, TAG_END};
use sqlquack_core::parsing::models::{SourceLines, SourceLocation};
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::raises::main::identify_raises::identify_raises;

use crate::conversion::constants::{SPAN_END, SPAN_FIELD, SPAN_START};

/// The source ranges of the nodes of one converted tree, indexed when first asked for.
#[derive(Debug)]
pub(crate) struct Locator<'a> {
    tree: &'a Value,
    positioned: &'a Value,
    lines: &'a SourceLines,
    ranges: OnceCell<HashMap<usize, (usize, usize)>>,
    adopted: RefCell<HashMap<usize, usize>>,
    raises: RefCell<BTreeMap<String, Vec<SourceLocation>>>,
}

/// Keeps a built node's copies of tree nodes located as their originals until dropped.
#[derive(Debug)]
#[must_use = "copies are located only while the adoption is held"]
pub(crate) struct Adoption<'l, 'a> {
    locator: &'l Locator<'a>,
    copies: Vec<usize>,
}

impl Drop for Adoption<'_, '_> {
    fn drop(&mut self) {
        let mut adopted = self.locator.adopted.borrow_mut();
        for copy in &self.copies {
            adopted.remove(copy);
        }
    }
}

impl<'a> Locator<'a> {
    /// Locates the nodes of `tree`, the `positioned` tree with its positions stripped.
    pub(crate) fn new(tree: &'a Value, positioned: &'a Value, lines: &'a SourceLines) -> Self {
        Self {
            tree,
            positioned,
            lines,
            ranges: OnceCell::new(),
            adopted: RefCell::new(HashMap::new()),
            raises: RefCell::new(BTreeMap::new()),
        }
    }

    /// The whole source text the statement came from.
    pub(crate) fn source(&self) -> String {
        self.lines.text(&self.lines.whole())
    }

    /// The statement tree being converted, positions stripped.
    pub(crate) fn tree(&self) -> &'a Value {
        self.tree
    }

    /// `sql`, converted from `node`, with its unidentified raises identified and located at `node`.
    pub(crate) fn identified(&self, sql: &str, node: &Value) -> String {
        let (sql, ids) = identify_raises(sql);
        let Some(location) = ids.first().and_then(|_| self.locate(node)) else {
            return sql;
        };
        let mut raises = self.raises.borrow_mut();
        for id in ids {
            let locations = raises.entry(id).or_default();
            if !locations.contains(&location) {
                locations.push(location);
            }
        }
        sql
    }

    /// The output `sql` of `node` identified, and where each raise ID it holds comes from.
    pub(crate) fn finished(
        &self,
        sql: &str,
        node: &Value,
    ) -> (String, BTreeMap<String, Vec<SourceLocation>>) {
        let sql = self.identified(sql, node);
        let mut raises = self.raises.borrow().clone();
        raises.retain(|id, _| sql.contains(&format!("{RAISE_ID_MARKER}{id}{TAG_END}")));
        (sql, raises)
    }

    /// Locates the copies of `originals` inside `built`, a node conversion made from them.
    pub(crate) fn adopt<'l>(&'l self, built: &Value, originals: &[Value]) -> Adoption<'l, 'a> {
        let copies = copies_inside(built, originals, ADOPTION_DEPTH);
        self.adopted.borrow_mut().extend(copies.iter().copied());
        Adoption {
            locator: self,
            copies: copies.into_iter().map(|(copy, _)| copy).collect(),
        }
    }

    /// Where `node`, a node of the converted tree, is; `None` for a node built during conversion.
    pub(crate) fn locate(&self, node: &Value) -> Option<SourceLocation> {
        let ranges = self
            .ranges
            .get_or_init(|| node_ranges(self.tree, self.positioned));
        let original = self.adopted.borrow().get(&address(node)).copied();
        let (start, end) = ranges.get(&original.unwrap_or(address(node)))?;
        Some(self.lines.location(*start, *end))
    }

    /// The source text of `node`, a node of the converted tree; `None` for a node built during conversion.
    pub(crate) fn source_text(&self, node: &Value) -> Option<String> {
        self.locate(node).map(|location| self.lines.text(&location))
    }

    /// `refusal`, placed at `node` unless a node inside it placed it already.
    pub(crate) fn placed(&self, mut refusal: Refusal, node: &Value) -> Refusal {
        if refusal.location.is_none() {
            refusal.location = self.locate(node);
        }
        refusal
    }
}

/// How deep inside a built node its copies of tree nodes are looked for.
const ADOPTION_DEPTH: usize = 3;

/// Each node of a copy inside `built`, within `depth` levels, with its original.
fn copies_inside(built: &Value, originals: &[Value], depth: usize) -> Vec<(usize, usize)> {
    if let Some(original) = originals.iter().find(|original| *original == built) {
        return paired_nodes(built, original);
    }
    if depth == 0 {
        return Vec::new();
    }
    children(built)
        .into_iter()
        .flat_map(|child| copies_inside(child, originals, depth - 1))
        .collect()
}

/// Each node of `copy` with the node of `original` at the same place.
fn paired_nodes(copy: &Value, original: &Value) -> Vec<(usize, usize)> {
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    let mut pending: Vec<(&Value, &Value)> = vec![(copy, original)];
    while let Some((copy, original)) = pending.pop() {
        pairs.push((address(copy), address(original)));
        pending.extend(matched_children(copy, original));
    }
    pairs
}

/// The range of every node of `tree`: its own in `positioned`, else the range of its children's.
fn node_ranges(tree: &Value, positioned: &Value) -> HashMap<usize, (usize, usize)> {
    let mut ranges: HashMap<usize, (usize, usize)> = HashMap::new();
    let mut pending: Vec<(&Value, &Value, bool)> = vec![(tree, positioned, false)];
    while let Some((node, positioned, expanded)) = pending.pop() {
        let pairs = matched_children(node, positioned);
        if !expanded {
            pending.push((node, positioned, true));
            pending.extend(
                pairs
                    .into_iter()
                    .map(|(child, positioned)| (child, positioned, false)),
            );
            continue;
        }
        let inside = pairs
            .iter()
            .filter_map(|(child, _)| ranges.get(&address(child)).copied())
            .reduce(|(first, last), (start, end)| (first.min(start), last.max(end)));
        if let Some(range) = own_range(positioned).or(inside) {
            ranges.insert(address(node), range);
        }
    }
    ranges
}

/// The children of `node` with the children of `other` under the same field or index.
fn matched_children<'v>(node: &'v Value, other: &'v Value) -> Vec<(&'v Value, &'v Value)> {
    match (node, other) {
        (Value::Object(fields), Value::Object(other_fields)) => fields
            .iter()
            .filter_map(|(field, child)| Some((child, other_fields.get(field)?)))
            .collect(),
        (Value::Array(items), Value::Array(other_items)) => items.iter().zip(other_items).collect(),
        _ => Vec::new(),
    }
}

fn children(node: &Value) -> Vec<&Value> {
    match node {
        Value::Object(fields) => fields.values().collect(),
        Value::Array(items) => items.iter().collect(),
        _ => Vec::new(),
    }
}

fn address(node: &Value) -> usize {
    std::ptr::from_ref(node) as usize
}

fn own_range(positioned: &Value) -> Option<(usize, usize)> {
    let span = positioned.get(SPAN_FIELD)?;
    let start = span.get(SPAN_START)?.as_u64()?;
    let end = span.get(SPAN_END)?.as_u64()?;
    Some((start as usize, end as usize))
}
