//! The census sidecars: each case's relations, written as a source file of a project convert reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Value, json};

use crate::formats::main::current_dialect::current_dialect;
use crate::formats::models::SyntaxCoverage;
use crate::translation::constants::CENSUS_DIR;
use crate::translation::models::{CensusColumn, CensusEntry};

/// Every sidecar entry by case id, or every problem: an unknown case, a duplicate, or an empty list.
pub(crate) fn load_census(
    root: &Path,
    coverage: &SyntaxCoverage,
) -> Result<BTreeMap<String, CensusEntry>, Vec<String>> {
    let known: BTreeSet<&str> = coverage
        .cases
        .values()
        .flatten()
        .map(|case| case.case_id.as_str())
        .collect();
    let mut entries: BTreeMap<String, CensusEntry> = BTreeMap::new();
    let mut problems: Vec<String> = Vec::new();
    for family in current_dialect().families.iter() {
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

/// A census name as Snowflake resolves it: quoted keeps its case, unquoted is upper-cased.
fn resolved(name: &str) -> String {
    match name
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(quoted) => quoted.replace("\"\"", "\""),
        None => name.to_uppercase(),
    }
}

/// `name` resolved and quoted, so a source file declares exactly the name Snowflake resolves.
fn source_name(name: &str) -> String {
    format!("\"{}\"", resolved(name).replace('"', "\"\""))
}

/// The entry as a source file's text, in JSON's flow style: each relation with its typed columns.
pub(crate) fn source_file(entry: &CensusEntry) -> String {
    let sources: Vec<Value> = entry
        .relations
        .iter()
        .map(|relation| {
            let columns: Vec<Value> = relation.columns.iter().map(source_column).collect();
            json!({ "name": source_name(&relation.name), "columns": columns })
        })
        .collect();
    json!({ "sources": sources }).to_string()
}

/// A census column as a source file's column: its resolved, quoted name and its type.
fn source_column(column: &CensusColumn) -> Value {
    json!({ "name": source_name(&column.name), "type": column.column_type })
}
