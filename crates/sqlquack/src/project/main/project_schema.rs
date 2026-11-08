//! `project_schema`: the tables and functions a project's folders declare.

use std::path::Path;

use serde_json::json;
use sqlquack_policy::policy::models::ProjectPolicy;
use sqlquack_semantics::types::errors::SchemaError;
use sqlquack_semantics::types::main::parse_schema::parse_schema;
use sqlquack_semantics::types::models::Schema;

use crate::project::_helpers::declarations::{
    declaration, no_clash, project_functions, source_tables,
};
use crate::project::constants::{FUNCTION_KIND, GIVEN_SCHEMA, TABLE_KIND};

/// The project's schema merged with `given`, a caller's in memory, and warnings; a name declared twice is an error.
pub fn project_schema(
    root: &Path,
    project: &ProjectPolicy,
    given: Option<&Schema>,
) -> Result<(Schema, Vec<String>), String> {
    let ((tables, mut declarations), warnings) = source_tables(root, project)?;
    let (functions, function_declarations) = project_functions(root, project)?;
    declarations.extend(function_declarations);
    let declared = json!({ "relations": tables, "functions": functions }).to_string();
    let mut schema = parse_schema(&declared).map_err(|error| match error {
        SchemaError::Shape(reason) | SchemaError::Declaration(reason) => {
            format!("the project at {}: {reason}", root.display())
        }
    })?;
    if let Some(given) = given {
        let place = Path::new(GIVEN_SCHEMA);
        declarations.extend(
            given
                .relations
                .keys()
                .map(|name| declaration(TABLE_KIND, name, place)),
        );
        declarations.extend(
            given
                .functions
                .keys()
                .map(|name| declaration(FUNCTION_KIND, name, place)),
        );
        schema.relations.extend(given.relations.clone());
        schema.functions.extend(given.functions.clone());
    }
    no_clash(&declarations)?;
    Ok((schema, warnings))
}
