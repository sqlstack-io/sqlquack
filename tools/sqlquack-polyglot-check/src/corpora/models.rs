//! Corpus entries.

use serde::Deserialize;
use sqlquack_syntax::Dialect;

/// One statement, or several, to parse in one dialect.
#[derive(Clone, Debug)]
pub struct CorpusEntry {
    /// The dialect to parse in.
    pub dialect: Dialect,
    /// The SQL.
    pub sql: String,
    /// Where it comes from.
    pub from: String,
}

/// A vendored corpus file.
#[derive(Debug, Deserialize)]
pub struct CorpusFile {
    /// Where the corpus comes from.
    pub origin: String,
    /// Its entries.
    pub entries: Vec<CorpusFileEntry>,
}

/// An entry of a vendored corpus file.
#[derive(Debug, Deserialize)]
pub struct CorpusFileEntry {
    /// `snowflake`, `duckdb` or `both`.
    pub dialect: String,
    /// The SQL.
    pub sql: String,
    /// Where it comes from.
    pub from: String,
}
