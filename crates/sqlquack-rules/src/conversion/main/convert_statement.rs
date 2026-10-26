//! `convert_statement`: DuckDB SQL for one canonical Snowflake query, or the first refusal.

use sqlquack_core::provenance::models::ProvenanceLog;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::quoted_identifier::quoted_identifier;
use sqlquack_duckdb::checking::main::check_printed::check_printed;
use sqlquack_duckdb::semi_structured::main::json_output::json_output;
use sqlquack_snowflake::typing::models::SnowflakeType;
use sqlquack_syntax::Expression;

use crate::conversion::_helpers::queries::function_body::{column_types, function_stub};
use crate::conversion::_helpers::queries::query::convert_query;
use crate::conversion::_helpers::support::scope::Context;
use crate::conversion::_helpers::support::tree::{missing, node_construct, strip_positions};
use crate::conversion::constants::OUTPUT_ALIAS;
use crate::conversion::models::{ColumnSpec, ConversionOptions, Converted, Schema};

/// Converts a statement over the schema's relations, recording its rules; DuckDB must bind it.
pub fn convert_statement(
    statement: &Expression,
    (schema, options): (&Schema, &ConversionOptions),
    log: ProvenanceLog,
) -> Result<Converted, Refusal> {
    let tree = serde_json::to_value(statement).map_err(|error| {
        missing(
            node_construct("statement"),
            format!("the tree does not serialise: {error}"),
        )
    })?;
    let tree = strip_positions(tree);
    let context = Context {
        log: &log,
        schema,
        options,
        ctes: Vec::new(),
    };
    let (sql, columns) = convert_query(&tree, None, &context, true)?;
    let sql = output_variants(sql, &columns, &log);
    let relation = |name: &str| -> Option<Vec<(String, SnowflakeType)>> {
        let columns = schema.relations.get(name)?;
        Some(column_types(columns))
    };
    let function = |name: &str| function_stub(schema, name);
    check_printed(&sql, (&relation, &function)).map_err(|failure| {
        missing(
            node_construct("statement"),
            format!("the emitted DuckDB SQL fails DuckDB's check: {failure}"),
        )
    })?;
    Ok(Converted {
        sql,
        columns,
        provenance: log.finish(),
    })
}

/// The query with each VARIANT result column written as Snowflake returns it.
fn output_variants(sql: String, columns: &[ColumnSpec], log: &ProvenanceLog) -> String {
    let replaced: Vec<String> = columns
        .iter()
        .filter(|column| column.data_type == SnowflakeType::SemiStructured)
        .map(|column| {
            let name = quoted_identifier(&column.name, log);
            format!("{} AS {name}", json_output(&name, log))
        })
        .collect();
    match replaced.is_empty() {
        true => sql,
        false => format!(
            "SELECT * REPLACE ({}) FROM ({sql}) AS {OUTPUT_ALIAS}",
            replaced.join(", ")
        ),
    }
}
