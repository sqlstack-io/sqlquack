//! `check_json`: what `sqlquack check --json` reports, with the rendered diagnostics.

use pyo3::exceptions::PyValueError;
use pyo3::{PyResult, pyfunction};
use sqlquack::analysis::main::check_source::check_source;
use sqlquack::cli::models::CheckReport;
use sqlquack_diagnostics::diagnostics::main::render_diagnostics::render_diagnostics;
use sqlquack_parse::parsing::main::checked_call_depth::checked_call_depth;

use crate::bindings::models::Rendered;

/// The check report of `sql` as JSON, named `path`; `ValueError` without SQL or past the ceiling.
#[pyfunction]
#[pyo3(signature = (sql, path, max_call_depth=None))]
pub(crate) fn check_json(sql: &str, path: &str, max_call_depth: Option<usize>) -> PyResult<String> {
    let max_call_depth = max_call_depth
        .map(checked_call_depth)
        .transpose()
        .map_err(|reason| PyValueError::new_err(format!("max_call_depth: {reason}")))?;
    let diagnostics = check_source(sql, max_call_depth).map_err(PyValueError::new_err)?;
    let rendered = render_diagnostics(&diagnostics, sql, path);
    let report = CheckReport {
        ok: diagnostics.is_empty(),
        diagnostics,
    };
    serde_json::to_string(&Rendered { report, rendered })
        .map_err(|error| PyValueError::new_err(error.to_string()))
}
