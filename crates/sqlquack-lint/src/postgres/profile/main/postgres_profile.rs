//! `postgres_profile`: how lint reads PostgreSQL SQL.

use sqlquack_syntax::Dialect;

use crate::dialects::main::native_statements::native_statements;
use crate::dialects::main::shared_predicates::shared_predicates;
use crate::dialects::main::shared_tokens::shared_tokens;
use crate::dialects::models::LintProfile;
use crate::postgres::profile::_helpers::strict::strict_parse;
use crate::postgres::profile::constants::{
    AGGREGATE_CALLS, FOREIGN_AGGREGATE_CALLS, SINGLE_ARGUMENT_AGGREGATES,
};

/// How PostgreSQL SQL is read, and the lint facts only it has.
pub(crate) fn postgres_profile() -> LintProfile {
    LintProfile {
        tokens: |sql| shared_tokens(sql, Dialect::PostgreSQL),
        statements: |sql, _| native_statements(sql, strict_parse),
        predicates: |_, tokens| shared_predicates(tokens, Dialect::PostgreSQL),
        own_predicate_parser: false,
        foreign_aggregates: &FOREIGN_AGGREGATE_CALLS,
        plain_call_aggregates: true,
        named_aggregates: &AGGREGATE_CALLS,
        single_argument_aggregates: &SINGLE_ARGUMENT_AGGREGATES,
        json_parse_functions: &[],
        folds_upper: false,
        quoted_names_ignore_case: false,
        identifier_quote: '"',
        union_distinct: true,
    }
}
