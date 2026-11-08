use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::dialects::constants::SNOWFLAKE_TO_DUCKDB;
use crate::scopes::main::resolve_name::resolve_name;
use crate::scopes::models::{Scope, ScopeRelation};
use crate::types::models::ColumnSpec;

/// A relation of VARCHAR columns.
pub(crate) fn relation(alias: &str, columns: &[&str]) -> ScopeRelation {
    ScopeRelation {
        alias: alias.to_owned(),
        columns: columns
            .iter()
            .map(|name| ColumnSpec::new(*name, SnowflakeType::Varchar))
            .collect(),
        merged: Vec::new(),
    }
}

/// What a name resolves to, as `alias.COLUMN` or `alias value`, or the refusal's reason.
pub(crate) fn resolved(
    scope: &Scope<'_, &'static str>,
    name: (Option<&str>, &str),
) -> Result<String, String> {
    resolve_name(scope, name, SNOWFLAKE_TO_DUCKDB)
        .map(|resolution| {
            resolution
                .column()
                .map(|(alias, column)| format!("{alias}.{}", column.name))
                .or_else(|| {
                    resolution
                        .alias_value()
                        .map(|value| format!("alias {value}"))
                })
                .unwrap_or_default()
        })
        .map_err(|refusal| refusal.reason)
}
