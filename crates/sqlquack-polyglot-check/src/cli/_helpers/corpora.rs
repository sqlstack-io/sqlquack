//! The corpora a command line names.

use sqlquack_syntax::Dialect;

use crate::cli::models::{CorpusArgument, DialectArgument};
use crate::corpora::main::case_corpus::case_corpus;
use crate::corpora::main::directory_corpus::directory_corpus;
use crate::corpora::main::load_corpus::load_corpus;
use crate::corpora::models::CorpusEntry;

/// A corpus name and its entries, or why it could not be loaded.
pub(crate) type NamedCorpus = (String, Result<Vec<CorpusEntry>, String>);

/// Each corpus `argument` names.
pub(crate) fn named_corpora(argument: &CorpusArgument) -> Vec<NamedCorpus> {
    match argument {
        CorpusArgument::Files { paths } => paths
            .iter()
            .map(|path| (path.display().to_string(), load_corpus(path)))
            .collect(),
        CorpusArgument::Directory { path, dialect } => {
            let dialect = match dialect {
                DialectArgument::Snowflake => Dialect::Snowflake,
                DialectArgument::Duckdb => Dialect::DuckDB,
            };
            vec![(path.display().to_string(), directory_corpus(path, dialect))]
        }
        CorpusArgument::Cases { cases, policy } => {
            vec![(cases.display().to_string(), case_corpus(cases, policy))]
        }
    }
}
