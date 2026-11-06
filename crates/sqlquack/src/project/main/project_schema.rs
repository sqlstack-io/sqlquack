//! `project_schema`: the tables and functions a project's folders declare, with a schema file's.

use std::path::Path;

use serde_json::json;
use sqlquack_policy::policy::models::ProjectPolicy;
use sqlquack_semantics::types::errors::SchemaError;
use sqlquack_semantics::types::main::parse_schema::parse_schema;
use sqlquack_semantics::types::models::Schema;

use crate::project::_helpers::declarations::{
    declaration, no_clash, project_functions, source_tables,
};
use crate::project::constants::{FUNCTION_KIND, TABLE_KIND};

/// The schema of the project at `root` merged with `file`'s; one name declared twice is an error.
pub fn project_schema(
    root: &Path,
    project: &ProjectPolicy,
    file: Option<(&Path, Schema)>,
) -> Result<Schema, String> {
    let (tables, mut declarations) = source_tables(root, project)?;
    let (functions, function_declarations) = project_functions(root, project)?;
    declarations.extend(function_declarations);
    let declared = json!({ "relations": tables, "functions": functions }).to_string();
    let mut schema = parse_schema(&declared).map_err(|error| match error {
        SchemaError::Shape(reason) | SchemaError::Declaration(reason) => {
            format!("the project at {}: {reason}", root.display())
        }
    })?;
    if let Some((path, file)) = file {
        declarations.extend(
            file.relations
                .keys()
                .map(|name| declaration(TABLE_KIND, name, path)),
        );
        declarations.extend(
            file.functions
                .keys()
                .map(|name| declaration(FUNCTION_KIND, name, path)),
        );
        schema.relations.extend(file.relations);
        schema.functions.extend(file.functions);
    }
    no_clash(&declarations)?;
    Ok(schema)
}
