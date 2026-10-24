//! `run_census`: the census of every SQL file under some paths.

use std::path::PathBuf;

use rayon::ThreadPoolBuilder;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;

use crate::census::_helpers::aggregate::aggregate;
use crate::census::_helpers::files::collect_sql_files;
use crate::census::_helpers::inspect::{FileCensus, inspect_file};
use crate::census::constants::WORKER_STACK_BYTES;
use crate::census::errors::CensusError;
use crate::census::models::{CensusOptions, CensusReport};

/// The census of every `.sql` file under `paths` against the built-in rule catalogue.
pub fn run_census(paths: &[PathBuf], options: &CensusOptions) -> Result<CensusReport, CensusError> {
    let files = collect_sql_files(paths)?;
    let catalogue = builtin_catalogue();
    let workers = ThreadPoolBuilder::new()
        .stack_size(WORKER_STACK_BYTES)
        .build()
        .map_err(|error| CensusError::Workers(error.to_string()))?;
    let censuses: Vec<FileCensus> = workers.install(|| {
        files
            .par_iter()
            .map(|path| inspect_file(path, options.dialect, &catalogue))
            .collect::<Result<Vec<FileCensus>, CensusError>>()
    })?;
    Ok(aggregate(
        &censuses,
        options.dialect,
        &options.coverage_points,
    ))
}
