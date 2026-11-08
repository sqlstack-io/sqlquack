//! Type constants.

/// The offsets a schema may declare for a TIMESTAMP_TZ source column, all meaning +00:00.
pub const DECLARABLE_OFFSETS: &[&str] = &["+00:00", "UTC"];

/// The top-level keys of a schema file written in sections.
pub const SCHEMA_SECTIONS: [&str; 2] = ["relations", "functions"];

/// The keys a column written as an object may hold.
pub const COLUMN_KEYS: [&str; 4] = ["name", "type", "typed_leaves", "offset"];

/// The keys a column written as an object must give, as strings.
pub const REQUIRED_COLUMN_KEYS: [&str; 2] = ["name", "type"];

/// The YAML key that merges another mapping's keys into the one holding it.
pub const MERGE_KEY: &str = "<<";
