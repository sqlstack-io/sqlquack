//! The in-memory DuckDB engine each thread checks with, and the objects created in it.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::mem::ManuallyDrop;

use duckdb::Connection;
use sqlquack_core::provenance::models::ProvenanceLog;

use crate::blocks::main::quoted_identifier::quoted_identifier;
use crate::checking::main::quoted_relation::quoted_relation;

use crate::checking::constants::{DISABLE_OPTIMIZER, MACRO_PREFIX};
use crate::checking::errors::{CheckStage, PrintedSqlFailure};
use crate::macros::main::all_helper_statements::all_helper_statements;

/// A DuckDB connection and the definition of each table created in it.
pub(crate) struct Engine {
    /// The connection.
    pub(crate) connection: Connection,
    /// Table name to the `CREATE TABLE` statement that made it.
    pub(crate) tables: BTreeMap<String, String>,
}

impl Engine {
    /// Creates table `name` with `statement`, dropping only a table it made earlier under that name.
    pub(crate) fn define_table(
        &mut self,
        name: &str,
        statement: &str,
    ) -> Result<(), PrintedSqlFailure> {
        if self.tables.get(name).map(String::as_str) == Some(statement) {
            return Ok(());
        }
        let log = ProvenanceLog::default();
        let drop = match (
            statement.starts_with(MACRO_PREFIX),
            self.tables.contains_key(name),
        ) {
            (true, _) => format!("DROP TABLE IF EXISTS {}", quoted_identifier(name, &log)),
            (false, true) => format!("DROP TABLE IF EXISTS {}", quoted_relation(name, &log)),
            (false, false) => String::new(),
        };
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

/// Drops every object a check created that `kept` does not name, so a check sees only its own.
impl Engine {
    pub(crate) fn keep_only(&mut self, kept: &BTreeSet<&str>) -> Result<(), PrintedSqlFailure> {
        let stale: Vec<(String, String)> = self
            .tables
            .iter()
            .filter(|(name, _)| !kept.contains(name.as_str()))
            .map(|(name, statement)| (name.clone(), statement.clone()))
            .collect();
        for (name, statement) in stale {
            let log = ProvenanceLog::default();
            let drop = match statement.starts_with(MACRO_PREFIX) {
                true => {
                    let quoted = quoted_identifier(&name, &log);
                    format!("DROP MACRO TABLE IF EXISTS {quoted}; DROP MACRO IF EXISTS {quoted}")
                }
                false => format!("DROP TABLE IF EXISTS {}", quoted_relation(&name, &log)),
            };
            self.connection
                .execute_batch(&drop)
                .map_err(|error| PrintedSqlFailure {
                    stage: CheckStage::Engine,
                    message: format!("cannot drop {name}: {error}"),
                })?;
            self.tables.remove(&name);
        }
        Ok(())
    }
}

thread_local! {
    /// The thread's engine, reused and never closed: closing one during process exit killed processes.
    static ENGINE: ManuallyDrop<RefCell<Option<Engine>>> =
        const { ManuallyDrop::new(RefCell::new(None)) };
}

/// Runs `check` with the thread's in-memory engine; an engine a check failed in is discarded.
pub(crate) fn with_engine<T>(
    check: impl FnOnce(&mut Engine) -> Result<T, PrintedSqlFailure>,
) -> Result<T, PrintedSqlFailure> {
    let mut engine = match ENGINE.with(|cell| cell.borrow_mut().take()) {
        Some(engine) => engine,
        None => Engine {
            connection: binding_connection()?,
            tables: BTreeMap::new(),
        },
    };
    let result = check(&mut engine);
    let healthy = !matches!(&result, Err(failure) if failure.stage == CheckStage::Engine);
    if healthy {
        ENGINE.with(|cell| *cell.borrow_mut() = Some(engine));
    }
    result
}

/// An in-memory connection that binds without optimising, holding every helper macro.
fn binding_connection() -> Result<Connection, PrintedSqlFailure> {
    let engine_failure = |error: duckdb::Error| PrintedSqlFailure {
        stage: CheckStage::Engine,
        message: error.to_string(),
    };
    let connection = Connection::open_in_memory().map_err(engine_failure)?;
    connection
        .execute_batch(DISABLE_OPTIMIZER)
        .map_err(engine_failure)?;
    for statement in all_helper_statements() {
        connection
            .execute_batch(&statement)
            .map_err(engine_failure)?;
    }
    Ok(connection)
}
