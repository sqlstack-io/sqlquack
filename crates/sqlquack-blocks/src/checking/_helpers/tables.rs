//! `CREATE TABLE` statements for the relations a statement reads, and macros for its functions.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::blocks::main::column_type::column_type;
use crate::blocks::main::quoted_identifier::quoted_identifier;
use crate::checking::constants::MACRO_PREFIX;
use crate::checking::constants::SEMI_STRUCTURED_HOLDING_TYPE;
use crate::checking::main::create_table::create_table;
use crate::checking::models::{FunctionStub, StubReturns};
use crate::checking::types::{FunctionLookup, RelationLookup};

/// Each named relation the lookup knows, with the statement that creates it.
pub(crate) fn table_definitions(
    names: &[String],
    lookup: RelationLookup<'_>,
) -> Vec<(String, String)> {
    let mut definitions: Vec<(String, String)> = Vec::new();
    for name in names {
        if let Some(columns) = lookup(name) {
            definitions.push((name.clone(), create_table(name, &columns)));
        }
    }
    definitions
}

/// Each called project function the lookup knows, with a macro of its shape returning NULLs.
pub(crate) fn function_definitions(
    names: &[String],
    lookup: FunctionLookup<'_>,
) -> Vec<(String, String)> {
    let mut definitions: Vec<(String, String)> = Vec::new();
    for name in names {
        if let Some(stub) = lookup(name) {
            definitions.push((name.clone(), create_macro(name, &stub)));
        }
    }
    definitions
}

fn create_macro(name: &str, stub: &FunctionStub) -> String {
    let log = ProvenanceLog::default();
    let parameters: Vec<String> = (1..=stub.arguments)
        .map(|index| format!("p{index}"))
        .collect();
    let body = match &stub.returns {
        StubReturns::Scalar(data_type) => format!("CAST(NULL AS {})", held_type(*data_type)),
        StubReturns::Table(columns) => {
            let projected: Vec<String> = columns
                .iter()
                .map(|(column, data_type)| {
                    format!(
                        "CAST(NULL AS {}) AS {}",
                        held_type(*data_type),
                        quoted_identifier(column, &log)
                    )
                })
                .collect();
            format!("TABLE SELECT {} WHERE false", projected.join(", "))
        }
    };
    format!(
        "{MACRO_PREFIX} {}({}) AS {body}",
        quoted_identifier(name, &log),
        parameters.join(", ")
    )
}

fn held_type(data_type: SnowflakeType) -> String {
    column_type(data_type, &ProvenanceLog::default())
        .unwrap_or_else(|_| SEMI_STRUCTURED_HOLDING_TYPE.to_owned())
}
