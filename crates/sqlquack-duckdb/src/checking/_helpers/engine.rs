//! An in-memory DuckDB engine for one check, and the tables created in it.

use std::collections::BTreeMap;

use duckdb::Connection;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::main::quoted_identifier::quoted_identifier;

use crate::checking::errors::{CheckStage, PrintedSqlFailure};

/// A DuckDB connection and the definition of each table created in it.
pub(crate) struct Engine {
    /// The connection.
    pub(crate) connection: Connection,
    /// Table name to the `CREATE TABLE` statement that made it.
    pub(crate) tables: BTreeMap<String, String>,
}

impl Engine {
    /// Creates table `name` with `statement`, replacing a table created with other columns.
    pub(crate) fn define_table(
        &mut self,
        name: &str,
        statement: &str,
    ) -> Result<(), PrintedSqlFailure> {
        if self.tables.get(name).map(String::as_str) == Some(statement) {
            return Ok(());
        }
        let drop = format!(
            "DROP TABLE IF EXISTS {}",
            quoted_identifier(name, &ProvenanceLog::default())
        );
        self.connection
            .execute_batch(&format!("{drop}; {statement}"))
            .map_err(|error| PrintedSqlFailure {
                stage: CheckStage::Engine,
                message: format!("cannot create table {name}: {error}"),
            })?;
        self.tables.insert(name.to_owned(), statement.to_owned());
        Ok(())
    }
}

/// Runs `check` with a new in-memory engine, closed before this returns.
pub(crate) fn with_engine<T>(
    check: impl FnOnce(&mut Engine) -> Result<T, PrintedSqlFailure>,
) -> Result<T, PrintedSqlFailure> {
    let connection = Connection::open_in_memory().map_err(|error| PrintedSqlFailure {
        stage: CheckStage::Engine,
        message: error.to_string(),
    })?;
    let mut engine = Engine {
        connection,
        tables: BTreeMap::new(),
    };
    check(&mut engine)
}
