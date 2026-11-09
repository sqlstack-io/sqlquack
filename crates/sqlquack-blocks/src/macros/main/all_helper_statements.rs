//! `all_helper_statements`: the statements defining every helper macro.

use crate::macros::_helpers::registry::{definition, helper_macros};
use crate::macros::constants::CREATE_HELPER_SCHEMA;

/// The schema, then every helper macro: what an engine checking converted SQL creates once.
pub fn all_helper_statements() -> Vec<String> {
    std::iter::once(CREATE_HELPER_SCHEMA.to_owned())
        .chain(helper_macros().iter().map(definition))
        .collect()
}
