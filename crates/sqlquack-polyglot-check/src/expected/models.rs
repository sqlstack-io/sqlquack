//! Expected differences.

use crate::expected::constants::{ERROR_PREFIX, OK_PREFIX};
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
    /// One entry per reviewed statement SQLQuack rejects where polyglot parses it.
    #[serde(default, rename = "rejection")]
    pub rejections: Vec<ParserRejection>,
}

/// A statement SQLQuack rejects and polyglot parses, recognised by its exact text and dialect.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserRejection {
    /// A lower-case identifier.
    pub id: String,
    /// The exact statement.
    pub sql: String,
    /// The dialects it is rejected in, such as `DuckDB`.
    pub dialects: Vec<String>,
    /// Text SQLQuack's error contains.
    pub sqlquack_error: String,
    /// Why SQLQuack's error is right; a test in sqlquack-syntax pins it.
    pub note: String,
}

/// Valid Snowflake polyglot rejects and SQLQuack parses, recognised by its text and polyglot's error.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserFix {
    /// A lower-case identifier.
    pub id: String,
    /// Text the statement contains, with whitespace removed and ignoring case, such as `]:`.
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
    /// The reviewed rejections.
    pub rejections: Vec<ParserRejection>,
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
            .map(|character| character.to_ascii_uppercase())
            .collect();
        self.fixes.iter().find(|fix| {
            polyglot.starts_with(ERROR_PREFIX)
                && fix.names_error(polyglot)
                && compact.contains(&fix.marker.to_ascii_uppercase())
        })
    }

    /// The rejection explaining SQLQuack's error `sqlquack` on `sql` in `dialect` where polyglot parsed it.
    pub fn rejection_for(
        &self,
        sql: &str,
        dialect: &str,
        polyglot: &str,
        sqlquack: &str,
    ) -> Option<&ParserRejection> {
        self.rejections.iter().find(|rejection| {
            polyglot.starts_with(OK_PREFIX)
                && sqlquack.starts_with(ERROR_PREFIX)
                && sqlquack.contains(&rejection.sqlquack_error)
                && rejection.sql == sql
                && rejection.dialects.contains(&dialect.to_owned())
        })
    }

    /// Whether keeping `name` where polyglot makes `polyglot` of it is listed.
    pub fn allows(&self, name: &str, polyglot: &str) -> bool {
        self.allowed
            .get(name)
            .is_some_and(|nodes| nodes.contains(polyglot))
    }
}
