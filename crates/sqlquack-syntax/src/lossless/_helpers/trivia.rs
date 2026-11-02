//! Lossless: trivia.

use crate::lossless::errors::TokenizeFailure;
use crate::lossless::models::LosslessToken;
use crate::lossless::models::TokenKind;
use crate::parser::models::DuckDBDialect;
use crate::parser::models::PostgresDialect;
use crate::parser::models::SnowflakeDialect;
use crate::parser::models::SqliteDialect;
use crate::parser::types::DialectImpl;
use crate::tokens::models::Dialect;
use crate::tokens::models::TokenizerConfig;
use std::ops::Range;

pub(crate) fn tokenizer_config(dialect: Dialect) -> TokenizerConfig {
    match dialect {
        Dialect::Snowflake => SnowflakeDialect.tokenizer_config(),
        Dialect::DuckDB => DuckDBDialect.tokenizer_config(),
        Dialect::PostgreSQL => PostgresDialect.tokenizer_config(),
        Dialect::SQLite => SqliteDialect.tokenizer_config(),
    }
}

/// Splits the gap `sql[start..end]` into whitespace and comment tokens.
pub(crate) fn trivia(
    sql: &str,
    span: Range<usize>,
    nested: bool,
    mut out: Vec<LosslessToken>,
) -> Result<Vec<LosslessToken>, TokenizeFailure> {
    let Range { start, end } = span;
    let gap = &sql[start..end];
    let mut at = 0;
    while at < gap.len() {
        let rest = &gap[at..];
        let (kind, len) =
            if rest.starts_with("--") || rest.starts_with("//") || rest.starts_with('#') {
                (
                    TokenKind::LineComment,
                    rest.find('\n').unwrap_or(rest.len()),
                )
            } else if rest.starts_with("/*") {
                (TokenKind::BlockComment, block_comment_len(rest, nested))
            } else {
                let len = rest
                    .char_indices()
                    .find(|(_, c)| !is_whitespace(*c))
                    .map_or(rest.len(), |(index, _)| index);
                if len == 0 {
                    return Err(TokenizeFailure::Uncovered {
                        start: start + at,
                        end,
                    });
                }
                (TokenKind::Whitespace, len)
            };
        out.push(LosslessToken {
            kind,
            start: start + at,
            end: start + at + len,
        });
        at += len;
    }
    Ok(out)
}

/// The whitespace the copied tokenizer skips between tokens.
pub(crate) fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\r' | '\n' | '\u{00A0}' | '\u{2000}'..='\u{200B}' | '\u{3000}' | '\u{FEFF}'
    )
}

/// The length of the block comment at the start of `text`, up to the end if unterminated.
pub(crate) fn block_comment_len(text: &str, nested: bool) -> usize {
    let bytes = text.as_bytes();
    let mut depth = 0_usize;
    let mut at = 0;
    while at + 1 < bytes.len() {
        match (bytes[at], bytes[at + 1]) {
            (b'/', b'*') if depth == 0 || nested => {
                depth += 1;
                at += 2;
            }
            (b'*', b'/') => {
                depth -= 1;
                at += 2;
                if depth == 0 {
                    return at;
                }
            }
            _ => at += 1,
        }
    }
    text.len()
}
