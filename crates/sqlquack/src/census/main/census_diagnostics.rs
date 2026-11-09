//! `census_diagnostics`: each blocked file's diagnostics, what blocks files, and upstream warnings.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sqlquack_convert::snowflake::analysis::models::ConversionChoices;
use sqlquack_policy::policy::models::Policy;

use crate::census::_helpers::corpus::model_relation;
use crate::census::_helpers::inspect::diagnosed_file;
use crate::census::errors::CensusError;
use crate::census::models::{
    BlockedDiagnostics, BlockingCause, CensusDiagnostics, CensusFile, UpstreamWarning,
};

/// The diagnostics of `files`' blocked files, read again from disk, with help for `policy`.
pub fn census_diagnostics(
    files: &[CensusFile],
    help: (Option<&Policy>, &ConversionChoices),
) -> Result<CensusDiagnostics, CensusError> {
    let refused_models: BTreeMap<String, &Path> = files
        .iter()
        .filter(|file| file.blocked)
        .filter_map(|file| Some((model_relation(&file.path)?, file.path.as_path())))
        .collect();
    let mut blocked: Vec<BlockedDiagnostics> = Vec::new();
    let mut causes: BTreeMap<(String, String), BTreeSet<PathBuf>> = BTreeMap::new();
    for file in files.iter().filter(|file| file.blocked) {
        let (diagnosed, own) = diagnosed_file(file, &refused_models, help)?;
        for key in own {
            causes.entry(key).or_default().insert(file.path.clone());
        }
        blocked.push(diagnosed);
    }
    Ok(CensusDiagnostics {
        ok: blocked.is_empty(),
        files: files.len(),
        converted: files.len() - blocked.len(),
        blocked,
        causes: ranked(causes),
        upstream: upstream_warnings(files),
    })
}

/// Each cause with the files it blocks, most first.
fn ranked(causes: BTreeMap<(String, String), BTreeSet<PathBuf>>) -> Vec<BlockingCause> {
    let mut ranked: Vec<BlockingCause> = causes
        .into_iter()
        .map(|((code, cause), paths)| BlockingCause {
            code,
            cause,
            files: paths.len(),
        })
        .collect();
    ranked.sort_by_key(|cause| std::cmp::Reverse(cause.files));
    ranked
}

/// Each converted file reading refused models, with those models' files.
fn upstream_warnings(files: &[CensusFile]) -> Vec<UpstreamWarning> {
    files
        .iter()
        .filter(|file| !file.blocked && !file.refused_upstream.is_empty())
        .map(|file| UpstreamWarning {
            path: file.path.clone(),
            reads: file.refused_upstream.clone(),
        })
        .collect()
}
