//! `resolve_name`: what a possibly qualified name means in a scope.

use sqlquack_diagnostics::diagnostics::constants::CASE_ONLY_MATCH;
use sqlquack_diagnostics::refusals::main::unresolved::unresolved;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::dialects::models::Profiles;
use crate::snowflake::scopes::models::{Resolution, Scope};

/// FROM columns, then select aliases, then outer queries; names the target cannot tell apart refuse.
pub fn resolve_name<'s, A>(
    scope: &'s Scope<'_, A>,
    (qualifier, name): (Option<&str>, &str),
    profiles: Profiles,
) -> Result<Resolution<'s, A>, Refusal> {
    let Profiles { source, target } = profiles;
    let alias = || {
        scope
            .aliases
            .iter()
            .rev()
            .find(|(alias, _)| source.same_identifier(alias, name))
            .map(|(_, value)| Resolution::Alias(value))
    };
    if scope.aliases_first
        && qualifier.is_none()
        && let Some(resolution) = alias()
    {
        return Ok(resolution);
    }
    let mut found: Vec<Resolution<'s, A>> = Vec::new();
    for relation in &scope.relations {
        if qualifier.is_some_and(|qualifier| !target.same_identifier(&relation.alias, qualifier)) {
            continue;
        }
        let hidden: &[String] = match qualifier {
            Some(_) => &[],
            None => &relation.merged,
        };
        for column in relation.columns.iter().filter(|column| {
            target.same_identifier(&column.name, name) && !hidden.contains(&column.name)
        }) {
            if !source.same_identifier(&column.name, name)
                || qualifier
                    .is_some_and(|qualifier| !source.same_identifier(&relation.alias, qualifier))
            {
                return Err(unresolved(format!(
                    "`{name}` matches `{}` {CASE_ONLY_MATCH}, which {} cannot tell apart",
                    column.name,
                    target.name()
                )));
            }
            found.push(Resolution::Column {
                alias: &relation.alias,
                column,
            });
        }
    }
    match found.len() {
        1 => return Ok(found.remove(0)),
        0 => {}
        _ => return Err(unresolved(format!("column `{name}` is ambiguous"))),
    }
    if qualifier.is_none()
        && let Some(resolution) = alias()
    {
        return Ok(resolution);
    }
    match scope.outer {
        Some(outer) => resolve_name(outer, (qualifier, name), profiles),
        None => Err(unresolved(match qualifier {
            Some(qualifier) => format!("column `{qualifier}.{name}` does not resolve"),
            None => format!("column `{name}` does not resolve"),
        })),
    }
}
