//! Accumulating dependents per identifier while records are folded into an index.

use std::collections::{BTreeMap, BTreeSet};

use crate::index::models::{Dependents, KeyedEntry, RecordSource, RuleEntry, UsedEntry};

/// Cases and files per identifier, kept sorted.
#[derive(Debug, Default)]
pub(crate) struct DependentSets {
    pub(crate) by_id: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)>,
}

impl DependentSets {
    pub(crate) fn add(&mut self, id: &str, source: &RecordSource) {
        let (cases, files) = self.by_id.entry(id.to_owned()).or_default();
        match source {
            RecordSource::Case(name) => cases.insert(name.clone()),
            RecordSource::File(name) => files.insert(name.clone()),
        };
    }

    pub(crate) fn dependents(&self, id: &str) -> Dependents {
        self.by_id
            .get(id)
            .map(|(cases, files)| Dependents {
                cases: cases.iter().cloned().collect(),
                files: files.iter().cloned().collect(),
            })
            .unwrap_or_default()
    }

    pub(crate) fn keyed(&self) -> Vec<KeyedEntry> {
        self.by_id
            .keys()
            .map(|id| KeyedEntry {
                id: id.clone(),
                dependents: self.dependents(id),
            })
            .collect()
    }
}

fn listed(map: &BTreeMap<String, BTreeSet<String>>, id: &str) -> Vec<String> {
    map.get(id)
        .map(|items| items.iter().cloned().collect())
        .unwrap_or_default()
}

/// One entry per building block or typing function, with the rules calling it.
pub(crate) fn used_entries(
    sets: &DependentSets,
    callers: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<UsedEntry> {
    let mut entries: Vec<UsedEntry> = Vec::with_capacity(sets.by_id.len());
    for id in sets.by_id.keys() {
        entries.push(UsedEntry {
            id: id.clone(),
            rules: listed(callers, id),
            dependents: sets.dependents(id),
        });
    }
    entries
}

/// One entry per rule, with what it calls.
pub(crate) fn rule_entries(
    rules: &DependentSets,
    uses: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<RuleEntry> {
    let mut entries: Vec<RuleEntry> = Vec::with_capacity(rules.by_id.len());
    for id in rules.by_id.keys() {
        entries.push(RuleEntry {
            id: id.clone(),
            uses: listed(uses, id),
            dependents: rules.dependents(id),
        });
    }
    entries
}
