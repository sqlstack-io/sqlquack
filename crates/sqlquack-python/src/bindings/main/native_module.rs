//! `native_module`: the `sqlquack._native` module the `sqlquack` package wraps.

use pyo3::types::{PyModule, PyModuleMethods};
use pyo3::{Bound, PyResult, pymodule, wrap_pyfunction};

use crate::bindings::_helpers::check_json::check_json;
use crate::bindings::_helpers::convert_json::convert_json;

/// Adds the module's functions and its version.
#[pymodule(name = "_native")]
pub fn native_module(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(check_json, module)?)?;
    module.add_function(wrap_pyfunction!(convert_json, module)?)?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
