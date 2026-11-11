//! The recorded verdicts and the corpus files they are taken from.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One corpus file's statements in one dialect: how many, and which SQLQuack refuses.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct DialectVerdicts {
    /// The statements parsed in the dialect.
    pub statements: usize,
    /// The positions, in the file's `entries`, of the statements SQLQuack refuses.
    pub refused: Vec<usize>,
}

/// The recorded verdicts: by corpus file name, then by dialect.
#[derive(Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct Verdicts {
    /// What the file records and how to record it again.
    pub about: String,
    /// The verdicts of each corpus file.
    pub corpora: BTreeMap<String, BTreeMap<String, DialectVerdicts>>,
}

/// The corpora as parsed now: their verdicts, and the statements that break an invariant.
#[derive(Debug, Default)]
pub struct CorpusParse {
    /// The verdict on every statement.
    pub verdicts: Verdicts,
    /// `<file> <dialect> #<position>: <problem>` for each statement that breaks an invariant.
    pub broken: Vec<String>,
}

/// One statement in one dialect: whether SQLQuack refuses it, and the invariants it breaks.
#[derive(Debug, Default)]
pub struct StatementCheck {
    /// Whether the faithful parse fails.
    pub refused: bool,
    /// What each broken invariant says.
    pub problems: Vec<String>,
}

/// A vendored corpus file.
#[derive(Debug, Deserialize)]
pub struct CorpusFile {
    /// Its statements.
    pub entries: Vec<CorpusEntry>,
}

/// A statement of a corpus file.
#[derive(Debug, Deserialize)]
pub struct CorpusEntry {
    /// `snowflake`, `duckdb`, `postgres`, `sqlite`, or `both` for Snowflake and DuckDB.
    pub dialect: String,
    /// The SQL.
    pub sql: String,
}
