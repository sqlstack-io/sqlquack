//! The census of every SQL file under some paths, with each file's conversion time.

use std::path::PathBuf;

use crate::census::_helpers::aggregate::aggregate;
use crate::census::_helpers::blockers::blockers;
use crate::census::_helpers::corpus::inspect_files;
use crate::census::_helpers::files::collect_sql_files;
use crate::census::errors::CensusError;
use crate::census::models::{CensusOptions, CensusProgress, CensusRun};

/// The census and each file's conversion time, counting files into `progress` as they finish.
pub(crate) fn run_census_timed(
    paths: &[PathBuf],
    options: &CensusOptions,
    progress: &CensusProgress,
) -> Result<CensusRun, CensusError> {
    let files = collect_sql_files(paths)?;
    let censuses = inspect_files(&files, options, progress)?;
    let mut report = aggregate(&censuses, options.dialect, &options.coverage_points);
    report.blockers = blockers(&files, &censuses, &options.schema, options.names);
    let mut timings: Vec<(PathBuf, std::time::Duration)> = files
        .into_iter()
        .zip(censuses.iter().map(|census| census.elapsed))
        .collect();
    timings.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    Ok(CensusRun { report, timings })
}
