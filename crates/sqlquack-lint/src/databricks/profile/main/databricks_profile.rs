//! `databricks_profile`: how lint reads Databricks SQL.

use crate::databricks::profile::_helpers::strict::strict_parse;
use crate::databricks::profile::_helpers::tokenizer::DATABRICKS_TOKENIZER;
use crate::databricks::profile::constants::{
    AGGREGATE_CALLS, FOREIGN_AGGREGATE_CALLS, SINGLE_ARGUMENT_AGGREGATES,
};
use crate::dialects::main::native_predicates::native_predicates;
use crate::dialects::main::native_statements::native_statements;
use crate::dialects::models::LintProfile;

/// How Databricks SQL is read, and the lint facts only it has.
pub(crate) fn databricks_profile() -> LintProfile {
    LintProfile {
        tokens: |sql| {
            DATABRICKS_TOKENIZER
                .tokenize(sql)
                .map_err(|error| error.to_string())
        },
        statements: |sql, _| native_statements(sql, strict_parse),
        predicates: |text, _| native_predicates(text, strict_parse),
        own_predicate_parser: true,
        foreign_aggregates: &FOREIGN_AGGREGATE_CALLS,
        plain_call_aggregates: true,
        named_aggregates: &AGGREGATE_CALLS,
        single_argument_aggregates: &SINGLE_ARGUMENT_AGGREGATES,
        json_parse_functions: &["PARSE_JSON", "TRY_PARSE_JSON"],
        folds_upper: false,
        quoted_names_ignore_case: true,
        identifier_quote: '`',
        union_distinct: true,
    }
}
