//! `check_json`: what `sqlquack check --json` reports, with the rendered diagnostics.

use pyo3::exceptions::PyValueError;
use pyo3::{PyResult, pyfunction};
use sqlquack::analysis::main::check_source::check_source;
use sqlquack::cli::models::CheckReport;
use sqlquack_diagnostics::diagnostics::main::render_diagnostics::render_diagnostics;

use crate::bindings::models::Rendered;

/// The check report of `sql` as JSON, naming the source `path`; `ValueError` without SQL.
#[pyfunction]
pub(crate) fn check_json(sql: &str, path: &str) -> PyResult<String> {
    let diagnostics = check_source(sql).map_err(PyValueError::new_err)?;
    let rendered = render_diagnostics(&diagnostics, sql, path);
    let report = CheckReport {
        ok: diagnostics.is_empty(),
        diagnostics,
    };
    serde_json::to_string(&Rendered { report, rendered })
        .map_err(|error| PyValueError::new_err(error.to_string()))
}
