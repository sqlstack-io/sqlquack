//! The built-in Rules a family selector or an absent selection enables.

/// The codes enabled when no Rule selection is given.
pub fn default_rules() -> Vec<&'static str> {
    crate::lint::_helpers::engine::DEFAULT_RULES.to_vec()
}
