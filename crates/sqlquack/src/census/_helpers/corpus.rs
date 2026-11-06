//! Inspecting a corpus: every file's census, chaining converted models into the schema.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use rayon::ThreadPoolBuilder;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use std::collections::BTreeSet;

use sqlquack_semantics::names::constants::RELATION_KEY_SEPARATOR;
use sqlquack_semantics::types::models::{ColumnSpec, Schema};

use crate::census::_helpers::completion::{Completed, complete_in_waves};
use crate::census::_helpers::inspect::{FileCensus, complete_refusals, inspect_file};
use crate::census::_helpers::placeholders::relation_name;
use crate::census::constants::{INTEGER_CURSOR_BOUND, TIMESTAMP_CURSOR_BOUND, WORKER_STACK_BYTES};
use crate::census::errors::CensusError;
use crate::census::models::{CensusOptions, CensusProgress, CursorType};

/// Each file's census, typing converted models as later files' relations when chaining.
pub(crate) fn inspect_files(
    files: &[PathBuf],
    options: &CensusOptions,
    progress: &CensusProgress,
) -> Result<Vec<FileCensus>, CensusError> {
    let catalogue = builtin_catalogue();
    let workers = ThreadPoolBuilder::new()
        .stack_size(WORKER_STACK_BYTES)
        .build()
        .map_err(|error| CensusError::Workers(error.to_string()))?;
    let declared_model = files
        .iter()
        .filter_map(|path| model_relation(path))
        .find(|relation| declares_offset(options.schema.relations.get(relation)));
    if let Some(relation) = declared_model {
        return Err(CensusError::DeclaredModelOffset(relation));
    }
    let mut schema = options.schema.clone();
    let cursor_bound: Option<&str> = options.cursor_type.map(|cursor| match cursor {
        CursorType::Timestamp => TIMESTAMP_CURSOR_BOUND,
        CursorType::Integer => INTEGER_CURSOR_BOUND,
    });
    let inspect_all = |schema: &Schema, selected: &[usize]| {
        progress.pass.fetch_add(1, Ordering::Relaxed);
        progress.done.store(0, Ordering::Relaxed);
        progress.total.store(selected.len(), Ordering::Relaxed);
        workers.install(|| {
            selected
                .par_iter()
                .map(|index| {
                    let census = inspect_file(
                        &files[*index],
                        (options.dialect, cursor_bound),
                        &catalogue,
                        (schema, &options.conversion),
                    );
                    progress.done.fetch_add(1, Ordering::Relaxed);
                    progress.inspected.fetch_add(1, Ordering::Relaxed);
                    census
                })
                .collect::<Result<Vec<FileCensus>, CensusError>>()
        })
    };
    let everything: Vec<usize> = (0..files.len()).collect();
    let mut censuses: Vec<FileCensus> = inspect_all(&schema, &everything)?;
    for _ in 0..usize::from(options.chain_models) * files.len() {
        let mut added = 0usize;
        for (path, census) in files.iter().zip(&censuses) {
            let Some(columns) = &census.outputs else {
                continue;
            };
            for relation in model_relation(path)
                .into_iter()
                .chain(model_plain_name(path))
            {
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    schema.relations.entry(relation)
                {
                    entry.insert(columns.clone());
                    added += 1;
                }
            }
        }
        if added == 0 {
            break;
        }
        let retry: Vec<usize> = everything
            .iter()
            .copied()
            .filter(|index| censuses[*index].outputs.is_none())
            .collect();
        for (index, mut census) in retry.iter().zip(inspect_all(&schema, &retry)?) {
            census.elapsed += censuses[*index].elapsed;
            censuses[*index] = census;
        }
    }
    let complete = |wave: &BTreeSet<usize>, schema: &Schema, censuses: &[FileCensus]| {
        progress.pass.fetch_add(1, Ordering::Relaxed);
        progress.done.store(0, Ordering::Relaxed);
        progress.total.store(wave.len(), Ordering::Relaxed);
        workers.install(|| {
            wave.par_iter()
                .map(|index| {
                    let (census, columns) = complete_refusals(
                        (&censuses[*index], &files[*index]),
                        (options.dialect, cursor_bound),
                        &catalogue,
                        (schema, &options.conversion),
                    )?;
                    progress.done.fetch_add(1, Ordering::Relaxed);
                    Ok((*index, census, columns))
                })
                .collect::<Result<Vec<Completed>, CensusError>>()
        })
    };
    let censuses = complete_in_waves(files, censuses, (&schema, options.chain_models), &complete)?;
    Ok(censuses)
}

/// The placeholder relation a model file stands for: its file name, as references name it.
pub(crate) fn model_relation(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    Some(relation_name(stem.as_bytes()).to_ascii_uppercase())
}

/// The relation dbt's compiled SQL reads a model file by, qualifiers aside: its file name, upper case.
pub(crate) fn model_plain_name(path: &Path) -> Option<String> {
    Some(path.file_stem()?.to_str()?.to_ascii_uppercase())
}

/// The model relation a reference reads, upper case: a qualified name by its last part, as dbt writes.
pub(crate) fn referenced_model(relation: &str) -> String {
    match relation.rsplit_once(RELATION_KEY_SEPARATOR) {
        Some((_, name)) => relation_name(name.as_bytes()).to_ascii_uppercase(),
        None => relation.to_ascii_uppercase(),
    }
}

/// Whether a relation's schema columns declare an offset for any column.
fn declares_offset(columns: Option<&Vec<ColumnSpec>>) -> bool {
    columns.is_some_and(|columns| {
        columns
            .iter()
            .any(|column| !column.declared_offsets.is_empty())
    })
}
