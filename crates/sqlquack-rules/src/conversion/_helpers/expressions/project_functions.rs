//! Calls to project functions, typed by the signatures the schema declares.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_duckdb::blocks::main::column_type::column_type;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::outside_conditions;
use crate::conversion::constants::{PROJECT_FUNCTION_CALL, SEMI_STRUCTURED_HOLDING};
use crate::conversion::models::{FunctionLanguage, FunctionReturns, FunctionSpec};

/// A scalar project function call: arguments coerced to the parameters, the result to its type.
pub(crate) fn project_call(
    (name, spec): (&str, &FunctionSpec),
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(PROJECT_FUNCTION_CALL);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let FunctionReturns::Scalar(returns) = spec.returns else {
        return Err(refuse("a table function called as a scalar".to_owned()));
    };
    let arguments = typed_arguments(spec, arguments, construct, site)?;
    let held = held_type(returns, log).map_err(refuse)?;
    allowed_crossing(spec.language, returns).map_err(refuse)?;
    Ok(Emitted::new(
        format!(
            "CAST({}({}) AS {held})",
            quoted_identifier(name, log),
            arguments.join(", ")
        ),
        returns,
    ))
}

/// A table function call's arguments, each coerced to its parameter's type.
pub(crate) fn typed_arguments(
    spec: &FunctionSpec,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Vec<String>, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    if arguments.len() != spec.arguments.len() {
        return Err(refuse(format!(
            "{} arguments to a function declaring {}",
            arguments.len(),
            spec.arguments.len()
        )));
    }
    let mut typed: Vec<String> = Vec::with_capacity(arguments.len());
    for (argument, parameter) in arguments.iter().zip(&spec.arguments) {
        allowed_crossing(spec.language, parameter.data_type).map_err(refuse)?;
        let emitted = convert_expression(argument, site)?;
        typed.push(coerced(&emitted, parameter.data_type, site.context.log).map_err(refuse)?);
    }
    Ok(typed)
}

/// A table function's arguments, each of exactly its parameter's type.
pub(crate) fn exact_arguments(
    spec: &FunctionSpec,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Vec<String>, Refusal> {
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    if arguments.len() != spec.arguments.len() {
        return Err(refuse(format!(
            "{} arguments to a function declaring {}",
            arguments.len(),
            spec.arguments.len()
        )));
    }
    let mut exact: Vec<String> = Vec::with_capacity(arguments.len());
    for (argument, parameter) in arguments.iter().zip(&spec.arguments) {
        allowed_crossing(spec.language, parameter.data_type).map_err(refuse)?;
        let emitted = convert_expression(argument, site)?;
        if emitted.data_type != parameter.data_type {
            return Err(refuse(format!(
                "a {} argument to a {} table-function parameter, which Snowflake rejects (error 1044)",
                emitted.data_type, parameter.data_type
            )));
        }
        exact.push(emitted.sql);
    }
    Ok(exact)
}

/// The DuckDB type a value of `data_type` is held in, semi-structured values as JSON.
pub(crate) fn held_type(data_type: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
    match data_type {
        SnowflakeType::SemiStructured => Ok(SEMI_STRUCTURED_HOLDING.to_owned()),
        other => column_type(other, log),
    }
}

/// An argument as its parameter's type: unchanged, or a numeric or NULL cast.
fn coerced(emitted: &Emitted, to: SnowflakeType, log: &ProvenanceLog) -> Result<String, String> {
    match (emitted.data_type, to) {
        (from, to) if from == to => Ok(emitted.sql.clone()),
        (SnowflakeType::Null, to) => Ok(format!("CAST(NULL AS {})", held_type(to, log)?)),
        (SnowflakeType::Number { .. }, SnowflakeType::Number { .. } | SnowflakeType::Float) => {
            cast(&emitted.sql, to, log)
        }
        (from, to) => Err(format!(
            "a {from} argument to a {to} parameter, whose implicit coercion no recording shows"
        )),
    }
}

/// Python handlers take and return only the types recordings will prove crossing intact.
fn allowed_crossing(language: FunctionLanguage, data_type: SnowflakeType) -> Result<(), String> {
    match (language, data_type) {
        (FunctionLanguage::Sql, _)
        | (
            FunctionLanguage::Python,
            SnowflakeType::Varchar
            | SnowflakeType::Number { .. }
            | SnowflakeType::Float
            | SnowflakeType::Boolean,
        ) => Ok(()),
        (FunctionLanguage::Python, other) => Err(format!(
            "a Python function passing {other}, whose crossing no recording shows"
        )),
    }
}
