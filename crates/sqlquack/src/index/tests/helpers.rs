use std::collections::{BTreeMap, BTreeSet};

use sqlquack_core::provenance::models::Provenance;

use crate::index::models::{ConversionRecord, RecordSource};

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// A converted record whose rules each call the given building blocks and typing functions.
pub(crate) fn converted(source: RecordSource, rule_uses: &[(&str, &[&str])]) -> ConversionRecord {
    let uses: BTreeMap<String, BTreeSet<String>> = rule_uses
        .iter()
        .map(|(rule, used)| ((*rule).to_owned(), set(used)))
        .collect();
    let all: BTreeSet<String> = uses.values().flatten().cloned().collect();
    ConversionRecord {
        source,
        provenance: Some(Provenance {
            equivalences: BTreeSet::new(),
            rules: uses.keys().cloned().collect(),
            building_blocks: all
                .iter()
                .filter(|id| id.starts_with("duckdb."))
                .cloned()
                .collect(),
            typing: all
                .iter()
                .filter(|id| id.starts_with("snowflake."))
                .cloned()
                .collect(),
            rule_uses: uses,
            divergences: BTreeMap::new(),
        }),
        refused_constructs: Vec::new(),
    }
}

/// A refused record.
pub(crate) fn refused(source: RecordSource, constructs: &[&str]) -> ConversionRecord {
    ConversionRecord {
        source,
        provenance: None,
        refused_constructs: constructs
            .iter()
            .map(|construct| (*construct).to_owned())
            .collect(),
    }
}
