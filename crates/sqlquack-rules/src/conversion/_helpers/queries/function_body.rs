//! Pieces of a SQL function body's conversion: type checks, the parameters join and DuckDB's check.

use crate::conversion::_helpers::support::typed_leaves::checked;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_duckdb::checking::main::check_printed::check_printed;
use sqlquack_duckdb::checking::models::{FunctionStub, StubReturns};
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::project_functions::held_type;
use crate::conversion::_helpers::queries::query::convert_query;
use crate::conversion::_helpers::support::scope::{Context, Scope};
use crate::conversion::_helpers::support::tree::node_construct;
use crate::conversion::constants::{
    BODY_ALIAS, NO_PARAMETERS_COLUMN, PARAMETERS_RELATION, VARIANT_HELD_UNREAD,
};
use sqlquack_semantics::scopes::models::ScopeRelation;
use sqlquack_semantics::types::models::{ColumnSpec, FunctionReturns, FunctionSpec, Schema};

/// The body `tree` over the function's parameters, checked for typed leaves compared by text.
pub(crate) fn converted_body(
    tree: &serde_json::Value,
    spec: &FunctionSpec,
    context: &Context<'_>,
) -> Result<(String, Vec<ColumnSpec>), Refusal> {
    let parameters = Scope::new(
        vec![ScopeRelation {
            alias: PARAMETERS_RELATION.to_owned(),
            columns: spec.arguments.clone(),
            merged: Vec::new(),
        }],
        None,
    );
    checked(
        context.typed,
        convert_query(tree, Some(&parameters), context, true),
    )
}

/// Refuses a body whose columns differ from the declared types or are VARIANT-held.
pub(crate) fn require_declared_types(
    columns: &[ColumnSpec],
    returns: &FunctionReturns,
) -> Result<(), Refusal> {
    let expected: Vec<SnowflakeType> = match returns {
        FunctionReturns::Scalar(data_type) => vec![*data_type],
        FunctionReturns::Table(declared) => {
            declared.iter().map(|column| column.data_type).collect()
        }
    };
    if columns.iter().any(|column| !column.holding.is_json()) {
        return Err(refused(VARIANT_HELD_UNREAD));
    }
    let found: Vec<SnowflakeType> = columns.iter().map(|column| column.data_type).collect();
    match found == expected {
        true => Ok(()),
        false => Err(refused("the body's columns are not of the declared types")),
    }
}

/// The parameters' values, cast to their types, joined LATERAL to the converted body.
pub(crate) fn parameters_joined(
    values: &[String],
    (parameters, held): (&[ColumnSpec], &[String]),
    sql: &str,
    log: &ProvenanceLog,
) -> String {
    let mut projected: Vec<String> = values
        .iter()
        .zip(parameters.iter().zip(held))
        .map(|(value, (parameter, held))| {
            format!(
                "CAST({value} AS {held}) AS {}",
                quoted_identifier(&parameter.name, log)
            )
        })
        .collect();
    if projected.is_empty() {
        projected.push(format!("NULL AS {NO_PARAMETERS_COLUMN}"));
    }
    format!(
        "(SELECT {}) AS {} CROSS JOIN LATERAL ({sql}) AS {BODY_ALIAS}",
        projected.join(", "),
        quoted_identifier(PARAMETERS_RELATION, log)
    )
}

/// DuckDB parses and binds the body over NULL parameters and the schema's relations.
pub(crate) fn check_body(checked: &str, schema: &Schema) -> Result<(), Refusal> {
    let relation = |name: &str| -> Option<Vec<(String, SnowflakeType)>> {
        Some(
            schema
                .relations
                .get(name)?
                .iter()
                .map(|column| (column.name.clone(), column.data_type))
                .collect(),
        )
    };
    check_printed(checked, (&relation, &|name| function_stub(schema, name))).map_err(|failure| {
        missing(
            node_construct("function_body"),
            format!("the emitted DuckDB SQL fails DuckDB's check: {failure}"),
        )
    })
}

/// The stub DuckDB's check binds a call to the project function `name` with, matched ignoring case.
pub(crate) fn function_stub(schema: &Schema, name: &str) -> Option<FunctionStub> {
    let (_, spec) = schema
        .functions
        .iter()
        .find(|(declared, _)| declared.eq_ignore_ascii_case(name))?;
    Some(FunctionStub {
        arguments: spec.arguments.len(),
        returns: match &spec.returns {
            FunctionReturns::Scalar(data_type) => StubReturns::Scalar(*data_type),
            FunctionReturns::Table(columns) => StubReturns::Table(column_types(columns)),
        },
    })
}

/// Each column's name and type.
pub(crate) fn column_types(columns: &[ColumnSpec]) -> Vec<(String, SnowflakeType)> {
    columns
        .iter()
        .map(|column| (column.name.clone(), column.data_type))
        .collect()
}

pub(crate) fn selected_columns(columns: &[ColumnSpec], log: &ProvenanceLog) -> String {
    columns
        .iter()
        .map(|column| format!("{BODY_ALIAS}.{}", quoted_identifier(&column.name, log)))
        .collect::<Vec<String>>()
        .join(", ")
}

pub(crate) fn held_types(
    parameters: &[ColumnSpec],
    log: &ProvenanceLog,
) -> Result<Vec<String>, Refusal> {
    parameters
        .iter()
        .map(|parameter| held_type(parameter.data_type, log).map_err(|reason| refused(&reason)))
        .collect()
}

pub(crate) fn refused(reason: &str) -> Refusal {
    outside_conditions(node_construct("function_body"), reason.to_owned())
}
