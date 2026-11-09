//! The census of every SQL file under some paths, with each file's conversion time.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::census::_helpers::aggregate::aggregate;
use crate::census::_helpers::blockers::blockers;
use crate::census::_helpers::corpus::inspect_files;
use crate::census::_helpers::files::collect_sql_files;
use crate::census::errors::CensusError;
use crate::census::models::{CensusOptions, CensusProgress, CensusRun};
use crate::planning::main::assumption_free_counts::assumption_free_counts;
use crate::planning::main::plan_census::plan_census;
use sqlquack_convert::snowflake::analysis::main::assumption_id::assumption_id;

/// The census and each file's conversion time, counting files into `progress` as they finish.
pub(crate) fn run_census_timed(
    paths: &[PathBuf],
    options: &CensusOptions,
    progress: &CensusProgress,
) -> Result<CensusRun, CensusError> {
    let files = collect_sql_files(paths)?;
    let censuses = inspect_files(&files, options, progress)?;
    let mut report = aggregate(
        &censuses,
        options.dialect,
        &options.coverage_points,
        options.names,
    );
    let (blocked, plan_input) = blockers(&files, &censuses, &options.schema, options.names);
    report.blockers = blocked;
    let assumed: BTreeSet<String> = options.assume_supported.iter().cloned().collect();
    report.plan = plan_census(&plan_input, &assumed);
    report.refusals.convertible_strict =
        report.refusals.convertible - report.blockers.upstream_refused.len();
    let named: BTreeSet<&String> = options
        .conversion
        .divergences
        .iter()
        .filter(|(_, divergence)| divergence.assumption)
        .map(|(id, _)| id)
        .collect();
    let mut relied: Vec<BTreeSet<String>> = Vec::with_capacity(censuses.len());
    for census in &censuses {
        let mut ids: BTreeSet<String> = BTreeSet::new();
        if let Some(provenance) = &census.provenance {
            ids.extend(
                provenance
                    .divergences
                    .keys()
                    .filter(|id| named.contains(&assumption_id(id).to_owned()))
                    .cloned(),
            );
        }
        relied.push(ids);
    }
    let (unit, strict) = assumption_free_counts(&plan_input, &relied);
    report.refusals.convertible_no_assumptions = unit;
    report.refusals.convertible_strict_no_assumptions = strict;
    for (index, ids) in relied.iter().enumerate() {
        if censuses[index].parsed && censuses[index].refusal_categories.is_empty() {
            let shown: BTreeSet<String> = ids
                .iter()
                .map(|id| match options.names {
                    true => id.clone(),
                    false => assumption_id(id).to_owned(),
                })
                .collect();
            for id in shown {
                *report.refusals.assumptions.entry(id).or_default() += 1;
            }
        }
    }
    let mut timings: Vec<(PathBuf, std::time::Duration)> = files
        .into_iter()
        .zip(censuses.iter().map(|census| census.elapsed))
        .collect();
    timings.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    Ok(CensusRun { report, timings })
}
