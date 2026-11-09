//! `postgres_profile`: how PostgreSQL SQL is read and laid out.

use sqlquack_syntax::Dialect;

use crate::dialects::models::{FormatProfile, LayoutRules};
use crate::postgres::profile::_helpers::reading::{apply_native_keywords, parse_postgres};
use crate::postgres::profile::constants::{
    CTE_CLAUSES, INLINE_CALLS, INLINE_SOURCES, INTO_CLAUSE, OPERATOR_BYTES, QUANTIFIERS,
    SPACED_QUANTIFIER,
};

/// How PostgreSQL SQL is read and laid out.
pub(crate) fn postgres_profile() -> FormatProfile {
    FormatProfile {
        dialect: Dialect::PostgreSQL,
        lex: None,
        keywords: Some(apply_native_keywords),
        parse: Some(parse_postgres),
        rules: LayoutRules {
            inline_calls: &INLINE_CALLS,
            operator_bytes: OPERATOR_BYTES,
            quantifiers: &QUANTIFIERS,
            spaced_quantifier: SPACED_QUANTIFIER,
            cte_clauses: &CTE_CLAUSES,
            clause_keywords: &[INTO_CLAUSE],
            inline_sources: &INLINE_SOURCES,
            ..LayoutRules::default()
        },
    }
}
