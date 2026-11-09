//! `parse_tolerant`: parses each statement of `sql` on its own and keeps the ranges that do not parse.

use std::panic::catch_unwind;

use crate::modes::_helpers::statements::parse_one_statement;
use crate::modes::_helpers::statements::statement_ranges;
use crate::modes::_helpers::statements::unparsed_whole;
use crate::modes::models::TolerantParse;
use crate::modes::models::TolerantStatement;
use crate::parser::main::tokenize::tokenize;
use sqlquack_syntax::guard::models::ParseOptions;
use sqlquack_syntax::tokens::models::Dialect;

/// Parses each statement of `sql` on its own and keeps the ranges that do not parse.
pub fn parse_tolerant(sql: &str, dialect: Dialect, options: &ParseOptions) -> TolerantParse {
    let tokens = match catch_unwind(|| tokenize(sql, dialect)) {
        Ok(Ok(tokens)) => tokens,
        Ok(Err(failure)) => return unparsed_whole(sql, failure.to_string()),
        Err(_) => return unparsed_whole(sql, "the tokenizer panicked".to_owned()),
    };
    let statements: Vec<TolerantStatement> = statement_ranges(&tokens)
        .into_iter()
        .map(|(start, end)| TolerantStatement {
            start,
            end,
            parsed: parse_one_statement(&sql[start..end], dialect, options),
        })
        .collect();
    TolerantParse {
        tokens,
        token_failure: None,
        statements,
    }
}
