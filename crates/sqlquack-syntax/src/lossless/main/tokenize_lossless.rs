//! `tokenize_lossless`: lossless tokens of `sql`, with a tokenizer configuration deciding the significant tokens.

use crate::lossless::_helpers::trivia::trivia;
use crate::lossless::errors::TokenizeFailure;
use crate::lossless::models::CharOffsets;
use crate::lossless::models::LosslessToken;
use crate::lossless::models::TokenKind;
use crate::tokens::models::Tokenizer;
use crate::tokens::models::TokenizerConfig;

/// Lossless tokens of `sql`, with the tokenizer `config` builds deciding the significant tokens.
pub fn tokenize_lossless(
    sql: &str,
    config: TokenizerConfig,
) -> Result<Vec<LosslessToken>, TokenizeFailure> {
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
        out = trivia(sql, cursor..start, nested, out)?;
        out.push(LosslessToken {
            kind: TokenKind::Significant(token.token_type),
            start,
            end,
        });
        cursor = end;
    }
    out = trivia(sql, cursor..sql.len(), nested, out)?;
    Ok(out)
}
