//! Provenance records.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

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
}

/// Collects provenance while a conversion runs; steps borrow it immutably.
#[derive(Debug, Default)]
pub struct ProvenanceLog {
    recorded: RefCell<Provenance>,
    active_rules: RefCell<Vec<String>>,
    muted: Cell<usize>,
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
