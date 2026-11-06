//! `load_results_files`: the link, list and review files, checked against the catalogue.

use std::path::Path;

use crate::formats::models::SyntaxCoverage;
use crate::results::_helpers::files::load_files;
use crate::results::models::ResultsFiles;
use crate::translation::main::catalogued_forms::catalogued_forms;

/// The files under the translation `directory`, or every problem with them.
pub fn load_results_files(
    directory: &Path,
    coverage: &SyntaxCoverage,
) -> Result<ResultsFiles, Vec<String>> {
    load_files(directory, &catalogued_forms(coverage, None))
}
