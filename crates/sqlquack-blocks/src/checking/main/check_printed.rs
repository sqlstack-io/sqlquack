//! `check_printed`: DuckDB parses printed SQL as one statement and binds it.

use crate::checking::_helpers::binding::bound;
use crate::checking::errors::PrintedSqlFailure;
use crate::checking::main::check_unbound::check_unbound;
use crate::checking::types::{FunctionLookup, RelationLookup};

/// Succeeds when DuckDB parses `sql` as one statement and binds it over the relations and functions.
pub fn check_printed(
    sql: &str,
    (lookup, functions): (RelationLookup<'_>, FunctionLookup<'_>),
) -> Result<(), PrintedSqlFailure> {
    let names: Vec<String> = check_unbound(sql)?;
    bound(sql, &names, (lookup, functions))
}
