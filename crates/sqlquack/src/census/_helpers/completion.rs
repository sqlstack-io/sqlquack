//! Completing refused files' refusals, upstream models first.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use sqlquack_semantics::types::models::{ColumnSpec, Schema};

use crate::census::_helpers::corpus::{model_plain_name, model_relation, referenced_model};
use crate::census::_helpers::inspect::FileCensus;
use crate::census::errors::CensusError;

/// A completed file: its index, its census with every refusal, and the columns conversion reached.
pub(crate) type Completed = (usize, FileCensus, Option<Vec<ColumnSpec>>);

/// Completes the refusals of a wave of files over a schema.
pub(crate) type Completer<'a> =
    dyn Fn(&BTreeSet<usize>, &Schema, &[FileCensus]) -> Result<Vec<Completed>, CensusError> + 'a;

/// Every refused file completed by `complete`, upstream models first, typing readers when chaining.
pub(crate) fn complete_in_waves(
    files: &[PathBuf],
    mut censuses: Vec<FileCensus>,
    (schema, chain_models): (&Schema, bool),
    complete: &Completer<'_>,
) -> Result<Vec<FileCensus>, CensusError> {
    let mut by_relation: BTreeMap<String, usize> = BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        if let Some(relation) = model_relation(file) {
            by_relation
                .entry(relation.to_ascii_uppercase())
                .or_insert(index);
        }
    }
    let mut pending: BTreeSet<usize> = (0..censuses.len())
        .filter(|index| censuses[*index].parsed && !censuses[*index].refusal_categories.is_empty())
        .collect();
    let depends: BTreeMap<usize, BTreeSet<usize>> = pending
        .iter()
        .map(|index| {
            (
                *index,
                refused_upstream(&censuses[*index], *index, (&by_relation, &pending)),
            )
        })
        .collect();
    let mut schema = schema.clone();
    while !pending.is_empty() {
        let mut wave: BTreeSet<usize> = pending
            .iter()
            .copied()
            .filter(|index| ready(&depends[index], &pending))
            .collect();
        if wave.is_empty() {
            wave = pending.clone();
        }
        for (index, census, columns) in complete(&wave, &schema, &censuses)? {
            censuses[index] = census;
            pending.remove(&index);
            if let (true, Some(columns)) = (chain_models, columns) {
                let file = &files[index];
                for relation in model_relation(file)
                    .into_iter()
                    .chain(model_plain_name(file))
                {
                    schema.relations.entry(relation).or_insert(columns.clone());
                }
            }
        }
    }
    Ok(censuses)
}

/// The refused models `census`, at `index`, reads.
fn refused_upstream(
    census: &FileCensus,
    index: usize,
    (by_relation, refused): (&BTreeMap<String, usize>, &BTreeSet<usize>),
) -> BTreeSet<usize> {
    let mut upstream: BTreeSet<usize> = BTreeSet::new();
    for relation in census.references.iter().chain(&census.unknown_relations) {
        if let Some(model) = by_relation.get(&referenced_model(relation))
            && *model != index
            && refused.contains(model)
        {
            upstream.insert(*model);
        }
    }
    upstream
}

/// Whether every model in `upstream` is completed.
fn ready(upstream: &BTreeSet<usize>, pending: &BTreeSet<usize>) -> bool {
    upstream.is_disjoint(pending)
}
