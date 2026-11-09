//! The DuckDB session a query runs in.

use duckdb::Connection;
use sqlquack_policy::policy::constants::SESSION_TIME_ZONE;
use sqlquack_policy::policy::models::{NullOrder, Policy};

/// A fresh in-memory DuckDB in the profile's time zone and null order, with `macros` created.
pub(crate) fn session(
    policy: Option<&Policy>,
    profile: &str,
    macros: &[String],
) -> Result<Connection, String> {
    let (time_zone, null_order) = match policy {
        Some(policy) => (
            policy
                .session_profiles
                .get(profile)
                .map_or(SESSION_TIME_ZONE, |profile| &profile.duckdb_time_zone),
            policy.comparator.null_order,
        ),
        None => (SESSION_TIME_ZONE, NullOrder::NullsLastOnAscFirstOnDesc),
    };
    let connection = Connection::open_in_memory().map_err(duckdb_error)?;
    connection
        .execute_batch(&format!(
            "SET default_null_order = '{}'; SET threads = 1;",
            null_order.duckdb_setting()
        ))
        .map_err(duckdb_error)?;
    let zones: bool = connection
        .query_row(
            "SELECT count(*) > 0 FROM duckdb_extensions() WHERE extension_name = 'icu' AND loaded",
            [],
            |row| row.get(0),
        )
        .map_err(duckdb_error)?;
    match (zones, time_zone == SESSION_TIME_ZONE) {
        (true, _) => connection
            .execute_batch(&format!("SET TimeZone = {};", literal(time_zone)))
            .map_err(duckdb_error)?,
        (false, true) => {}
        (false, false) => {
            return Err(format!(
                "the session profile runs in {time_zone}, and this DuckDB knows only UTC"
            ));
        }
    }
    for statement in macros {
        connection.execute_batch(statement).map_err(duckdb_error)?;
    }
    Ok(connection)
}

/// `text` as a SQL string literal.
pub(crate) fn literal(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// A DuckDB error as text.
pub(crate) fn duckdb_error(error: duckdb::Error) -> String {
    error.to_string()
}
