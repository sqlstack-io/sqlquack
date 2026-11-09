//! The Snowflake type a refused expression would have, where its node alone decides it.

use serde_json::Value;
use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::main::cast_target::cast_target;
use sqlquack_semantics::snowflake::typing::main::count::count;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::snowflake::conversion::_helpers::expressions::casts::data_type_name;
use crate::snowflake::conversion::_helpers::support::tree::node_kind;
use crate::snowflake::conversion::constants::{
    FUNCTION_RESULT_TYPES, PREDICATE_KINDS, TEXT_RESULT_KINDS,
};

/// The type `node` has whatever its operands, or none when they decide it.
pub(crate) fn result_type(node: &Value) -> Option<SnowflakeType> {
    let (kind, payload) = node_kind(node)?;
    match kind {
        "cast" | "try_cast" => {
            let name = data_type_name(payload.get("to")?)?;
            match cast_target(&name, &ProvenanceLog::default()) {
                Ok(target) => Some(target.data_type),
                Err(_) => None,
            }
        }
        "j_s_o_n_extract" | "subscript" => Some(SnowflakeType::SemiStructured),
        "count" => Some(count(&ProvenanceLog::default())),
        "function" | "aggregate_function" => {
            let name = payload.get("name").and_then(Value::as_str)?;
            FUNCTION_RESULT_TYPES
                .iter()
                .find(|(function, _)| function.eq_ignore_ascii_case(name))
                .map(|(_, data_type)| *data_type)
        }
        kind if PREDICATE_KINDS.contains(&kind) => Some(SnowflakeType::Boolean),
        kind if TEXT_RESULT_KINDS.contains(&kind) => Some(SnowflakeType::Varchar),
        _ => None,
    }
}
