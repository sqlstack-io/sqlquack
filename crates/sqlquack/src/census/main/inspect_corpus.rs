//! `inspect_corpus`: what converting each SQL file under some paths relied on, or refused.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::census::_helpers::corpus::inspect_files;
use crate::census::_helpers::files::collect_sql_files;
use crate::census::errors::CensusError;
use crate::census::models::{CensusOptions, CorpusFile};

/// Each `.sql` file under `paths`, in path order, with its provenance or refused constructs.
pub fn inspect_corpus(
    paths: &[PathBuf],
    options: &CensusOptions,
) -> Result<Vec<CorpusFile>, CensusError> {
    let files = collect_sql_files(paths)?;
    let censuses = inspect_files(&files, options)?;
    let mut corpus: Vec<CorpusFile> = Vec::with_capacity(files.len());
    for (path, census) in files.into_iter().zip(censuses) {
        let constructs: BTreeSet<String> = census
            .refusal_constructs
            .into_iter()
            .map(|(_, construct)| construct)
            .collect();
        corpus.push(CorpusFile {
            path,
            provenance: census.provenance,
            refused_constructs: constructs.into_iter().collect(),
        });
    }
    Ok(corpus)
}
