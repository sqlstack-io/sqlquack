//! `create_table`: a relation as a DuckDB table, the way converted SQL reads it.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::constants::NAME_SEPARATOR;
use crate::blocks::main::column_type::column_type;
use crate::blocks::main::quoted_identifier::quoted_identifier;
use crate::checking::constants::SEMI_STRUCTURED_HOLDING_TYPE;
use crate::checking::main::quoted_relation::quoted_relation;

/// `CREATE TABLE` for the typed columns, semi-structured as `JSON`, in its name's catalog and schema.
pub fn create_table(name: &str, columns: &[(String, SnowflakeType)]) -> String {
    let log = ProvenanceLog::default();
    let definitions: Vec<String> = columns
        .iter()
        .filter_map(|(column, data_type)| {
            let duckdb = match (column_type(*data_type, &log), data_type) {
                (Ok(duckdb), _) => duckdb,
                (Err(_), SnowflakeType::SemiStructured) => SEMI_STRUCTURED_HOLDING_TYPE.to_owned(),
                (Err(_), _) => return None,
            };
            Some(format!("{} {duckdb}", quoted_identifier(column, &log)))
        })
        .collect();
    let parts: Vec<&str> = name.split(NAME_SEPARATOR).collect();
    let mut prelude = String::new();
    if let [catalog, _, _] = parts.as_slice() {
        let catalog = quoted_identifier(catalog, &log);
        prelude.push_str(&format!("ATTACH IF NOT EXISTS ':memory:' AS {catalog}; "));
    }
    if let [qualifiers @ .., _] = parts.as_slice()
        && !qualifiers.is_empty()
    {
        let schema = quoted_relation(&qualifiers.join(NAME_SEPARATOR), &log);
        prelude.push_str(&format!("CREATE SCHEMA IF NOT EXISTS {schema}; "));
    }
    format!(
        "{prelude}CREATE TABLE {} ({})",
        quoted_relation(name, &log),
        definitions.join(", ")
    )
}
