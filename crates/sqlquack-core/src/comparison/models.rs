//! Comparison declarations and results.

use std::fmt::{self, Display};

use serde::Serialize;

/// How a declared field name matches JSON object keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldMatch {
    /// Keys equal to the name.
    Exact,
    /// Keys containing the name.
    Contains,
}

/// A field whose value records only how SQL was spelled, never what it means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellingOnlyField {
    /// The JSON key, or the part of it, that is ignored.
    pub name: &'static str,
    /// How `name` matches keys.
    pub matching: FieldMatch,
    /// Why the field cannot change meaning.
    pub reason: &'static str,
}

impl SpellingOnlyField {
    /// Whether `key` is this field.
    pub fn matches(&self, key: &str) -> bool {
        match self.matching {
            FieldMatch::Exact => key == self.name,
            FieldMatch::Contains => key.contains(self.name),
        }
    }
}

/// A value rewritten to one spelling before comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellingNormalisation {
    /// What is rewritten.
    pub description: &'static str,
    /// Why both spellings mean the same.
    pub reason: &'static str,
}

/// The first place two trees differ.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TreeDifference {
    /// JSON path of the first differing value, such as `.select.expressions[2]`.
    pub path: String,
    /// The expected value there, as compact JSON, shortened.
    pub expected: String,
    /// The actual value there, as compact JSON, shortened.
    pub actual: String,
}

impl Display for TreeDifference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "trees differ at {}: expected {}, found {}",
            self.path, self.expected, self.actual
        )
    }
}
