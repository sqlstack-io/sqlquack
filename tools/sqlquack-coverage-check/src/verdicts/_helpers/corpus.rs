//! Reading the corpus files and taking each file's verdicts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sqlquack_syntax::Dialect;

use crate::verdicts::_helpers::invariants::check_statement;
use crate::verdicts::constants::CORPUS_EXTENSION;
use crate::verdicts::models::{CorpusFile, DialectVerdicts, StatementCheck};

/// The dialects an entry is parsed in, by the name it gives.
fn dialects(name: &str) -> Result<&'static [(&'static str, Dialect)], String> {
    match name {
        "snowflake" => Ok(&[("snowflake", Dialect::Snowflake)]),
        "duckdb" => Ok(&[("duckdb", Dialect::DuckDB)]),
        "postgres" => Ok(&[("postgres", Dialect::PostgreSQL)]),
        "sqlite" => Ok(&[("sqlite", Dialect::SQLite)]),
        "both" => Ok(&[
            ("snowflake", Dialect::Snowflake),
            ("duckdb", Dialect::DuckDB),
        ]),
        other => Err(format!("unknown corpus dialect {other}")),
    }
}

/// The corpus file at `path`.
pub(crate) fn read_corpus(path: &Path) -> Result<CorpusFile, String> {
    std::fs::read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|error| error.to_string()))
        .map_err(|problem| format!("{}: {problem}", path.display()))
}

/// The `.json` corpus files in `corpora`, sorted.
pub(crate) fn corpus_files(corpora: &Path) -> Result<Vec<PathBuf>, String> {
    let listing =
        std::fs::read_dir(corpora).map_err(|error| format!("{}: {error}", corpora.display()))?;
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in listing {
        let path: PathBuf = entry
            .map_err(|error| format!("{}: {error}", corpora.display()))?
            .path();
        if path.extension() == Some(std::ffi::OsStr::new(CORPUS_EXTENSION)) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// One corpus file's verdicts by dialect, and its statements that break an invariant.
pub(crate) fn file_verdicts(
    file: &CorpusFile,
    name: &str,
) -> Result<(BTreeMap<String, DialectVerdicts>, Vec<String>), String> {
    let mut by_dialect: BTreeMap<String, DialectVerdicts> = BTreeMap::new();
    let mut broken: Vec<String> = Vec::new();
    for (position, entry) in file.entries.iter().enumerate() {
        let parsed_in =
            dialects(&entry.dialect).map_err(|problem| format!("{name} #{position}: {problem}"))?;
        for (dialect_name, dialect) in parsed_in {
            let check: StatementCheck = check_statement(&entry.sql, *dialect);
            let verdicts: &mut DialectVerdicts =
                by_dialect.entry((*dialect_name).to_owned()).or_default();
            verdicts.statements += 1;
            if check.refused {
                verdicts.refused.push(position);
            }
            for problem in check.problems {
                broken.push(format!("{name} {dialect_name} #{position}: {problem}"));
            }
        }
    }
    Ok((by_dialect, broken))
}
