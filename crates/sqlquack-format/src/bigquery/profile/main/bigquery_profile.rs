//! `bigquery_profile`: how BigQuery's GoogleSQL is read and laid out.

use sqlquack_parse::bigquery::keywords::main::is_reserved::is_reserved;
use sqlquack_syntax::Dialect;

use crate::bigquery::profile::_helpers::reading::{lex_bigquery, parse_bigquery};
use crate::bigquery::profile::constants::{INLINE_CALLS, SET_OPERATOR_PREFIXES};
use crate::dialects::models::{FormatProfile, LayoutRules};

/// How GoogleSQL is read, by its own scanner and parser, and laid out; the shared tokenizer types its words as PostgreSQL's.
pub(crate) fn bigquery_profile() -> FormatProfile {
    FormatProfile {
        dialect: Dialect::PostgreSQL,
        lex: Some(lex_bigquery),
        keywords: None,
        parse: Some(parse_bigquery),
        rules: LayoutRules {
            inline_calls: &INLINE_CALLS,
            is_reserved,
            dashed_names: true,
            referenced_table_names: true,
            set_operator_prefixes: &SET_OPERATOR_PREFIXES,
            ..LayoutRules::default()
        },
    }
}
