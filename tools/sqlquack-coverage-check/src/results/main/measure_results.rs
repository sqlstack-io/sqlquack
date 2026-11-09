//! `measure_results`: every catalogue form's state from the recorded statements linked to it.

use crate::formats::models::SyntaxCoverage;
use crate::results::_helpers::links::function_index;
use crate::results::_helpers::obligations::obligation_states;
use crate::results::_helpers::report::{assemble, form_results, link_all};
use crate::results::constants::STATEMENTS_FORMAT;
use crate::results::models::{ResultsFiles, ResultsReport, StatementsFile};

/// The report, or every problem: a wrong format, unmapped constructs, or a listed form matching.
pub fn measure_results(
    coverage: &SyntaxCoverage,
    files: &ResultsFiles,
    statements: &StatementsFile,
) -> Result<ResultsReport, Vec<String>> {
    if statements.format != STATEMENTS_FORMAT {
        return Err(vec![format!(
            "statements format {} is not {STATEMENTS_FORMAT}",
            statements.format
        )]);
    }
    let statements = statements.statements.as_slice();
    let functions = function_index(coverage);
    let links = link_all(statements, &functions, files)?;
    let states = obligation_states(&files.obligations, statements)?;
    let results = form_results(coverage, files, (statements, &links), &states)?;
    Ok(assemble(
        files,
        functions,
        (statements, &links),
        (results, states),
    ))
}
