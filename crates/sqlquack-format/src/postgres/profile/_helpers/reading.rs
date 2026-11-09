//! Reading SQL with PostgreSQL's own scanner and parser: its keywords, which the formatter checks the shared tokens against, and its statements.

use sqlquack_parse::postgres::scan::main::scan::scan as scan_postgres;
use sqlquack_parse::postgres::scan::models::{
    Token as PostgresToken, TokenKind as PostgresTokenKind,
};
use sqlquack_parse::postgres::{StrictFailure, parse_strict};
use sqlquack_syntax::Expression;
use sqlquack_syntax::tokens::models::TokenType;

use crate::dialects::main::is_keyword::is_keyword;
use crate::dialects::main::is_unquoted_word::is_unquoted_word;
use crate::dialects::models::Lexeme;
use crate::dialects::models::StatementSpan;
use crate::format::constants::PARSE_FAILURE;
use crate::postgres::profile::constants::{OPAQUE_STATEMENT_FAILURE, SCANNER_MISMATCH_FAILURE};

/// Give each of `lexemes` the type PostgreSQL's scanner reads it as; refused unless that scanner splits `sql` into exactly `lexemes`.
pub(crate) fn apply_native_keywords(
    sql: &str,
    mut lexemes: Vec<Lexeme>,
) -> Result<Vec<Lexeme>, String> {
    let keywords = native_keywords(sql, &lexemes)?;
    for (lexeme, keyword) in lexemes.iter_mut().zip(keywords) {
        lexeme.kind = native_kind(lexeme, keyword);
    }
    Ok(lexemes)
}

/// The statements PostgreSQL's grammar reads from `sql`, refused when any is kept as opaque text.
pub(crate) fn parse_postgres(sql: &str) -> Result<(Vec<Expression>, Vec<StatementSpan>), String> {
    let parsed = parse_strict(sql).map_err(|failure| match failure {
        StrictFailure::Parse(error) => format!("{PARSE_FAILURE}: {}", error.message),
        StrictFailure::Uncovered { position } => {
            format!("{PARSE_FAILURE}: no token covers byte {position}")
        }
    })?;
    if !parsed.gaps.is_empty() {
        return Err(OPAQUE_STATEMENT_FAILURE.to_string());
    }
    Ok((parsed.statements, Vec::new()))
}

/// For each of `lexemes`, whether PostgreSQL's scanner reads it as a keyword; refused unless that scanner splits `sql` into exactly `lexemes`.
fn native_keywords(sql: &str, lexemes: &[Lexeme]) -> Result<Vec<bool>, String> {
    let scanned: Vec<PostgresToken> = scan_postgres(sql)
        .into_iter()
        .filter(|token| !matches!(token.kind, PostgresTokenKind::Fault(_)))
        .collect();
    let same_tokens = scanned.len() == lexemes.len()
        && scanned
            .iter()
            .zip(lexemes)
            .all(|(token, lexeme)| (token.start, token.end) == (lexeme.start, lexeme.end));
    if !same_tokens {
        return Err(SCANNER_MISMATCH_FAILURE.to_string());
    }
    Ok(scanned
        .iter()
        .map(|token| matches!(token.kind, PostgresTokenKind::Keyword(_)))
        .collect())
}

/// The shared token type of `lexeme` once PostgreSQL's scanner has said whether it is a `keyword`: a word the shared tokenizer reads the other way becomes `Unknown`, which the guide treats as a keyword, or `Var`, a name.
fn native_kind(lexeme: &Lexeme, keyword: bool) -> TokenType {
    if !is_unquoted_word(&lexeme.raw) || keyword == is_keyword(lexeme) {
        lexeme.kind
    } else if keyword {
        TokenType::Unknown
    } else {
        TokenType::Var
    }
}
