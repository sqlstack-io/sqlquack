//! `convert_function_body`: DuckDB macro bodies for a SQL project function, converted like models.

use sqlquack_core::parsing::models::SourceLines;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_syntax::Expression;

use crate::conversion::_helpers::queries::function_body::{
    check_body, converted_body, held_types, parameters_joined, refused, require_declared_types,
    selected_columns,
};
use crate::conversion::_helpers::support::locations::Locator;
use crate::conversion::_helpers::support::scope::Context;
use crate::conversion::_helpers::support::tree::strip_positions;
use crate::conversion::_helpers::support::typed_leaves::TypedLeaves;
use crate::conversion::models::{
    ConversionOptions, ConvertedFunction, FunctionLanguage, FunctionReturns, FunctionSpec, Schema,
};

/// The macro body of a SQL function; `query` is its body, a scalar's wrapped as one CAST column.
pub fn convert_function_body(
    query: &Expression,
    spec: &FunctionSpec,
    (schema, options, lines): (&Schema, &ConversionOptions, &SourceLines),
    log: ProvenanceLog,
) -> Result<ConvertedFunction, Refusal> {
    if spec.language != FunctionLanguage::Sql {
        return Err(refused("only SQL function bodies convert"));
    }
    let positioned = serde_json::to_value(query)
        .map_err(|error| refused(&format!("the tree does not serialise: {error}")))?;
    let tree = strip_positions(&positioned);
    let locator = Locator::new(&tree, &positioned, lines);
    let typed = TypedLeaves::default();
    let context = Context {
        log: &log,
        schema,
        options,
        ctes: Vec::new(),
        locator: &locator,
        typed: &typed,
    };
    let (sql, columns) =
        converted_body(&tree, spec, &context).map_err(|refusal| locator.placed(refusal, &tree))?;
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
    let (body, raises) = locator.finished(&body, &tree);
    Ok(ConvertedFunction {
        raises,
        parameters: names,
        body,
        provenance: log.finish(),
    })
}
