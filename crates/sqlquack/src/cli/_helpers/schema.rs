//! Reading a schema file: typed relations, and the signatures of project functions.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_rules::conversion::models::{
    ColumnSpec, FunctionLanguage, FunctionReturns, FunctionSpec, Schema,
};
use sqlquack_snowflake::typing::main::cast_target::cast_target;

/// Columns and parameters as the schema file writes them.
type RawColumns = Vec<RawColumn>;

/// A name and Snowflake type pair, or an object that may also mark a VARIANT's typed leaves.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawColumn {
    Pair(String, String),
    Described(RawDescribedColumn),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDescribedColumn {
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    #[serde(default)]
    typed_leaves: bool,
}

/// The schema file: relations and functions, or relations alone as the top-level map.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawSchema {
    Sections(RawSections),
    Relations(BTreeMap<String, RawColumns>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSections {
    #[serde(default)]
    relations: BTreeMap<String, RawColumns>,
    #[serde(default)]
    functions: BTreeMap<String, RawFunction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFunction {
    language: RawLanguage,
    #[serde(default)]
    arguments: RawColumns,
    returns: RawReturns,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawLanguage {
    Sql,
    Python,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawReturns {
    Scalar(String),
    Table { table: RawColumns },
}

/// The schema in `path`; a relation or function using a type SQLQuack does not know is left out.
pub(crate) fn load_schema(path: &Path) -> Result<Schema, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let raw: RawSchema = serde_json::from_str(&text)
        .map_err(|error| format!("{} is not a schema file: {error}", path.display()))?;
    let (relations, functions) = match raw {
        RawSchema::Sections(sections) => (sections.relations, sections.functions),
        RawSchema::Relations(relations) => (relations, BTreeMap::new()),
    };
    let log = ProvenanceLog::default();
    let mut schema = Schema::default();
    for (relation, columns) in relations {
        if let Some(typed) = typed_columns(columns, &log) {
            schema.relations.insert(relation, typed);
        }
    }
    for (name, function) in functions {
        if let Some(typed) = typed_function(function, &log) {
            schema.functions.insert(name, typed);
        }
    }
    Ok(schema)
}

fn typed_function(function: RawFunction, log: &ProvenanceLog) -> Option<FunctionSpec> {
    let returns = match function.returns {
        RawReturns::Scalar(type_name) => match cast_target(&type_name, log) {
            Ok(target) => FunctionReturns::Scalar(target.data_type),
            Err(_) => return None,
        },
        RawReturns::Table { table } => FunctionReturns::Table(typed_columns(table, log)?),
    };
    Some(FunctionSpec {
        language: match function.language {
            RawLanguage::Sql => FunctionLanguage::Sql,
            RawLanguage::Python => FunctionLanguage::Python,
        },
        arguments: typed_columns(function.arguments, log)?,
        returns,
    })
}

fn typed_columns(columns: RawColumns, log: &ProvenanceLog) -> Option<Vec<ColumnSpec>> {
    let mut typed: Vec<ColumnSpec> = Vec::with_capacity(columns.len());
    for column in columns {
        let (name, type_name, typed_leaves) = match column {
            RawColumn::Pair(name, type_name) => (name, type_name, false),
            RawColumn::Described(column) => (column.name, column.type_name, column.typed_leaves),
        };
        match cast_target(&type_name, log) {
            Ok(target) => typed.push(ColumnSpec {
                typed_leaves,
                ..ColumnSpec::new(name, target.data_type)
            }),
            Err(_) => return None,
        }
    }
    Some(typed)
}
