//! What a dialect profile answers.

use crate::dialects::main::canonical_type::canonical_type;

/// How one dialect resolves and compares names.
pub trait DialectProfile: std::fmt::Debug + Sync {
    /// The dialect's name.
    fn name(&self) -> &'static str;
    /// The resolved name of an identifier written `name`, quoted or not.
    fn resolved_identifier(&self, name: &str, quoted: bool) -> String;
    /// Whether two resolved names name the same object.
    fn same_identifier(&self, left: &str, right: &str) -> bool;
    /// The type a TIMESTAMP written without its kind is, by default TIMESTAMP itself.
    fn timestamp_type(&self) -> &'static str {
        "TIMESTAMP"
    }
    /// Whether two type spellings denote the same type; by default, the same words and arguments.
    fn same_type(&self, left: &str, right: &str) -> bool {
        canonical_type(left) == canonical_type(right)
    }
}
