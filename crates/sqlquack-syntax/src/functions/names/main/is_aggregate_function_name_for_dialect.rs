//! `is_aggregate_function_name_for_dialect`: whether `name` parses as an aggregate in the source dialect.

use crate::functions::names::_helpers::aggregates::is_aggregate_function_name;
use crate::functions::names::_helpers::lookups::DUCKDB_AGGREGATE_FUNCTION_NAME_SET;
use crate::tokens::models::DialectType;

/// Whether `name` parses as an aggregate in the source dialect.
pub(crate) fn is_aggregate_function_name_for_dialect(
    name: &str,
    dialect: Option<DialectType>,
) -> bool {
    let upper = name.to_uppercase();
    (is_aggregate_function_name(name))
        || (matches!(dialect, Some(DialectType::DuckDB))
            && DUCKDB_AGGREGATE_FUNCTION_NAME_SET.contains(upper.as_str()))
}
