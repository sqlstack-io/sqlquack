//! Run summaries.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::comparison::models::Spelling;

/// A statement whose results differ, or whose tokens are not lossless.
#[derive(Clone, Debug)]
pub struct Finding {
    /// Where the statement comes from.
    pub from: String,
    /// What differs.
    pub detail: String,
}

/// Counts and timings for one corpus.
#[derive(Clone, Debug, Default)]
pub struct RunSummary {
    /// Entries compared.
    pub entries: usize,
    /// Entries both parsers parsed into identical trees.
    pub same_trees: usize,
    /// Entries both parsers refused with identical errors.
    pub same_errors: usize,
    /// Entries polyglot rejects and SQLQuack parses, as a reviewed fix explains.
    pub fixed: BTreeMap<String, usize>,
    /// Entries SQLQuack rejects and polyglot parses, as a reviewed rejection explains.
    pub rejected: BTreeMap<String, usize>,
    /// Entries whose results differ.
    pub differences: Vec<Finding>,
    /// Entries whose lossless tokens do not reproduce the input.
    pub tokens_lost: Vec<Finding>,
    /// Casts whose classified type name differs from polyglot's printed type.
    pub type_names: Vec<Finding>,
    /// Kept call spellings and how many calls used each.
    pub spellings: BTreeMap<Spelling, usize>,
    /// Kept spellings the expected-differences list does not allow.
    pub unlisted: Vec<Finding>,
    /// Faithful parses that differ from the lowered parse beyond their kept spellings.
    pub unexplained: Vec<Finding>,
    /// polyglot's total parse time.
    pub polyglot: Duration,
    /// SQLQuack's total parse time.
    pub sqlquack: Duration,
}

impl RunSummary {
    /// Whether every entry agrees, keeps its tokens and cast names, and differs only as listed.
    pub fn exact(&self) -> bool {
        self.differences.is_empty()
            && self.tokens_lost.is_empty()
            && self.type_names.is_empty()
            && self.unlisted.is_empty()
            && self.unexplained.is_empty()
    }
}

/// Median whole-corpus parse times.
#[derive(Clone, Copy, Debug, Default)]
pub struct ParserTimes {
    /// polyglot's.
    pub polyglot: Duration,
    /// SQLQuack's.
    pub sqlquack: Duration,
}
