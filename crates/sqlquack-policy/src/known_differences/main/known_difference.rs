//! `known_difference`: one known difference by its divergence name.

use crate::known_differences::constants::KNOWN_DIFFERENCES;
use crate::known_differences::models::KnownDifference;

/// The entry describing the divergence `name`, such as `hash`, if there is one.
pub fn known_difference(name: &str) -> Option<&'static KnownDifference> {
    KNOWN_DIFFERENCES.iter().find(|entry| entry.name == name)
}
