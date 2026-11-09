//! `duckdb_profile`: how lint reads DuckDB SQL.

use sqlquack_syntax::Dialect;

use crate::dialects::main::shared_predicates::shared_predicates;
use crate::dialects::main::shared_statements::shared_statements;
use crate::dialects::main::shared_tokens::shared_tokens;
use crate::dialects::models::LintProfile;

/// How DuckDB SQL is read, and the lint facts only it has.
pub(crate) fn duckdb_profile() -> LintProfile {
    LintProfile {
        tokens: |sql| shared_tokens(sql, Dialect::DuckDB),
        statements: |sql, options| shared_statements(sql, Dialect::DuckDB, options),
        predicates: |_, tokens| shared_predicates(tokens, Dialect::DuckDB),
        own_predicate_parser: false,
        foreign_aggregates: &[],
        plain_call_aggregates: false,
        named_aggregates: &[],
        single_argument_aggregates: &[],
        json_parse_functions: &["JSON"],
        folds_upper: false,
        quoted_names_ignore_case: true,
        identifier_quote: '"',
        union_distinct: true,
    }
}
