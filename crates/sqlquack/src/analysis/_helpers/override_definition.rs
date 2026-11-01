//! User overrides written in SQLBuild's FUNCTION() format, with `replaces` and `dialect duckdb`.

use sha2::{Digest, Sha256};
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_rules::conversion::models::UserOverride;
use sqlquack_semantics::types::models::ColumnSpec;
use sqlquack_snowflake::typing::main::cast_target::cast_target;

use crate::analysis::constants::VARIANT_HOLDING_TYPE;
use crate::analysis::constants::{
    AGGREGATE_OR_WINDOW_NAMES, OVERRIDE_ARGUMENTS, OVERRIDE_DIALECT, OVERRIDE_DUCKDB,
    OVERRIDE_HEADER, OVERRIDE_REPLACES, OVERRIDE_RETURNS, SHA256_HEX_LENGTH,
};
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
    let rest = text.trim_start();
    let rest = rest
        .get(..OVERRIDE_HEADER.len())
        .filter(|head| head.eq_ignore_ascii_case(OVERRIDE_HEADER))
        .map(|_| rest[OVERRIDE_HEADER.len()..].trim_start())
        .ok_or_else(|| failed("does not start with FUNCTION ("))?;
    let rest = rest
        .strip_prefix('(')
        .ok_or_else(|| failed("does not start with FUNCTION ("))?;
    let close = closing(rest).ok_or_else(|| failed("its FUNCTION ( header is not closed"))?;
    let header = &rest[..close];
    let body = rest[close + 1..]
        .trim_start()
        .strip_prefix(';')
        .ok_or_else(|| failed("its header must end with );"))?
        .trim();
    let body = body.trim_end_matches(';').trim_end();
    if body.is_empty() {
        return Err(failed("has no body"));
    }
    let (mut arguments, mut returns, mut replaces, mut dialect) = (None, None, None, None);
    for property in top_level(header)
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
    {
        let (key, value) = property
            .split_once(|character: char| character.is_whitespace() || character == '(')
            .map(|(key, _)| (key, property[key.len()..].trim()))
            .ok_or_else(|| failed(&format!("the property `{property}` has no value")))?;
        match key.to_ascii_lowercase().as_str() {
            OVERRIDE_ARGUMENTS => {
                arguments = Some(parameters(value).map_err(|reason| failed(&reason))?)
            }
            OVERRIDE_RETURNS => {
                returns = Some(
                    cast_target(value, &ProvenanceLog::default())
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
            body: body.to_owned(),
        },
    ))
}

/// `(name TYPE, ...)` as parameters.
fn parameters(value: &str) -> Result<Vec<ColumnSpec>, String> {
    let inner = value
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .ok_or_else(|| "arguments must be written `arguments (name TYPE, ...)`".to_owned())?;
    let mut parameters: Vec<ColumnSpec> = Vec::new();
    for parameter in top_level(inner)
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
    {
        let (name, type_name) = parameter
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("the argument `{parameter}` has no type"))?;
        let identifier = name.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_alphabetic()
                || (index > 0 && character.is_ascii_digit())
        });
        if !identifier {
            return Err(format!(
                "the argument name `{name}` is not a plain identifier"
            ));
        }
        let target = cast_target(type_name.trim(), &ProvenanceLog::default())?;
        parameters.push(ColumnSpec::new(name.to_ascii_uppercase(), target.data_type));
    }
    Ok(parameters)
}

/// The byte offset of the parenthesis closing `text`'s open one, or None.
fn closing(text: &str) -> Option<usize> {
    let mut depth: usize = 1;
    for (offset, character) in text.char_indices() {
        match character {
            '(' => depth += 1,
            ')' if depth == 1 => return Some(offset),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// `text` split at commas outside parentheses.
fn top_level(text: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = Vec::new();
    let (mut depth, mut start) = (0_usize, 0_usize);
    for (offset, character) in text.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&text[start..offset]);
                start = offset + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
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
