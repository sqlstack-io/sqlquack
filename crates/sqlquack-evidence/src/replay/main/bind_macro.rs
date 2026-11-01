//! `bind_macro`: whether DuckDB creates a macro and binds a call of it.

use duckdb::Connection;

/// Each of `statements` run alone, then `call` prepared, in a fresh database; or DuckDB's error.
pub fn bind_macro(statements: &[String], call: &str) -> Result<(), String> {
    let connection = Connection::open_in_memory().map_err(|error| error.to_string())?;
    for statement in statements {
        connection
            .execute(statement, [])
            .map_err(|error| error.to_string())?;
    }
    connection
        .prepare(call)
        .map(|_| ())
        .map_err(|error| error.to_string())
}
