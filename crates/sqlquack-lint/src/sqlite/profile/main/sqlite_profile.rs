//! `sqlite_profile`: how lint reads SQLite SQL.

use sqlquack_syntax::Dialect;

use crate::dialects::main::shared_predicates::shared_predicates;
use crate::dialects::main::shared_statements::shared_statements;
use crate::dialects::main::shared_tokens::shared_tokens;
use crate::dialects::models::LintProfile;

/// How SQLite SQL is read, and the lint facts only it has.
pub(crate) fn sqlite_profile() -> LintProfile {
    LintProfile {
        tokens: |sql| shared_tokens(sql, Dialect::SQLite),
        statements: |sql, options| shared_statements(sql, Dialect::SQLite, options),
        predicates: |_, tokens| shared_predicates(tokens, Dialect::SQLite),
        own_predicate_parser: false,
        foreign_aggregates: &[],
        plain_call_aggregates: false,
        named_aggregates: &[],
        single_argument_aggregates: &[],
        json_parse_functions: &[],
        folds_upper: false,
        quoted_names_ignore_case: true,
        identifier_quote: '"',
        union_distinct: false,
    }
}
