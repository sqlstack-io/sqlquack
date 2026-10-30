//! What a dialect profile answers.

/// How one dialect resolves and compares names.
pub trait DialectProfile: std::fmt::Debug + Sync {
    /// The dialect's name.
    fn name(&self) -> &'static str;
    /// The resolved name of an identifier written `name`, quoted or not.
    fn resolved_identifier(&self, name: &str, quoted: bool) -> String;
    /// Whether two resolved names name the same object.
    fn same_identifier(&self, left: &str, right: &str) -> bool;
}
