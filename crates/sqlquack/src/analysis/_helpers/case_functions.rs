//! The functions a case defines: their schema, DuckDB macros and provenance.

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_evidence::cases::models::{CaseFunction, InputColumn};
use sqlquack_rules::catalogue::main::builtin_catalogue::builtin_catalogue;
use sqlquack_rules::conversion::models::{
    ColumnSpec, ConversionOptions, FunctionLanguage, FunctionReturns, FunctionSpec, Schema,
};
use sqlquack_snowflake::typing::main::cast_target::cast_target;

use crate::analysis::main::analyze_function::analyze_function;
use crate::analysis::models::CaseFunctions;

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
    };
    for function in functions {
        let name = function.name.to_ascii_uppercase();
        let spec = &schema.functions[&name];
        let body = analyze_function(
            (&function.body, spec),
            SourceDialect::Snowflake,
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
        language: FunctionLanguage::Sql,
        arguments: typed_columns(function, &function.arguments, log)?,
        returns,
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
            Ok(ColumnSpec {
                name: column.name.to_ascii_uppercase(),
                data_type: cast_target(&column.data_type, log)
                    .map_err(|reason| format!("function {}: {reason}", function.name))?
                    .data_type,
            })
        })
        .collect()
}
