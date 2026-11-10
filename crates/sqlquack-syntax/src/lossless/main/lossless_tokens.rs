//! `lossless_tokens`: lossless tokens of `sql` around significant tokens already read from it.

use crate::lossless::_helpers::trivia::trivia;
use crate::lossless::errors::TokenizeFailure;
use crate::lossless::models::CharOffsets;
use crate::lossless::models::LosslessToken;
use crate::lossless::models::TokenKind;
use crate::tokens::models::{Span, TokenType};

/// `significant` with the blanks and comments between them; refused on an overlap or other text.
pub fn lossless_tokens(
    sql: &str,
    significant: &[(Span, TokenType)],
    nested_comments: bool,
) -> Result<Vec<LosslessToken>, TokenizeFailure> {
    let offsets = CharOffsets::new(sql);
    let mut out = Vec::with_capacity(significant.len() * 2);
    let mut cursor = 0;
    for (span, token_type) in significant {
        let (start, end) = offsets.bytes(*span);
        if start == end {
            continue;
        }
        if start < cursor {
            return Err(TokenizeFailure::Overlap { start, end });
        }
        out = trivia(sql, cursor..start, nested_comments, out)?;
        out.push(LosslessToken {
            kind: TokenKind::Significant(*token_type),
            start,
            end,
        });
        cursor = end;
    }
    trivia(sql, cursor..sql.len(), nested_comments, out)
}
