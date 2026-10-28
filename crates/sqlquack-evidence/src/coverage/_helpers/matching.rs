//! Whether a recorded case puts a hazard into a rule's argument or rows.

use std::collections::BTreeMap;

use sqlquack_core::provenance::models::Shape;

use crate::cases::models::{CaseInput, LoadedCase};
use crate::coverage::constants::{
    CASE_ONLY_PAIR, COLUMN_SOURCE, GROUP_COLUMN, LITERAL_SOURCE, NULL_CELL,
};
use crate::coverage::models::Hazard;

/// Whether `source` feeds the hazard in `case`: alone in its column, or a lone literal, for errors.
pub(crate) fn source_shows(
    hazard: &Hazard,
    source: Option<&str>,
    (case, crowded): (&LoadedCase, bool),
    alone: bool,
) -> bool {
    let Some(source) = source else {
        return false;
    };
    if let Some(literal) = source.strip_prefix(LITERAL_SOURCE) {
        return hazard.pair.is_none() && !crowded && cell_is(hazard, literal);
    }
    let Some(input) = source
        .strip_prefix(COLUMN_SOURCE)
        .and_then(|name| input(case, name))
    else {
        return false;
    };
    match hazard.pair.as_deref() {
        Some(CASE_ONLY_PAIR) => {
            let mut folded: Vec<(String, &str)> = input
                .cells
                .iter()
                .map(|cell| (cell.trim().to_lowercase(), cell.trim()))
                .collect();
            folded.sort_unstable();
            folded.dedup();
            folded.windows(2).any(|pair| pair[0].0 == pair[1].0)
        }
        Some(_) => false,
        None if alone => input
            .cells
            .iter()
            .filter(|cell| !cell.trim().eq_ignore_ascii_case(NULL_CELL))
            .all(|cell| cell_is(hazard, cell)),
        None => input.cells.iter().any(|cell| cell_is(hazard, cell)),
    }
}

/// Whether the rows a shape's columns read in `case` are the row-set hazard.
pub(crate) fn rows_show(hazard: &Hazard, shape: &Shape, case: &LoadedCase) -> bool {
    if hazard
        .sql_contains
        .iter()
        .any(|text| contains_folded(&case.source_sql, text))
    {
        return true;
    }
    let Some(matcher) = hazard.group.as_deref() else {
        return false;
    };
    for argument in &shape.arguments {
        let fed = argument
            .source
            .as_deref()
            .and_then(|source| source.strip_prefix(COLUMN_SOURCE))
            .and_then(|name| input(case, name));
        let Some(fed) = fed else {
            continue;
        };
        if groups(case, fed)
            .values()
            .any(|cells| group_matches(matcher, cells))
        {
            return true;
        }
    }
    false
}

fn cell_is(hazard: &Hazard, cell: &str) -> bool {
    let cell = cell.trim();
    hazard
        .cells
        .iter()
        .any(|hazard_cell| hazard_cell.trim() == cell)
        || hazard
            .contains
            .iter()
            .any(|text| cell.contains(text.as_str()))
}

fn contains_folded(text: &str, needle: &str) -> bool {
    text.to_ascii_uppercase()
        .contains(&needle.to_ascii_uppercase())
}

fn input<'a>(case: &'a LoadedCase, name: &str) -> Option<&'a CaseInput> {
    case.inputs
        .iter()
        .find(|input| input.column.name.eq_ignore_ascii_case(name))
}

/// The cells of `fed` in each group of `case`'s group column, or in one group without it.
fn groups<'a>(case: &'a LoadedCase, fed: &'a CaseInput) -> BTreeMap<&'a str, Vec<&'a str>> {
    let keys: Option<&CaseInput> = input(case, GROUP_COLUMN);
    let mut grouped: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (index, cell) in fed.cells.iter().enumerate() {
        let key = keys
            .and_then(|keys| keys.cells.get(index))
            .map_or("", String::as_str);
        grouped.entry(key).or_default().push(cell.trim());
    }
    grouped
}

fn group_matches(matcher: &str, cells: &[&str]) -> bool {
    let values: Vec<&str> = cells
        .iter()
        .copied()
        .filter(|cell| !cell.eq_ignore_ascii_case(NULL_CELL))
        .collect();
    let mut distinct = values.clone();
    distinct.sort_unstable();
    distinct.dedup();
    match matcher {
        "all_null" => values.is_empty(),
        "mixed" => distinct.len() > 1,
        "null_beside_value" => !values.is_empty() && values.len() < cells.len(),
        "duplicates" => distinct.len() < values.len(),
        "single_row" => cells.len() == 1,
        _ => false,
    }
}
