//! `duckdb_profile`: how DuckDB SQL is read and laid out.

use sqlquack_syntax::Dialect;

use crate::dialects::models::{FormatProfile, LayoutRules};
use crate::duckdb::profile::constants::{INLINE_CALLS, OPERATOR_BYTES};

/// How DuckDB SQL is read and laid out.
pub(crate) fn duckdb_profile() -> FormatProfile {
    FormatProfile {
        dialect: Dialect::DuckDB,
        lex: None,
        keywords: None,
        parse: None,
        rules: LayoutRules {
            inline_calls: &INLINE_CALLS,
            operator_bytes: OPERATOR_BYTES,
            tight_colons: true,
            percent_suffix: true,
            from_first: true,
            ..LayoutRules::default()
        },
    }
}
