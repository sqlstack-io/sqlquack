//! Lossless tokens: every input byte belongs to exactly one token, trivia included.

use crate::dialects::{DialectImpl, DuckDBDialect, SnowflakeDialect};
use crate::tokens::{Span, TokenType, Tokenizer, TokenizerConfig};
use crate::{Dialect, Error};

/// What a lossless token holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// A token the parser reads.
    Significant(TokenType),
    /// A run of whitespace.
    Whitespace,
    /// A comment that runs to the end of the line (`--`, or `//` in Snowflake).
    LineComment,
    /// A `/* ... */` comment.
    BlockComment,
}

/// One token of the input, with its byte range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessToken {
    /// What the token holds.
    pub kind: TokenKind,
    /// Byte offset of the token's first byte.
    pub start: usize,
    /// Byte offset just past the token's last byte.
    pub end: usize,
}

impl LosslessToken {
    /// The token's text in `sql`, the input it was produced from.
    pub fn text<'sql>(&self, sql: &'sql str) -> &'sql str {
        &sql[self.start..self.end]
    }
}

/// Why SQL could not be split into lossless tokens.
#[derive(Debug, thiserror::Error)]
pub enum TokenizeFailure {
    /// The tokenizer itself rejected the input.
    #[error(transparent)]
    Tokenizer(#[from] Error),
    /// Text between significant tokens that is neither whitespace nor a comment.
    #[error("text at bytes {start}..{end} belongs to no token")]
    Uncovered {
        /// Byte offset where the uncovered text starts.
        start: usize,
        /// Byte offset where it ends.
        end: usize,
    },
    /// A significant token that starts before the previous one ends.
    #[error("the token at bytes {start}..{end} overlaps the previous token")]
    Overlap {
        /// Byte offset where the token starts.
        start: usize,
        /// Byte offset where it ends.
        end: usize,
    },
}

/// Lossless tokens of `sql`, with the dialect's tokenizer deciding the significant tokens.
pub(crate) fn tokenize_lossless(
    sql: &str,
    dialect: Dialect,
) -> Result<Vec<LosslessToken>, TokenizeFailure> {
    let config = tokenizer_config(dialect);
    let nested = config.nested_comments;
    let tokens = Tokenizer::new(config).tokenize(sql)?;
    let offsets = CharOffsets::new(sql);
    let mut out = Vec::with_capacity(tokens.len() * 2);
    let mut cursor = 0;
    for token in &tokens {
        let (start, end) = offsets.bytes(token.span);
        if start == end {
            continue;
        }
        if start < cursor {
            return Err(TokenizeFailure::Overlap { start, end });
        }
        trivia(sql, cursor, start, nested, &mut out)?;
        out.push(LosslessToken {
            kind: TokenKind::Significant(token.token_type),
            start,
            end,
        });
        cursor = end;
    }
    trivia(sql, cursor, sql.len(), nested, &mut out)?;
    Ok(out)
}

fn tokenizer_config(dialect: Dialect) -> TokenizerConfig {
    match dialect {
        Dialect::Snowflake => SnowflakeDialect.tokenizer_config(),
        Dialect::DuckDB => DuckDBDialect.tokenizer_config(),
    }
}

/// Byte offsets for the tokenizer's character offsets.
struct CharOffsets {
    bytes: Option<Vec<usize>>,
    len: usize,
}

impl CharOffsets {
    fn new(sql: &str) -> Self {
        let bytes = (!sql.is_ascii()).then(|| sql.char_indices().map(|(byte, _)| byte).collect());
        Self {
            bytes,
            len: sql.len(),
        }
    }

    fn byte(&self, char_offset: usize) -> usize {
        match &self.bytes {
            None => char_offset.min(self.len),
            Some(bytes) => bytes.get(char_offset).copied().unwrap_or(self.len),
        }
    }

    fn bytes(&self, span: Span) -> (usize, usize) {
        (self.byte(span.start), self.byte(span.end))
    }
}

/// Splits the gap `sql[start..end]` into whitespace and comment tokens.
fn trivia(
    sql: &str,
    start: usize,
    end: usize,
    nested: bool,
    out: &mut Vec<LosslessToken>,
) -> Result<(), TokenizeFailure> {
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
    Ok(())
}

/// The whitespace the copied tokenizer skips between tokens.
fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t' | '\r' | '\n' | '\u{00A0}' | '\u{2000}'..='\u{200B}' | '\u{3000}' | '\u{FEFF}'
    )
}

/// The length of the block comment at the start of `text`, up to the end if unterminated.
fn block_comment_len(text: &str, nested: bool) -> usize {
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
