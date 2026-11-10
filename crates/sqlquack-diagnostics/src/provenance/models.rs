//! Provenance records.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use sqlquack_syntax::tokens::models::SourceLocation;

use crate::provenance::constants::QUERY_CONSTRUCT;

/// Everything one conversion relied on, each set sorted for deterministic output.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Recorded spelling pairs a rule relied on.
    pub equivalences: BTreeSet<String>,
    /// Conversion rules applied.
    pub rules: BTreeSet<String>,
    /// DuckDB building blocks the rules emitted through.
    pub building_blocks: BTreeSet<String>,
    /// Snowflake typing functions the rules relied on.
    pub typing: BTreeSet<String>,
    /// For each rule, the building blocks and typing functions it called itself.
    #[serde(default)]
    pub rule_uses: BTreeMap<String, BTreeSet<String>>,
    /// Known policy divergences the conversion relies on, each with the constructs it applies to.
    #[serde(default)]
    pub divergences: BTreeMap<String, BTreeSet<String>>,
    /// For each rule, the shapes it accepted: the construct, its arguments and its facets.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shapes: BTreeMap<String, BTreeSet<Shape>>,
}

/// One accepted application of a rule: the construct converted, its arguments and its facets.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Shape {
    /// The construct key, such as `fn:GET` or `node:like`; `query` for a query-level rule.
    pub construct: String,
    /// The arguments converted for it, in source order.
    pub arguments: Vec<ShapeArgument>,
    /// Further dimensions the rule declares, such as a window's frame kind.
    pub facets: BTreeMap<String, String>,
}

/// One argument of a shape.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ShapeArgument {
    /// The argument's type family, such as `text`, `integer` or `variant`.
    pub family: String,
    /// Whether the argument is a literal rather than a computed value.
    pub literal: bool,
    /// What feeds it: `column:<NAME>` or `literal:<source text>`; none for computed values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// One reliance on a known divergence: its id, the construct relying on it, and where, once placed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DivergenceUse {
    /// The divergence id, with any `:<name>` an assumption adds.
    pub id: String,
    /// The construct key relying on it, such as `fn:HASH`.
    pub construct: String,
    /// Where in the source, from the innermost converted expression around the use.
    pub location: Option<SourceLocation>,
}

/// A construct being converted: the rules it opened, its converted arguments and its facets.
#[derive(Debug, Default)]
struct Frame {
    shape: Shape,
    rules: Vec<String>,
    query: bool,
    converted: Vec<usize>,
    offsets: Vec<Option<usize>>,
}

/// Collects provenance while a conversion runs; steps borrow it immutably.
#[derive(Debug, Default)]
pub struct ProvenanceLog {
    recorded: RefCell<Provenance>,
    active_rules: RefCell<Vec<String>>,
    frames: RefCell<Vec<Frame>>,
    muted: Cell<usize>,
    uses: RefCell<Vec<DivergenceUse>>,
}

/// Keeps rules, building blocks, typing functions and equivalences unrecorded until dropped.
#[derive(Debug)]
#[must_use = "recording is muted only while its scope is held"]
pub struct MuteScope<'a> {
    log: &'a ProvenanceLog,
}

impl Drop for MuteScope<'_> {
    fn drop(&mut self) {
        self.log.muted.set(self.log.muted.get().saturating_sub(1));
    }
}

/// Keeps a rule active until dropped; building blocks and typing functions called meanwhile are its.
#[derive(Debug)]
#[must_use = "a rule is active only while its scope is held"]
pub struct RuleScope<'a> {
    log: &'a ProvenanceLog,
}

impl Drop for RuleScope<'_> {
    fn drop(&mut self) {
        self.log.active_rules.borrow_mut().pop();
    }
}

impl ProvenanceLog {
    /// Records a spelling pair a rule relies on.
    pub fn equivalence(&self, id: &str) {
        if self.is_muted() {
            return;
        }
        self.recorded
            .borrow_mut()
            .equivalences
            .insert(id.to_owned());
    }

