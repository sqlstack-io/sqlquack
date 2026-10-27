//! Inspecting a corpus: every file's census, chaining converted models into the schema.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use rayon::ThreadPoolBuilder;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::Schema;

use crate::census::_helpers::inspect::{FileCensus, inspect_file};
use crate::census::_helpers::placeholders::relation_name;
use crate::census::constants::WORKER_STACK_BYTES;
use crate::census::errors::CensusError;
use crate::census::models::{CensusOptions, CensusProgress};

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
    let mut schema = options.schema.clone();
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
                        options.dialect,
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
            if let (Some(columns), Some(relation)) = (&census.outputs, model_relation(path))
                && let std::collections::btree_map::Entry::Vacant(entry) =
                    schema.relations.entry(relation)
            {
                entry.insert(columns.clone());
                added += 1;
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
    Ok(censuses)
}

/// The placeholder relation a model file stands for: its file name, as references name it.
pub(crate) fn model_relation(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    Some(relation_name(stem.as_bytes()).to_ascii_uppercase())
}
