//! Expected differences.

use crate::expected::constants::ERROR_PREFIX;
use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

/// The expected-differences file.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedFile {
    /// One entry per spelling.
    #[serde(default, rename = "spelling")]
    pub spellings: Vec<ExpectedSpelling>,
    /// One entry per reviewed parser fix.
    #[serde(default, rename = "fix")]
    pub fixes: Vec<ParserFix>,
}

/// Valid Snowflake polyglot rejects and SQLQuack parses, recognised by its text and polyglot's error.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserFix {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed, such as `]:`.
    pub marker: String,
    /// Polyglot's errors on it; its error contains one of them.
    pub polyglot_errors: Vec<String>,
    /// Why SQLQuack's parse is right; a test in sqlquack-syntax pins its tree.
    pub note: String,
}

/// A call spelling SQLQuack keeps as written, and what polyglot makes of it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedSpelling {
    /// The upper-case call name.
    pub name: String,
    /// The nodes polyglot lowers it to, by arity; `error` where polyglot rejects the call.
    pub polyglot: Vec<String>,
    /// Why keeping the call changes spelling only.
    pub note: String,
}

/// The allowed polyglot nodes for each spelling.
#[derive(Clone, Debug, Default)]
pub struct ExpectedDifferences {
    /// Upper-case name to the polyglot nodes allowed for it.
    pub allowed: BTreeMap<String, BTreeSet<String>>,
    /// The reviewed parser fixes.
    pub fixes: Vec<ParserFix>,
}

impl ParserFix {
    /// Whether polyglot's rendered error is one this fix lists.
    fn names_error(&self, polyglot: &str) -> bool {
        self.polyglot_errors
            .iter()
            .any(|error| polyglot.contains(error.as_str()))
    }
}

impl ExpectedDifferences {
    /// The fix explaining polyglot's error `polyglot` on `sql` where SQLQuack parsed it.
    pub fn fix_for(&self, sql: &str, polyglot: &str) -> Option<&ParserFix> {
        let compact: String = sql
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect();
        self.fixes.iter().find(|fix| {
            polyglot.starts_with(ERROR_PREFIX)
                && fix.names_error(polyglot)
                && compact.contains(&fix.marker)
        })
    }

    /// Whether keeping `name` where polyglot makes `polyglot` of it is listed.
    pub fn allows(&self, name: &str, polyglot: &str) -> bool {
        self.allowed
            .get(name)
            .is_some_and(|nodes| nodes.contains(polyglot))
    }
}
