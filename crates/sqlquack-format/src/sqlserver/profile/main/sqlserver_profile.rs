//! `sqlserver_profile`: how SQL Server's Transact-SQL is read and laid out.

use sqlquack_parse::sqlserver::is_reserved;
use sqlquack_syntax::Dialect;

use crate::dialects::models::{FormatProfile, LayoutRules};
use crate::sqlserver::profile::_helpers::reading::{lex_sqlserver, parse_sqlserver};
use crate::sqlserver::profile::constants::{
    APPLY_PREFIXES, BEGIN_STATEMENTS, BLOCK_WORDS, BRANCH_STATEMENTS, INLINE_CALLS, LIST_CLAUSES,
    OBJECT_PRECEDERS, SPACED_BEFORE_PARENTHESIS,
};

/// How Transact-SQL is read, by its own scanner and parser, and laid out; the shared tokenizer types its words as PostgreSQL's.
pub(crate) fn sqlserver_profile() -> FormatProfile {
    FormatProfile {
        dialect: Dialect::PostgreSQL,
        lex: Some(lex_sqlserver),
        keywords: None,
        parse: Some(parse_sqlserver),
        rules: LayoutRules {
            inline_calls: &INLINE_CALLS,
            is_reserved,
            top_queries: true,
            object_names: true,
            object_name_preceders: &OBJECT_PRECEDERS,
            from_list_commas: true,
            list_clauses: &LIST_CLAUSES,
            apply_prefixes: &APPLY_PREFIXES,
            spaced_before_parenthesis: &SPACED_BEFORE_PARENTHESIS,
            whole_variables: true,
            begin_statements: &BEGIN_STATEMENTS,
            branch_statements: &BRANCH_STATEMENTS,
            block_words: &BLOCK_WORDS,
            ..LayoutRules::default()
        },
    }
}
