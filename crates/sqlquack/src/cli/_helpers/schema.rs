//! Reading a schema file: typed relations, and the signatures of project functions.

use std::path::Path;

use sqlquack_semantics::types::errors::SchemaError;
use sqlquack_semantics::types::main::parse_schema::parse_schema;
use sqlquack_semantics::types::models::Schema;

/// The schema in `path`; a relation or function using a type SQLQuack does not know is left out.
pub(crate) fn load_schema(path: &Path) -> Result<Schema, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_schema(&text).map_err(|error| match error {
        SchemaError::Shape(error) => format!("{} is not a schema file: {error}", path.display()),
        SchemaError::Declaration(reason) => reason,
    })
}
