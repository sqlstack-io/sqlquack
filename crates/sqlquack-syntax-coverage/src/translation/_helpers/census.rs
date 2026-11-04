//! The census sidecars: each case's relations, written as the schema file convert reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Value, json};

use crate::formats::constants::FAMILIES;
use crate::translation::constants::CENSUS_DIR;
use crate::translation::models::CensusEntry;

/// Every sidecar entry by case id, or every problem: an unknown case id, a duplicate, or an empty
/// relation list or column list.
pub(crate) fn load_census(
    root: &Path,
    known: &BTreeSet<&str>,
) -> Result<BTreeMap<String, CensusEntry>, Vec<String>> {
    let mut entries: BTreeMap<String, CensusEntry> = BTreeMap::new();
    let mut problems: Vec<String> = Vec::new();
    for family in FAMILIES {
        let path = root.join(CENSUS_DIR).join(format!("{family}.jsonl"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let place: String = format!("{}:{}", path.display(), index + 1);
            let entry: CensusEntry = match serde_json::from_str(line) {
                Ok(entry) => entry,
                Err(error) => {
                    problems.push(format!("{place}: {error}"));
                    continue;
                }
            };
            if !known.contains(entry.case_id.as_str()) {
                problems.push(format!("{place}: case {} does not exist", entry.case_id));
            }
            if entry.relations.is_empty()
                || entry
                    .relations
                    .iter()
                    .any(|relation| relation.columns.is_empty())
            {
                problems.push(format!("{place}: a relation list or column list is empty"));
            }
            if let Some(first) = entries.insert(entry.case_id.clone(), entry) {
                problems.push(format!("{place}: case {} listed twice", first.case_id));
            }
        }
    }
    match problems.is_empty() {
        true => Ok(entries),
        false => Err(problems),
    }
}

/// A name as the census writes it, as Snowflake resolves it: a quoted name keeps its case, an
/// unquoted one is upper-cased.
fn resolved(name: &str) -> String {
    match name
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(quoted) => quoted.replace("\"\"", "\""),
        None => name.to_uppercase(),
    }
}

/// The entry as convert's `--schema` JSON: each resolved relation to its `[column, type]` pairs.
pub(crate) fn schema_json(entry: &CensusEntry) -> Value {
    Value::Object(
        entry
            .relations
            .iter()
            .map(|relation| {
                let columns: Vec<Value> = relation
                    .columns
                    .iter()
                    .map(|column| json!([resolved(&column.name), column.column_type]))
                    .collect();
                (resolved(&relation.name), Value::Array(columns))
            })
            .collect(),
    )
}
