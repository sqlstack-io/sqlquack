//! `python_functions`: the Python functions converted SQL calls.

use std::collections::{BTreeMap, BTreeSet};

use sqlquack_policy::policy::constants::PROJECT_UDF;
use sqlquack_semantics::types::models::Schema;

use crate::analysis::_helpers::python_signature::python_signature;
use crate::analysis::models::PythonSignature;

/// The Python functions whose `project_udf:<NAME>` labels `divergences` holds, by name.
pub fn python_functions(
    divergences: &BTreeMap<String, BTreeSet<String>>,
    schema: &Schema,
) -> Result<Vec<PythonSignature>, String> {
    let prefix = format!("{PROJECT_UDF}:");
    let mut signatures: Vec<PythonSignature> = Vec::new();
    for name in divergences
        .keys()
        .filter_map(|label| label.strip_prefix(&prefix))
    {
        if let Some(spec) = schema.functions.get(name) {
            signatures.push(python_signature(name, spec)?);
        }
    }
    Ok(signatures)
}
