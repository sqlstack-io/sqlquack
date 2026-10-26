//! `merge_provenance`: everything several conversions used, as one record.

use crate::provenance::models::Provenance;

/// The union of `records`, sorted.
pub fn merge_provenance<'a>(records: impl IntoIterator<Item = &'a Provenance>) -> Provenance {
    let mut merged = Provenance::default();
    for record in records {
        merged
            .equivalences
            .extend(record.equivalences.iter().cloned());
        merged.rules.extend(record.rules.iter().cloned());
        merged
            .building_blocks
            .extend(record.building_blocks.iter().cloned());
        merged.typing.extend(record.typing.iter().cloned());
        for (divergence, constructs) in &record.divergences {
            merged
                .divergences
                .entry(divergence.clone())
                .or_default()
                .extend(constructs.iter().cloned());
        }
        for (rule, uses) in &record.rule_uses {
            merged
                .rule_uses
                .entry(rule.clone())
                .or_default()
                .extend(uses.iter().cloned());
        }
    }
    merged
}
