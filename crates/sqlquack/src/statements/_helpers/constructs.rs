//! The constructs a recorded statement uses, the case's own functions as one project key.

use std::collections::BTreeSet;

use sqlquack_core::classification::constants::PROJECT_FUNCTION_CONSTRUCT;
use sqlquack_core::classification::main::classify::classify;
use sqlquack_core::classification::models::{Construct, ConstructFamily};
use sqlquack_core::parsing::main::parse_source::parse_source;
use sqlquack_core::parsing::models::{SourceDialect, SourceLines};
use sqlquack_evidence::cases::models::LoadedCase;

use crate::statements::_helpers::spellings::{token_spellings, tree_spellings};

/// The upper-case names of the functions the case itself defines.
pub(crate) fn own_functions(case: &LoadedCase) -> BTreeSet<String> {
    case.functions
        .iter()
        .map(|function| function.name.to_ascii_uppercase())
        .collect()
}

/// `key`, or the project function key when it calls one of the case's own functions.
pub(crate) fn project_key(key: String, own: &BTreeSet<String>) -> String {
    let prefix = format!("{}:", ConstructFamily::Function.prefix());
    match key
        .strip_prefix(&prefix)
        .is_some_and(|name| own.contains(name))
    {
        true => Construct::new(ConstructFamily::Function, PROJECT_FUNCTION_CONSTRUCT).key(),
        false => key,
    }
}

/// Every construct and spelling key the case's statement uses, sorted, and why it did not parse, if not.
pub(crate) fn constructs(
    case: &LoadedCase,
    own: &BTreeSet<String>,
) -> (Vec<String>, Option<String>) {
    let sql = &case.source_sql;
    let lines = SourceLines::new(sql);
    let mut keys: BTreeSet<String> = token_spellings(sql);
    let statements = match parse_source(sql, SourceDialect::Snowflake) {
        Ok(statements) => statements,
        Err(failure) => return (keys.into_iter().collect(), Some(failure.to_string())),
    };
    for statement in &statements {
        keys.extend(tree_spellings(statement));
        for occurrence in classify(statement, &lines, SourceDialect::Snowflake) {
            keys.insert(project_key(occurrence.construct.key(), own));
        }
    }
    (keys.into_iter().collect(), None)
}
