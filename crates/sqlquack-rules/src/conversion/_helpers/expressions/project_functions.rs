//! Calls to project functions, typed by the signatures the schema declares.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::constants::NEEDS_DIVERGENCE;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::main::refused_divergence::refused_divergence;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb_blocks::blocks::main::cast::cast;
use sqlquack_duckdb_blocks::blocks::main::column_type::column_type;
use sqlquack_duckdb_blocks::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_duckdb_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_duckdb_blocks::semi_structured::main::json_guarded::json_guarded;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::scope::{Context, Emitted};
use crate::conversion::_helpers::support::site::Site;

use crate::conversion::constants::{
    PROJECT_FUNCTION_CALL, SEMI_STRUCTURED_HOLDING, USER_OVERRIDE_RULE,
};
use crate::conversion::main::override_macro_name::override_macro_name;
use crate::conversion::models::UserOverride;
use sqlquack_policy::policy::constants::{PROJECT_UDF, PYTHON_VALUE_CROSSING, USER_OVERRIDE};
use sqlquack_semantics::types::models::{
    FunctionLanguage, FunctionReturns, FunctionSpec, FunctionTraits,
};

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
    allowed_returns(spec.language, returns).map_err(refuse)?;
    let call = format!("{}({})", quoted_identifier(name, log), arguments.join(", "));
    if spec.language == FunctionLanguage::Sql {
        return Ok(Emitted::new(format!("CAST({call} AS {held})"), returns));
    }
    if spec.traits.vectorized {
        return Err(refuse(format!(
            "the vectorized Python function {name}, whose batched calls no recording shows"
        )));
    }
    rely_on_python(name, construct, site.context)?;
    if returns == SnowflakeType::SemiStructured {
        rely_on(JSON_FIDELITY, construct, site.context)?;
        return Ok(Emitted::new(
            json_guarded(&format!("CAST({call} AS VARCHAR)"), false, log),
            returns,
        ));
    }
    Ok(Emitted::new(format!("CAST({call} AS {held})"), returns))
}

/// A Python call relies on its handler running locally as in Snowflake, and on its values crossing.
fn rely_on_python(name: &str, construct: &Construct, context: &Context<'_>) -> Result<(), Refusal> {
    if !context.options.divergences.contains_key(PROJECT_UDF) {
        return Err(Refusal {
            reason: format!(
                "the Python function {name}, which runs only where a Python runtime registers it,\
                 {NEEDS_DIVERGENCE}{PROJECT_UDF}`, an assumption the caller has not confirmed"
            ),
            ..refused_divergence(construct.clone(), PROJECT_UDF)
        });
    }
    context
        .log
        .divergence(&format!("{PROJECT_UDF}:{name}"), &construct.key());
    rely_on(PYTHON_VALUE_CROSSING, construct, context)
}

/// Relies on `divergence` for a project function, whose construct the policy cannot list by name.
fn rely_on(divergence: &str, construct: &Construct, context: &Context<'_>) -> Result<(), Refusal> {
    if !context.options.divergences.contains_key(divergence) {
        return Err(refused_divergence(construct.clone(), divergence));
    }
    context.log.divergence(divergence, &construct.key());
    Ok(())
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

/// Python handlers return the argument types, or a VARIANT under `variant-json-v1`.
fn allowed_returns(language: FunctionLanguage, data_type: SnowflakeType) -> Result<(), String> {
    match (language, data_type) {
        (FunctionLanguage::Python, SnowflakeType::SemiStructured) => Ok(()),
        (language, data_type) => allowed_crossing(language, data_type),
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

/// A call to a function a user override replaces, its macro called, labelled `user_override:<NAME>`.
pub(crate) fn override_call(
    replacement: &UserOverride,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(USER_OVERRIDE_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let options = &site.context.options;
    let verified = options.verified_overrides.get(&replacement.name) == Some(&replacement.sha256);
    if !verified && !options.divergences.contains_key(USER_OVERRIDE) {
        return Err(refuse(format!(
            "the user override {} of {} with body sha256 {}, which this mode refuses until \
             recorded cases verify that body",
            replacement.name,
            construct.key(),
            replacement.sha256
        )));
    }
    let spec = FunctionSpec {
        language: FunctionLanguage::Sql,
        arguments: replacement.arguments.clone(),
        returns: FunctionReturns::Scalar(replacement.returns),
        traits: FunctionTraits::default(),
    };
    let arguments = typed_arguments(&spec, arguments, construct, site)?;
    let held = held_type(replacement.returns, log).map_err(refuse)?;
    log.divergence(
        &format!("{USER_OVERRIDE}:{}", replacement.name),
        &construct.key(),
    );
    Ok(Emitted::new(
        format!(
            "CAST({}({}) AS {held})",
            quoted_identifier(&override_macro_name(&replacement.name), log),
            arguments.join(", ")
        ),
        replacement.returns,
    ))
}
