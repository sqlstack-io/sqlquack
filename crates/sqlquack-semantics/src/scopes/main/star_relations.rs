//! `star_relations`: the relations a star expands to.

use sqlquack_diagnostics::refusals::main::unresolved::unresolved;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::dialects::models::Profiles;
use crate::schemas::types::DialectType;
use crate::scopes::models::{Scope, ScopeRelation};

/// The relation a qualified star names, or every relation in scope for a bare one.
pub fn star_relations<'s, T: DialectType, A>(
    scope: &'s Scope<'_, T, A>,
    qualifier: Option<&str>,
    profiles: Profiles,
) -> Result<Vec<&'s ScopeRelation<T>>, Refusal> {
    let relations: Vec<&ScopeRelation<T>> = scope
        .relations
        .iter()
        .filter(|relation| {
            qualifier
                .is_none_or(|qualifier| profiles.source.same_identifier(&relation.alias, qualifier))
        })
        .collect();
    if relations.is_empty() {
        return Err(unresolved(format!(
            "`{}.*` names no relation in scope",
            qualifier.unwrap_or_default()
        )));
    }
    Ok(relations)
}
