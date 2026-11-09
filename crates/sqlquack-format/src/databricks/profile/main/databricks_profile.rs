//! `databricks_profile`: how Databricks SQL is read and laid out.

use sqlquack_syntax::Dialect;

use crate::databricks::profile::_helpers::reading::{lex_databricks, parse_databricks};
use crate::databricks::profile::constants::{BY_CLAUSES, INLINE_CALLS, SPACED_BEFORE_PARENTHESIS};
use crate::dialects::models::{FormatProfile, LayoutRules};

/// How Databricks SQL is read, by its own scanner and parser, and laid out; the shared tokenizer types its words as PostgreSQL's.
pub(crate) fn databricks_profile() -> FormatProfile {
    FormatProfile {
        dialect: Dialect::PostgreSQL,
        lex: Some(lex_databricks),
        keywords: None,
        parse: Some(parse_databricks),
        rules: LayoutRules {
            inline_calls: &INLINE_CALLS,
            path_colons: true,
            bracket_paths: true,
            by_clauses: &BY_CLAUSES,
            spaced_before_parenthesis: &SPACED_BEFORE_PARENTHESIS,
            ..LayoutRules::default()
        },
    }
}
