//! Reading SQL with a dialect's own scanner and parser: its tokens with the comments in the blanks it skipped between them, and PostgreSQL's keywords and statements, which the formatter checks the shared tokens against.

use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::{Dialect, Expression, TokenKind};
use sqlquack_syntax_postgres::scan::main::scan::scan as scan_postgres;
use sqlquack_syntax_postgres::scan::models::{
    Token as PostgresToken, TokenKind as PostgresTokenKind,
};
use sqlquack_syntax_postgres::{StrictFailure, parse_strict};
use sqlquack_syntax_snowflake::tokenize;

use crate::format::_helpers::lexemes::{Comment, Lexeme, comment, is_keyword, is_unquoted_word};
use crate::format::constants::{
    BLOCK_COMMENT_END, BLOCK_COMMENT_START, HASH_COMMENT_START, LINE_COMMENT_START, PARSE_FAILURE,
    POSTGRES_OPAQUE_STATEMENT_FAILURE, SCANNER_MISMATCH_FAILURE,
};

/// How a dialect writes comments, beyond `--` and `/* */`.
#[derive(Clone, Copy)]
pub(crate) struct CommentSyntax {
    /// `#` starts a comment that runs to the end of the line.
    pub(crate) hash: bool,
    /// Block comments nest.
    pub(crate) nested: bool,
}

/// The comments in the blanks of `sql` from `start` to `end`, which the scanner skipped; only blanks and comments may lie there.
pub(crate) fn gap_comments(
    sql: &str,
    start: usize,
    end: usize,
    syntax: CommentSyntax,
) -> Vec<Comment> {
    let mut comments: Vec<Comment> = Vec::new();
    let mut at = start;
    while at < end {
        let rest = &sql[at..end];
        let length = if (syntax.hash && rest.starts_with(HASH_COMMENT_START))
            || rest.starts_with(LINE_COMMENT_START)
        {
            rest.find(['\r', '\n']).unwrap_or(rest.len())
        } else if rest.starts_with(BLOCK_COMMENT_START) {
            block_comment_length(rest, syntax.nested)
        } else {
            at += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let line = !rest.starts_with(BLOCK_COMMENT_START);
        comments.push(comment(sql, at, at + length, line));
        at += length;
    }
    comments
}

/// The length of the block comment `rest` starts with, through the `*/` that closes it.
fn block_comment_length(rest: &str, nested: bool) -> usize {
    let mut depth: usize = 0;
    let mut at = 0;
    while at < rest.len() {
        if rest[at..].starts_with(BLOCK_COMMENT_START) && (nested || depth == 0) {
            depth += 1;
            at += BLOCK_COMMENT_START.len();
        } else if rest[at..].starts_with(BLOCK_COMMENT_END) {
            depth -= 1;
            at += BLOCK_COMMENT_END.len();
            if depth == 0 {
                return at;
            }
        } else {
            at += rest[at..].chars().next().map_or(1, char::len_utf8);
        }
    }
    rest.len()
}

/// The type the shared tokenizer gives `text` when it reads it as exactly one token.
pub(crate) fn shared_type(text: &str) -> Option<TokenType> {
    let Ok(tokens) = tokenize(text, Dialect::PostgreSQL) else {
        return None;
    };
    match tokens.as_slice() {
        [token] if token.end - token.start == text.len() => match token.kind {
            TokenKind::Significant(kind) => Some(kind),
            _ => None,
        },
        _ => None,
    }
}

/// The statements PostgreSQL's grammar reads from `sql`, refused when any is kept as opaque text.
pub(crate) fn parse_postgres(sql: &str) -> Result<Vec<Expression>, String> {
    let parsed = parse_strict(sql).map_err(|failure| match failure {
        StrictFailure::Parse(error) => format!("{PARSE_FAILURE}: {}", error.message),
        StrictFailure::Uncovered { position } => {
            format!("{PARSE_FAILURE}: no token covers byte {position}")
        }
    })?;
    if !parsed.gaps.is_empty() {
        return Err(POSTGRES_OPAQUE_STATEMENT_FAILURE.to_string());
    }
    Ok(parsed.statements)
}

/// For each of `lexemes`, whether PostgreSQL's scanner reads it as a keyword; refused unless that scanner splits `sql` into exactly `lexemes`.
pub(crate) fn native_keywords(sql: &str, lexemes: &[Lexeme]) -> Result<Vec<bool>, String> {
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
pub(crate) fn native_kind(lexeme: &Lexeme, keyword: bool) -> TokenType {
    if !is_unquoted_word(&lexeme.raw) || keyword == is_keyword(lexeme) {
        lexeme.kind
    } else if keyword {
        TokenType::Unknown
    } else {
        TokenType::Var
    }
}
