//! Binding printed SQL in DuckDB, when the crate is built with the engine.

#[cfg(feature = "engine")]
use crate::checking::_helpers::engine::with_engine;
#[cfg(feature = "engine")]
use crate::checking::_helpers::tables::{function_definitions, table_definitions};
#[cfg(feature = "engine")]
use crate::checking::constants::PARSER_ERROR;
#[cfg(feature = "engine")]
use crate::checking::errors::CheckStage;
use crate::checking::errors::PrintedSqlFailure;
use crate::checking::types::{FunctionLookup, RelationLookup};

/// DuckDB parses `sql` and binds it over the relations and functions `names` reads.
#[cfg(feature = "engine")]
pub(crate) fn bound(
    sql: &str,
    names: &[String],
    (lookup, functions): (RelationLookup<'_>, FunctionLookup<'_>),
) -> Result<(), PrintedSqlFailure> {
    with_engine(|engine| {
        let mut definitions = table_definitions(names, lookup);
        definitions.extend(function_definitions(names, functions));
        engine.keep_only(&definitions.iter().map(|(name, _)| name.as_str()).collect())?;
        for (name, statement) in &definitions {
            engine.define_table(name, statement)?;
        }
        engine.connection.prepare(sql).map(drop).map_err(|error| {
            let message = error.to_string();
            let stage = match message.starts_with(PARSER_ERROR) {
                true => CheckStage::Parse,
                false => CheckStage::Bind,
            };
            PrintedSqlFailure { stage, message }
        })
    })
}

/// Without the engine the caller binds the SQL itself, so only the lexical checks apply.
#[cfg(not(feature = "engine"))]
pub(crate) fn bound(
    _sql: &str,
    _names: &[String],
    _lookups: (RelationLookup<'_>, FunctionLookup<'_>),
) -> Result<(), PrintedSqlFailure> {
    Ok(())
}
