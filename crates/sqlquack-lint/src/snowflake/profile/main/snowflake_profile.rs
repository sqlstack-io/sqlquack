//! `snowflake_profile`: how lint reads Snowflake SQL.

use sqlquack_syntax::Dialect;

use crate::dialects::main::shared_predicates::shared_predicates;
use crate::dialects::main::shared_statements::shared_statements;
use crate::dialects::main::shared_tokens::shared_tokens;
use crate::dialects::models::LintProfile;
use crate::snowflake::profile::constants::FOREIGN_AGGREGATE_CALLS;

/// How Snowflake SQL is read, and the lint facts only it has.
pub(crate) fn snowflake_profile() -> LintProfile {
    LintProfile {
        tokens: |sql| shared_tokens(sql, Dialect::Snowflake),
        statements: |sql, options| shared_statements(sql, Dialect::Snowflake, options),
        predicates: |_, tokens| shared_predicates(tokens, Dialect::Snowflake),
        own_predicate_parser: false,
        foreign_aggregates: &FOREIGN_AGGREGATE_CALLS,
        plain_call_aggregates: false,
        named_aggregates: &[],
        single_argument_aggregates: &[],
        json_parse_functions: &["PARSE_JSON", "TRY_PARSE_JSON"],
        folds_upper: true,
        quoted_names_ignore_case: false,
        identifier_quote: '"',
        union_distinct: true,
    }
}
