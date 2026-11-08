//! User overrides written in SQLBuild's FUNCTION() format, with `replaces` and `dialect duckdb`.

use sha2::{Digest, Sha256};
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_rules::conversion::models::UserOverride;
use sqlquack_semantics::types::models::ColumnSpec;
use sqlquack_snowflake::typing::main::cast_target::cast_target;

use crate::analysis::constants::VARIANT_HOLDING_TYPE;
use crate::analysis::constants::{
    AGGREGATE_OR_WINDOW_NAMES, OVERRIDE_ARGUMENTS, OVERRIDE_DIALECT, OVERRIDE_DUCKDB,
    OVERRIDE_REPLACES, OVERRIDE_RETURNS, SHA256_HEX_LENGTH,
};
use crate::project::main::function_header::function_header;
use crate::project::main::named_types::named_types;
use sqlquack_duckdb::blocks::main::column_type::column_type;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_evidence::replay::main::bind_macro::bind_macro;
use sqlquack_evidence::replay::main::single_expression::single_expression;
use sqlquack_rules::conversion::main::override_macro::override_macro;
use sqlquack_rules::conversion::main::override_macro_name::override_macro_name;
use sqlquack_snowflake::typing::models::SnowflakeType;

/// The function the override named `name` replaces and the override, once DuckDB binds a call.
pub(crate) fn load_override(name: &str, text: &str) -> Result<(String, UserOverride), String> {
    let (replaces, replacement) = override_definition(name, text)?;
    single_expression(&replacement.body)
        .map_err(|reason| format!("override {name}: its body is {reason}"))?;
    let log = ProvenanceLog::default();
    let mut columns: Vec<String> = Vec::new();
    for argument in &replacement.arguments {
        let held = match argument.data_type {
            SnowflakeType::SemiStructured => VARIANT_HOLDING_TYPE.to_owned(),
            other => {
                column_type(other, &log).map_err(|reason| format!("override {name}: {reason}"))?
            }
        };
        columns.push(format!("p{} {held}", columns.len()));
    }
    let names: Vec<String> = (0..columns.len())
        .map(|index| format!("p{index}"))
        .collect();
    let mut statements: Vec<String> = vec![override_macro(&replacement)];
    let from = match columns.is_empty() {
        true => "",
        false => {
            statements.push(format!("CREATE TABLE parameters ({})", columns.join(", ")));
            " FROM parameters"
        }
    };
    let invocation = format!(
        "{}({})",
        quoted_identifier(&override_macro_name(&replacement.name), &log),
        names.join(", ")
    );
    let call =
        format!("SELECT {invocation}{from} WHERE {invocation} IS NOT DISTINCT FROM {invocation}");
    bind_macro(&statements, &call).map_err(|error| {
        format!("override {name}: DuckDB does not bind its SQL as a per-row expression: {error}")
    })?;
    Ok((replaces, replacement))
}

/// The function `text` replaces, upper case, and the override named `name`, or why it is not one.
fn override_definition(name: &str, text: &str) -> Result<(String, UserOverride), String> {
    let failed = |reason: &str| format!("override {name}: {reason}");
    let (properties, body) = function_header(text).map_err(|reason| failed(&reason))?;
    let (mut arguments, mut returns, mut replaces, mut dialect) = (None, None, None, None);
    for (key, value) in properties {
        match key.as_str() {
            OVERRIDE_ARGUMENTS => {
                arguments = Some(parameters(&value).map_err(|reason| failed(&reason))?)
            }
            OVERRIDE_RETURNS => {
                returns = Some(
                    cast_target(&value, &ProvenanceLog::default())
                        .map_err(|reason| failed(&reason))?
                        .data_type,
                );
            }
            OVERRIDE_REPLACES => replaces = Some(value.to_ascii_uppercase()),
            OVERRIDE_DIALECT => dialect = Some(value.to_ascii_lowercase()),
            other => return Err(failed(&format!("the property `{other}` is not understood"))),
        }
    }
    if dialect.as_deref() != Some(OVERRIDE_DUCKDB) {
        return Err(failed("needs `dialect duckdb`"));
    }
    let replaces = replaces.ok_or_else(|| failed("needs `replaces <FUNCTION>`"))?;
    if AGGREGATE_OR_WINDOW_NAMES.contains(&replaces.as_str()) {
        return Err(failed(&format!(
            "replaces {replaces}, an aggregate or window function; only scalar functions can be \
             overridden"
        )));
    }
    Ok((
        replaces,
        UserOverride {
            name: name.to_ascii_uppercase(),
            arguments: arguments.unwrap_or_default(),
            returns: returns.ok_or_else(|| failed("needs `returns <TYPE>`"))?,
            sha256: Sha256::digest(body.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
            body,
        },
    ))
}

/// `(name TYPE, ...)` as parameters.
fn parameters(value: &str) -> Result<Vec<ColumnSpec>, String> {
    let mut parameters: Vec<ColumnSpec> = Vec::new();
    for (name, type_name) in named_types(value)? {
        let target = cast_target(&type_name, &ProvenanceLog::default())?;
        parameters.push(ColumnSpec::new(name.to_ascii_uppercase(), target.data_type));
    }
    Ok(parameters)
}

/// The upper-case name and lower-case hash of a `NAME=<sha256>` verification, or why it is not one.
pub(crate) fn verified_override(named: &str) -> Result<(String, String), String> {
    let (name, sha256) = named
        .split_once('=')
        .filter(|(_, sha256)| is_sha256(sha256))
        .ok_or_else(|| {
            format!("--verified-override {named}: write NAME=<sha256 of the verified body>")
        })?;
    Ok((name.to_ascii_uppercase(), sha256.to_ascii_lowercase()))
}

/// Whether `text` is a SHA-256 in hex.
fn is_sha256(text: &str) -> bool {
    text.len() == SHA256_HEX_LENGTH && text.chars().all(|character| character.is_ascii_hexdigit())
}
