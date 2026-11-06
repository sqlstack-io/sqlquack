//! CREATE TYPE of a user-defined type in a case's setup, which later statements resolve to its base type.

use serde_json::Value;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_snowflake::typing::main::cast_target::cast_target;
use sqlquack_syntax::expressions::schema_objects::models::{CreateType, TypeDefinition};

use crate::conversion::_helpers::expressions::casts::data_type_name;
use crate::conversion::_helpers::setup::tables::resolved;
use crate::conversion::_helpers::support::tree::{strip_positions, with_user_types};
use crate::conversion::models::ConvertedSetup;

/// `CREATE TYPE name AS data_type`: the resolved name and the serialised base type it stands for.
pub(crate) fn defined(
    create: &CreateType,
    converted: &ConvertedSetup,
) -> Result<(String, Value), String> {
    let TypeDefinition::Alias { data_type, .. } = &create.definition else {
        return Err("defines a type other than an alias of a data type".to_owned());
    };
    if create.or_replace
        || create.if_not_exists
        || create.name.schema.is_some()
        || create.name.catalog.is_some()
    {
        return Err("has a clause other than an unqualified name AS a data type".to_owned());
    }
    let name = resolved(&create.name.name)?;
    if converted.types.contains_key(&name) {
        return Err(format!("creates the type {name} again"));
    }
    match cast_target(&name, &ProvenanceLog::default()) {
        Ok(_) => {
            return Err(format!(
                "creates the type {name}, which names a built-in type"
            ));
        }
        Err(_unknown) => {}
    }
    let base = serde_json::to_value(data_type)
        .map(|spelled| with_user_types(&strip_positions(&spelled), &converted.types))
        .map_err(|error| format!("defines {name} as a type that does not serialise: {error}"))?;
    data_type_name(&base).ok_or_else(|| format!("defines {name} as a type that has no name"))?;
    Ok((name, base))
}