    /// Records a rule and makes it the active rule until the returned scope is dropped.
    pub fn rule(&self, id: &str) -> RuleScope<'_> {
        if !self.is_muted() {
            self.recorded.borrow_mut().rules.insert(id.to_owned());
        }
        match self.frames.borrow_mut().last_mut() {
            Some(frame) if !frame.rules.iter().any(|rule| rule == id) => {
                frame.rules.push(id.to_owned())
            }
            Some(_) => {}
            None => self.record_shapes(
                &[id.to_owned()],
                Shape {
                    construct: QUERY_CONSTRUCT.to_owned(),
                    ..Shape::default()
                },
            ),
        }
        self.active_rules.borrow_mut().push(id.to_owned());
        RuleScope { log: self }
    }

    /// Records that the conversion relies on a known divergence at a construct.
    pub fn divergence(&self, id: &str, construct: &str) {
        self.recorded
            .borrow_mut()
            .divergences
            .entry(id.to_owned())
            .or_default()
            .insert(construct.to_owned());
        self.uses.borrow_mut().push(DivergenceUse {
            id: id.to_owned(),
            construct: construct.to_owned(),
            location: None,
        });
    }

    /// How many divergence uses are recorded so far, to pass to `place_divergences` later.
    pub fn divergence_mark(&self) -> usize {
        self.uses.borrow().len()
    }

    /// Places the uses recorded since `mark` that no expression inside placed, at `location`.
    pub fn place_divergences(&self, mark: usize, location: Option<SourceLocation>) {
        for unplaced in self
            .uses
            .borrow_mut()
            .iter_mut()
            .skip(mark)
            .filter(|recorded| recorded.location.is_none())
        {
            unplaced.location = location;
        }
    }

    /// Every divergence use recorded, each once, in the order recorded.
    pub fn divergence_uses(&self) -> Vec<DivergenceUse> {
        let mut distinct: Vec<DivergenceUse> = Vec::new();
        for recorded in self.uses.borrow().iter() {
            if !distinct.contains(recorded) {
                distinct.push(recorded.clone());
            }
        }
        distinct
    }

    /// Records a DuckDB building block, used by the active rule.
    pub fn building_block(&self, id: &str) {
        if self.is_muted() {
            return;
        }
        self.recorded
            .borrow_mut()
            .building_blocks
            .insert(id.to_owned());
        self.used(id);
    }

    /// Records a Snowflake typing function, used by the active rule.
    pub fn typing(&self, id: &str) {
        if self.is_muted() {
            return;
        }
        self.recorded.borrow_mut().typing.insert(id.to_owned());
        self.used(id);
    }

    fn used(&self, id: &str) {
        if let Some(rule) = self.active_rules.borrow().last() {
            self.recorded
                .borrow_mut()
                .rule_uses
                .entry(rule.clone())
                .or_default()
                .insert(id.to_owned());
        }
    }

    /// Starts converting `construct`, a query when `query`; rules opened until `leave` accept it.
    pub fn enter(&self, construct: &str, query: bool) {
        self.frames.borrow_mut().push(Frame {
            shape: Shape {
                construct: construct.to_owned(),
                ..Shape::default()
            },
            rules: Vec::new(),
            query,
            converted: Vec::new(),
            offsets: Vec::new(),
        });
    }

    /// Records a facet of the construct being converted, such as a window's frame kind.
    pub fn facet(&self, dimension: &str, value: &str) {
        if let Some(frame) = self.frames.borrow_mut().last_mut() {
            frame
                .shape
                .facets
                .insert(dimension.to_owned(), value.to_owned());
        }
    }

    /// Ends `enter`'s construct, giving a converted one's `argument` to its parent once per `node`.
    pub fn leave(&self, argument: Option<ShapeArgument>, (node, offset): (usize, Option<usize>)) {
        let Some(frame) = self.frames.borrow_mut().pop() else {
            return;
        };
        let Some(argument) = argument else {
            return;
        };
        let mut shape = frame.shape;
        let mut ordered: Vec<(Option<usize>, ShapeArgument)> =
            frame.offsets.into_iter().zip(shape.arguments).collect();
        if ordered.iter().all(|(offset, _)| offset.is_some()) {
            ordered.sort_by_key(|(offset, _)| *offset);
        }
        shape.arguments = match frame.query {
            true => Vec::new(),
            false => ordered.into_iter().map(|(_, argument)| argument).collect(),
        };
        self.record_shapes(&frame.rules, shape);
        if let Some(parent) = self.frames.borrow_mut().last_mut()
            && !parent.query
            && !parent.converted.contains(&node)
        {
            parent.converted.push(node);
            parent.offsets.push(offset);
            parent.shape.arguments.push(argument);
        }
    }

    fn record_shapes(&self, rules: &[String], shape: Shape) {
        if self.is_muted() {
            return;
        }
        let mut recorded = self.recorded.borrow_mut();
        for rule in rules {
            recorded
                .shapes
                .entry(rule.clone())
                .or_default()
                .insert(shape.clone());
        }
    }

    /// Mutes recording of everything but divergences, such as for a case's input fixture.
    pub fn muted(&self) -> MuteScope<'_> {
        self.muted.set(self.muted.get() + 1);
        MuteScope { log: self }
    }

    fn is_muted(&self) -> bool {
        self.muted.get() > 0
    }

    /// Everything recorded so far.
    pub fn snapshot(&self) -> Provenance {
        self.recorded.borrow().clone()
    }

    /// Everything recorded.
    pub fn finish(self) -> Provenance {
        self.recorded.into_inner()
    }
}
