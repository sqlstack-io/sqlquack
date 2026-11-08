//! `load_corpus`: the entries of a vendored corpus file.

use std::path::Path;

use sqlquack_syntax::Dialect;

use crate::corpora::models::{CorpusEntry, CorpusFile};

/// The entries of the corpus file at `path`; an entry for both dialects becomes two.
pub fn load_corpus(path: &Path) -> Result<Vec<CorpusEntry>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let file: CorpusFile = serde_json::from_str(&text)
        .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
    let mut entries = Vec::with_capacity(file.entries.len() * 2);
    for entry in file.entries {
        let dialects: &[Dialect] = match entry.dialect.as_str() {
            "snowflake" => &[Dialect::Snowflake],
            "duckdb" => &[Dialect::DuckDB],
            "postgres" => &[Dialect::PostgreSQL],
            "sqlite" => &[Dialect::SQLite],
            "both" => &[Dialect::Snowflake, Dialect::DuckDB],
            other => return Err(format!("{}: unknown dialect {other}", path.display())),
        };
        for dialect in dialects {
            entries.push(CorpusEntry {
                dialect: *dialect,
                sql: entry.sql.clone(),
                from: entry.from.clone(),
            });
        }
    }
    Ok(entries)
}
