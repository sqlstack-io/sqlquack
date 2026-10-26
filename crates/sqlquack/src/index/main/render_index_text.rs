//! `render_index_text`: the index as plain text, one dependent per line so revisions diff cleanly.

use crate::index::_helpers::text::{Header, dependents, header, keyed, lines};
use crate::index::models::ProvenanceIndex;

/// The index as text: a header line per identifier, then its rules, uses, cases and files.
pub fn render_index_text(index: &ProvenanceIndex) -> String {
    let totals = index.totals;
    let mut text = format!(
        "{} cases, {} files: {} converted, {} refused\n",
        totals.cases, totals.files, totals.converted, totals.refused
    );
    for (section, entries) in [
        ("building block", &index.building_blocks),
        ("typing function", &index.typing),
    ] {
        for entry in entries {
            let title = Header {
                section,
                id: &entry.id,
                rules: Some(entry.rules.len()),
            };
            text.push_str(&header(&title, &entry.dependents));
            text.push_str(&lines("rule", &entry.rules));
            text.push_str(&dependents(&entry.dependents));
        }
    }
    for entry in &index.rules {
        let title = Header {
            section: "rule",
            id: &entry.id,
            rules: None,
        };
        text.push_str(&header(&title, &entry.dependents));
        text.push_str(&lines("uses", &entry.uses));
        text.push_str(&dependents(&entry.dependents));
    }
    text.push_str(&keyed("equivalence", &index.equivalences));
    text.push_str(&keyed("refused construct", &index.refusals));
    text
}
