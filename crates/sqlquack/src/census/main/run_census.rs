//! `run_census`: the census of every SQL file under some paths.

use std::path::PathBuf;

use crate::census::_helpers::aggregate::aggregate;
use crate::census::_helpers::corpus::inspect_files;
use crate::census::_helpers::files::collect_sql_files;
use crate::census::errors::CensusError;
use crate::census::models::{CensusOptions, CensusReport};

/// The census of every `.sql` file under `paths` against the built-in rule catalogue.
pub fn run_census(paths: &[PathBuf], options: &CensusOptions) -> Result<CensusReport, CensusError> {
    let files = collect_sql_files(paths)?;
    let censuses = inspect_files(&files, options)?;
    Ok(aggregate(
        &censuses,
        options.dialect,
        &options.coverage_points,
    ))
}
