//! `identifier_name`: the resolved name of a serialised identifier node under a dialect.

use serde_json::Value;

use crate::dialects::types::DialectProfile;

/// The name `identifier` resolves to in `profile`'s dialect, when it is an identifier node.
pub fn identifier_name(identifier: &Value, profile: &dyn DialectProfile) -> Option<String> {
    let object = identifier.as_object()?;
    let name = object.get("name")?.as_str()?;
    let quoted = object
        .get("quoted")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some(profile.resolved_identifier(name, quoted))
}
