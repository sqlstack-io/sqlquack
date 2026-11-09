//! The functions a case defines: their schema, DuckDB macros and provenance.

use sqlquack_convert::snowflake::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_convert::snowflake::conversion::models::ConversionOptions;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_evidence::cases::models::{CaseFunction, FunctionBodyLanguage, InputColumn};
use sqlquack_evidence::replay::models::PythonFunction;
use sqlquack_semantics::snowflake::types::models::{
    ColumnSpec, FunctionLanguage, FunctionReturns, FunctionSpec, FunctionTraits, Schema,
};
use sqlquack_semantics::snowflake::typing::main::cast_target::cast_target;

use crate::analysis::_helpers::python_signature::python_signature;
use crate::analysis::models::CaseFunctions;
use sqlquack_convert::snowflake::analysis::main::analyze_function::analyze_function;

/// Every function typed into the schema, then each body converted to a macro in file order.
pub(crate) fn case_functions(
    functions: &[CaseFunction],
    options: &ConversionOptions,
) -> Result<CaseFunctions, Vec<String>> {
    let log = ProvenanceLog::default();
    let mut schema = Schema::default();
    for function in functions {
        let spec = typed(function, &log).map_err(|reason| vec![reason])?;
        schema
            .functions
            .insert(function.name.to_ascii_uppercase(), spec);
    }
    let mut converted = CaseFunctions {
        provenance: Vec::with_capacity(functions.len()),
        setup: Vec::with_capacity(functions.len()),
        schema: Schema::default(),
        python: Vec::new(),
    };
    for function in functions {
        let name = function.name.to_ascii_uppercase();
        let spec = &schema.functions[&name];
        if function.language == FunctionBodyLanguage::Python {
            converted
                .python
                .push(python_function((&name, function), spec).map_err(|reason| vec![reason])?);
            continue;
        }
        let body = analyze_function(
            (&function.body, spec),
            &builtin_catalogue(),
            (&schema, options),
        )
        .map_err(|refusals| {
            refusals
                .iter()
                .map(|refusal| {
                    format!(
                        "function {}: {}: {}",
                        function.name, refusal.category, refusal.reason
                    )
                })
                .collect::<Vec<String>>()
        })?;
        converted.setup.push(format!(
            "CREATE MACRO \"{name}\"({}) AS {}",
            body.parameters.join(", "),
            body.body
        ));
        converted.provenance.push(body.provenance);
    }
    converted.schema = schema;
    Ok(converted)
}

fn typed(function: &CaseFunction, log: &ProvenanceLog) -> Result<FunctionSpec, String> {
    let returns = match (&function.returns, &function.returns_table) {
        (_, Some(columns)) => FunctionReturns::Table(typed_columns(function, columns, log)?),
        (Some(data_type), None) => FunctionReturns::Scalar(
            cast_target(data_type, log)
                .map_err(|reason| format!("function {}: {reason}", function.name))?
                .data_type,
        ),
        (None, None) => return Err(format!("function {} returns nothing", function.name)),
    };
    Ok(FunctionSpec {
        language: match function.language {
            FunctionBodyLanguage::Sql => FunctionLanguage::Sql,
            FunctionBodyLanguage::Python => FunctionLanguage::Python,
        },
        arguments: typed_columns(function, &function.arguments, log)?,
        returns,
        traits: FunctionTraits::default(),
    })
}

fn typed_columns(
    function: &CaseFunction,
    columns: &[InputColumn],
    log: &ProvenanceLog,
) -> Result<Vec<ColumnSpec>, String> {
    columns
        .iter()
        .map(|column| {
            Ok(ColumnSpec::new(
                column.name.to_ascii_uppercase(),
                cast_target(&column.data_type, log)
                    .map_err(|reason| format!("function {}: {reason}", function.name))?
                    .data_type,
            ))
        })
        .collect()
}

/// A Python function as the replay registers it, with its handler and source.
fn python_function(
    (name, function): (&str, &CaseFunction),
    spec: &FunctionSpec,
) -> Result<PythonFunction, String> {
    let signature = python_signature(name, spec)?;
    Ok(PythonFunction {
        name: signature.name,
        abi: signature.abi,
        arguments: signature.arguments,
        returns: signature.returns,
        volatility: signature.volatility,
        null_input: signature.null_input,
        handler: function.handler.clone().unwrap_or_default(),
        source: function.body.clone(),
    })
}
