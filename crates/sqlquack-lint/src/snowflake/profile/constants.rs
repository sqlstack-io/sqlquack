//! Constants of Snowflake's lint profile.

/// Other dialects' aggregates the Snowflake parser classifies, though Snowflake has none by the name.
pub(crate) const FOREIGN_AGGREGATE_CALLS: [&str; 6] = [
    "BIT_AND", "BIT_OR", "BIT_XOR", "BOOL_AND", "BOOL_OR", "EVERY",
];
