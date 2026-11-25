use crate::macros::main::all_helper_statements::all_helper_statements;

/// Every helper macro, created on a fresh DuckDB connection.
pub(crate) fn helpers() -> duckdb::Connection {
    let connection = duckdb::Connection::open_in_memory().expect("DuckDB opens");
    for statement in all_helper_statements() {
        connection
            .execute_batch(&statement)
            .expect("the helper is created");
    }
    connection
}

/// What a helper wrote, or the tagged message it raised.
pub(crate) type Outcome = Result<Option<String>, String>;

/// Each outcome with borrowed text, to compare with expected values.
pub(crate) fn shown(outcomes: &[Outcome]) -> Vec<Result<Option<&str>, &str>> {
    outcomes
        .iter()
        .map(|outcome| {
            outcome
                .as_ref()
                .map(Option::as_deref)
                .map_err(String::as_str)
        })
        .collect()
}

/// What the helper `name` writes for the JSON text `value`, or the tagged message it raises.
pub(crate) fn written(
    connection: &duckdb::Connection,
    (name, value): (&str, Option<&str>),
) -> Outcome {
    connection
        .query_row(
            &format!("SELECT sqlquack.{name}('#s', CAST(? AS JSON))"),
            [value],
            |row| row.get::<_, Option<String>>(0),
        )
        .map_err(|error| {
            let message = error.to_string();
            message
                .split_once("string '")
                .and_then(|(_, rest)| rest.split_once("' to "))
                .map_or(message.clone(), |(tagged, _)| tagged.to_owned())
        })
}

/// How many times the definition of the helper `name` calls `callee`.
pub(crate) fn calls_in_definition(name: &str, callee: &str) -> Option<usize> {
    let opening = format!("CREATE OR REPLACE MACRO sqlquack.{name}(");
    all_helper_statements()
        .into_iter()
        .find(|statement| statement.starts_with(&opening))
        .map(|statement| statement.matches(&format!("sqlquack.{callee}(")).count())
}
