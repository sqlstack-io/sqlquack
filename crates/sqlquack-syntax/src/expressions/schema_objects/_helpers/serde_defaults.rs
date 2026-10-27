//! Schema objects: serde defaults.

/// Helper function for serde default value
pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn is_true(v: &bool) -> bool {
    *v
}
