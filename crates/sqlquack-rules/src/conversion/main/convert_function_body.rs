//! `convert_function_body`: DuckDB macro bodies for a SQL project function, converted like models.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_syntax::Expression;

use crate::conversion::_helpers::queries::function_body::{
    check_body, held_types, parameters_joined, refused, require_declared_types, selected_columns,
};
use crate::conversion::_helpers::queries::query::convert_query;
use crate::conversion::_helpers::support::scope::{Context, Scope, ScopeRelation};
use crate::conversion::_helpers::support::tree::strip_positions;
use crate::conversion::constants::PARAMETERS_RELATION;
use crate::conversion::models::{
    ConversionOptions, ConvertedFunction, FunctionLanguage, FunctionReturns, FunctionSpec, Schema,
};

/// The macro body of a SQL function; `query` is its body, a scalar's wrapped as one CAST column.
pub fn convert_function_body(
    query: &Expression,
    spec: &FunctionSpec,
    (schema, options): (&Schema, &ConversionOptions),
    log: ProvenanceLog,
) -> Result<ConvertedFunction, Refusal> {
    if spec.language != FunctionLanguage::Sql {
        return Err(refused("only SQL function bodies convert"));
    }
    let tree = serde_json::to_value(query)
        .map(strip_positions)
        .map_err(|error| refused(&format!("the tree does not serialise: {error}")))?;
    let context = Context {
        log: &log,
        schema,
        options,
        ctes: Vec::new(),
    };
    let parameters = Scope::new(
        vec![ScopeRelation {
            alias: PARAMETERS_RELATION.to_owned(),
            columns: spec.arguments.clone(),
        }],
        None,
    );
    let (sql, columns) = convert_query(&tree, Some(&parameters), &context, true)?;
    require_declared_types(&columns, &spec.returns)?;
    let names: Vec<String> = (1..=spec.arguments.len())
        .map(|index| format!("p{index}"))
        .collect();
    let held = held_types(&spec.arguments, &log)?;
    let joined =
        |values: &[String]| parameters_joined(values, (&spec.arguments, &held), &sql, &log);
    let selected = selected_columns(&columns, &log);
    check_body(
        &format!(
            "SELECT {selected} FROM {}",
            joined(&vec!["NULL".to_owned(); held.len()])
        ),
        schema,
    )?;
    let body = match &spec.returns {
        FunctionReturns::Scalar(_) => format!("(SELECT {selected} FROM {})", joined(&names)),
        FunctionReturns::Table(_) => format!("TABLE SELECT {selected} FROM {}", joined(&names)),
    };
    Ok(ConvertedFunction {
        parameters: names,
        body,
        provenance: log.finish(),
    })
}
