//! `convert_json`: what `sqlquack convert --json` reports, with the rendered diagnostics.

use pyo3::exceptions::PyValueError;
use pyo3::{PyResult, pyfunction};
use sqlquack::cli::main::convert_report::convert_report;
use sqlquack::cli::models::{
    ColorArguments, ConvertArguments, OutputArguments, OutputFormat, PolicyArguments,
};
use sqlquack_diagnostics::diagnostics::main::render_diagnostics::render_diagnostics;
use sqlquack_diagnostics::diagnostics::models::Diagnostic;
use sqlquack_semantics::snowflake::types::errors::SchemaError;
use sqlquack_semantics::snowflake::types::main::parse_schema::parse_schema;

use crate::bindings::models::{ConvertRequest, Rendered};

/// The convert report of `sql` under the JSON `request`; `ValueError` where the command exits 2.
#[pyfunction]
pub(crate) fn convert_json(sql: &str, request: &str, path: &str) -> PyResult<String> {
    let request: ConvertRequest =
        serde_json::from_str(request).map_err(|error| PyValueError::new_err(error.to_string()))?;
    let schema = request
        .schema
        .map(|schema| {
            parse_schema(&schema.to_string()).map_err(|error| match error {
                SchemaError::Shape(reason) => {
                    format!("the schema option is not a schema: {reason}")
                }
                SchemaError::Declaration(reason) => format!("the schema option: {reason}"),
            })
        })
        .transpose()
        .map_err(PyValueError::new_err)?;
    let arguments = ConvertArguments {
        path: None,
        schema,
        overrides: request.overrides,
        verified_overrides: pairs(request.verified_overrides),
        session: pairs(request.session),
        compact: request.compact,
        output: OutputArguments {
            format: OutputFormat::Json,
            json: false,
        },
        colors: ColorArguments::default(),
        conversion: PolicyArguments {
            project: request.project,
            policy: request.policy,
            builtin_policy: request.builtin_policy,
            strict: request.strict,
            utc_session: request.utc_session,
            confirm_declared_offsets: request.confirm_declared_offsets,
            python_runtime: request.python_runtime,
            loud_divergences_only: request.loud_divergences_only,
        },
    };
    let report = convert_report(&arguments, sql).map_err(PyValueError::new_err)?;
    let diagnostics: Vec<Diagnostic> = report
        .refusals
        .iter()
        .filter_map(|refusal| refusal.diagnostic.clone())
        .collect();
    let rendered = render_diagnostics(&diagnostics, sql, path);
    serde_json::to_string(&Rendered { report, rendered })
        .map_err(|error| PyValueError::new_err(error.to_string()))
}

/// `NAME=VALUE` arguments, as the command line takes them.
fn pairs(values: std::collections::BTreeMap<String, String>) -> Vec<String> {
    values
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect()
}
