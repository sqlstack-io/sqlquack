//! `changed_verdicts`: the statements whose verdict moved since it was recorded.

use std::collections::{BTreeMap, BTreeSet};

use crate::verdicts::models::{DialectVerdicts, Verdicts};

/// One dialect's changes in one corpus file, as readable lines.
fn dialect_changes(
    place: &str,
    recorded: &DialectVerdicts,
    current: &DialectVerdicts,
) -> Vec<String> {
    if recorded.statements != current.statements {
        return vec![format!(
            "{place}: {} statements, recorded {}",
            current.statements, recorded.statements
        )];
    }
    let was: BTreeSet<usize> = recorded.refused.iter().copied().collect();
    let is: BTreeSet<usize> = current.refused.iter().copied().collect();
    let refused = is
        .difference(&was)
        .map(|position| format!("{place} #{position}: now refused, recorded as parsed"));
    let parsed = was
        .difference(&is)
        .map(|position| format!("{place} #{position}: now parsed, recorded as refused"));
    refused.chain(parsed).collect()
}

/// Every statement whose verdict differs between `recorded` and `current`, as readable lines.
pub fn changed_verdicts(recorded: &Verdicts, current: &Verdicts) -> Vec<String> {
    let empty: BTreeMap<String, DialectVerdicts> = BTreeMap::new();
    let none: DialectVerdicts = DialectVerdicts::default();
    let names: BTreeSet<&String> = recorded
        .corpora
        .keys()
        .chain(current.corpora.keys())
        .collect();
    let mut lines: Vec<String> = Vec::new();
    for name in names {
        let before = recorded.corpora.get(name).unwrap_or(&empty);
        let after = current.corpora.get(name).unwrap_or(&empty);
        let dialects: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
        for dialect in dialects {
            lines.extend(dialect_changes(
                &format!("{name} {dialect}"),
                before.get(dialect).unwrap_or(&none),
                after.get(dialect).unwrap_or(&none),
            ));
        }
    }
    lines
}
