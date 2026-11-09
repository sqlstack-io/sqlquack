//! `relation_columns`: the columns of the relation a name reads.

use sqlquack_diagnostics::diagnostics::constants::CASE_ONLY_MATCH;
use sqlquack_diagnostics::refusals::main::unresolved::unresolved;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::dialects::models::Profiles;
use crate::snowflake::names::constants::{UNKNOWN_RELATION_PREFIX, UNKNOWN_RELATION_SUFFIX};
use crate::snowflake::types::main::schema_columns::schema_columns;
use crate::snowflake::types::models::{ColumnSpec, Schema, TypedLeaves};

/// The innermost CTE so named, else the schema's relation; names the target cannot tell apart refuse.
pub fn relation_columns(
    name: &str,
    (ctes, schema): (&[(String, Vec<ColumnSpec>)], &Schema),
    typed: &TypedLeaves,
    profiles: Profiles,
) -> Result<Vec<ColumnSpec>, Refusal> {
    let Profiles { source, target } = profiles;
    if let Some((_, columns)) = ctes
        .iter()
        .rev()
        .find(|(cte, _)| source.same_identifier(cte, name))
    {
        return Ok(columns.clone());
    }
    let relation = schema.relations.get(name).or_else(|| {
        schema
            .relations
            .iter()
            .find(|(relation, _)| source.same_identifier(relation, name))
            .map(|(_, columns)| columns)
    });
    if let Some(columns) = relation {
        return Ok(schema_columns(columns, typed));
    }
    if let Some(known) = ctes
        .iter()
        .map(|(cte, _)| cte.as_str())
        .chain(schema.relations.keys().map(String::as_str))
        .find(|known| target.same_identifier(known, name))
    {
        return Err(unresolved(format!(
            "relation `{name}` matches `{known}` {CASE_ONLY_MATCH}"
        )));
    }
    Err(unresolved(format!(
        "{UNKNOWN_RELATION_PREFIX}{name}{UNKNOWN_RELATION_SUFFIX}"
    )))
}
