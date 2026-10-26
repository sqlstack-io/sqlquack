//! `build_index`: conversion records inverted into identifier → rules → cases and files.

use std::collections::{BTreeMap, BTreeSet};

use crate::index::_helpers::accumulate::{DependentSets, rule_entries, used_entries};
use crate::index::models::{ConversionRecord, IndexTotals, ProvenanceIndex, RecordSource};

/// The index of `records`; its order does not depend on the order of `records`.
pub fn build_index(records: &[ConversionRecord]) -> ProvenanceIndex {
    let mut totals = IndexTotals::default();
    let mut blocks = DependentSets::default();
    let mut typing = DependentSets::default();
    let mut rules = DependentSets::default();
    let mut equivalences = DependentSets::default();
    let mut refusals = DependentSets::default();
    let mut callers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut uses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for record in records {
        match &record.source {
            RecordSource::Case(_) => totals.cases += 1,
            RecordSource::File(_) => totals.files += 1,
        }
        let Some(provenance) = &record.provenance else {
            totals.refused += 1;
            for construct in &record.refused_constructs {
                refusals.add(construct, &record.source);
            }
            continue;
        };
        totals.converted += 1;
        for (target, ids) in [
            (&mut blocks, &provenance.building_blocks),
            (&mut typing, &provenance.typing),
            (&mut rules, &provenance.rules),
            (&mut equivalences, &provenance.equivalences),
        ] {
            for id in ids {
                target.add(id, &record.source);
            }
        }
        for (rule, used) in &provenance.rule_uses {
            uses.entry(rule.clone())
                .or_default()
                .extend(used.iter().cloned());
            for id in used {
                callers.entry(id.clone()).or_default().insert(rule.clone());
            }
        }
    }
    ProvenanceIndex {
        totals,
        building_blocks: used_entries(&blocks, &callers),
        typing: used_entries(&typing, &callers),
        rules: rule_entries(&rules, &uses),
        equivalences: equivalences.keyed(),
        refusals: refusals.keyed(),
    }
}
