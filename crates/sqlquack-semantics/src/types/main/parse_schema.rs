//! `parse_schema`: a schema file's text as typed relations and project function signatures.

use std::collections::BTreeMap;

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::types::_helpers::schema_file::{RawSchema, typed_columns, typed_function};
use crate::types::_helpers::schema_shape::shape_problem;
use crate::types::errors::SchemaError;
use crate::types::models::Schema;

/// The schema `text` declares; a relation or function using a type SQLQuack does not know is left out.
pub fn parse_schema(text: &str) -> Result<Schema, SchemaError> {
    let raw: RawSchema = serde_json::from_str(text).map_err(|error| {
        SchemaError::Shape(match serde_json::from_str::<serde_json::Value>(text) {
            Ok(top) => shape_problem(&top).unwrap_or_else(|| error.to_string()),
            Err(_not_json) => error.to_string(),
        })
    })?;
    let (relations, functions) = match raw {
        RawSchema::Sections(sections) => (sections.relations, sections.functions),
        RawSchema::Relations(relations) => (relations, BTreeMap::new()),
    };
    let log = ProvenanceLog::default();
    let mut schema = Schema::default();
    for (relation, columns) in relations {
        if let Some(typed) =
            typed_columns(columns, Some(&relation), &log).map_err(SchemaError::Declaration)?
        {
            schema.relations.insert(relation, typed);
        }
    }
    for (name, function) in functions {
        if let Some(typed) = typed_function(function, &log).map_err(SchemaError::Declaration)? {
            schema.functions.insert(name, typed);
        }
    }
    Ok(schema)
}
