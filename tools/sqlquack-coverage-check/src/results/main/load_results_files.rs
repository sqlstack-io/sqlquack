//! `load_results_files`: the link, list and review files, checked against the catalogue.

use std::path::Path;

use crate::formats::models::SyntaxCoverage;
use crate::results::_helpers::files::load_files;
use crate::results::models::ResultsFiles;
use crate::translation::main::catalogued_forms::catalogued_forms;

/// The files under the translation `directory`, with the cited cases under `cases`, or every problem.
pub fn load_results_files(
    (directory, cases): (&Path, &Path),
    coverage: &SyntaxCoverage,
) -> Result<ResultsFiles, Vec<String>> {
    load_files((directory, cases), &catalogued_forms(coverage, None))
}
