//! The shape of a construct a rule accepted: its construct key and its arguments' families.

use serde_json::Value;
use sqlquack_diagnostics::provenance::models::ShapeArgument;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::call_name;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::tree::{
    LITERAL_NUMBER, LITERAL_STRING, identifier_name, node_kind,
};
use crate::conversion::constants::{CALL_KIND, SHAPE_CALL_KINDS, SHAPE_WRAPPER_KINDS};

/// The construct key a shape records for `node`: `fn:<NAME>` for calls, else `node:<kind>`.
pub(crate) fn shape_construct(node: &Value) -> String {
    match node_kind(node) {
        Some((kind, payload)) if kind == CALL_KIND || SHAPE_CALL_KINDS.contains(&kind) => {
            format!("fn:{}", call_name(payload))
        }
        Some((kind, _)) => format!("node:{kind}"),
        None => "node:unknown".to_owned(),
    }
}

/// `node`, converted to `emitted`, as an argument of its parent's shape.
pub(crate) fn shape_argument(node: &Value, emitted: &Emitted) -> ShapeArgument {
    let source = source(node);
    ShapeArgument {
        family: type_family(emitted.data_type).to_owned(),
        literal: source
            .as_deref()
            .is_some_and(|source| source.starts_with("literal:")),
        source,
    }
}

/// The type family hazards and domains name for a Snowflake type.
pub(crate) fn type_family(data_type: SnowflakeType) -> &'static str {
    match data_type {
        SnowflakeType::Number { scale: 0, .. } => "integer",
        SnowflakeType::Number { .. } => "decimal",
        SnowflakeType::Float => "float",
        SnowflakeType::Varchar => "text",
        SnowflakeType::Boolean => "boolean",
        SnowflakeType::Date => "date",
        SnowflakeType::Time { .. } => "time",
        SnowflakeType::TimestampNtz { .. } => "timestamp_ntz",
        SnowflakeType::TimestampTz { .. } => "timestamp_tz",
        SnowflakeType::TimestampLtz { .. } => "timestamp_ltz",
        SnowflakeType::SemiStructured => "variant",
        SnowflakeType::Null => "null",
    }
}

/// `column:<NAME>` for a column, `literal:<text>` for a literal, through parentheses and signs.
fn source(node: &Value) -> Option<String> {
    let (kind, payload) = node_kind(node)?;
    match kind {
        "column" => identifier_name(payload.get("name")?)
            .map(|name| format!("column:{}", name.to_ascii_uppercase())),
        "literal" => {
            let text = payload.get("value")?.as_str()?;
            match payload.get("literal_type").and_then(Value::as_str) {
                Some(LITERAL_NUMBER) => Some(format!("literal:{text}")),
                Some(LITERAL_STRING) => Some(format!("literal:'{}'", text.replace('\'', "''"))),
                _ => None,
            }
        }
        "boolean" => Some(format!(
            "literal:{}",
            match payload.get("value").and_then(Value::as_bool) {
                Some(true) => "TRUE",
                _ => "FALSE",
            }
        )),
        "null" => Some("literal:NULL".to_owned()),
        "neg" => source(payload.get("this")?).and_then(|inner| {
            inner
                .strip_prefix("literal:")
                .map(|text| format!("literal:-{text}"))
        }),
        _ if SHAPE_WRAPPER_KINDS.contains(&kind) => source(payload.get("this")?),
        _ => None,
    }
}
