//! `shared_texts`: the significant token texts of SQL under the shared lossless tokenizer.

use sqlquack_parse::tokenize;
use sqlquack_syntax::lossless::models::TokenKind;
use sqlquack_syntax::tokens::models::Dialect;

use crate::fingerprint::errors::FingerprintError;

/// The text of each significant token of `sql` as the shared tokenizer reads `dialect`.
pub(crate) fn shared_texts(sql: &str, dialect: Dialect) -> Result<Vec<&str>, FingerprintError> {
    Ok(tokenize(sql, dialect)
        .map_err(|error| FingerprintError::Tokenize(error.to_string()))?
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Significant(_)))
        .map(|token| token.text(sql))
        .collect())
}
