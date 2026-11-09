//! Snowflake types the comparison treats specially.

/// Snowflake types returned as JSON text, compared without whitespace outside strings.
pub const SEMI_STRUCTURED_TYPES: &[&str] = &["VARIANT", "OBJECT", "ARRAY"];
