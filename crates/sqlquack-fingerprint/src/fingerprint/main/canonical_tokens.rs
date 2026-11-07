//! `canonical_tokens`: the token stream a fingerprint hashes.

use crate::fingerprint::_helpers::calls::{
    calls, is_unquoted_word, matching_closers, name_positions,
};
use crate::fingerprint::_helpers::significant_texts::significant_texts;
use crate::fingerprint::errors::FingerprintError;
use crate::fingerprint::main::is_reserved_keyword::is_reserved_keyword;
use crate::fingerprint::models::FingerprintDialect;

/// Significant token texts of `sql`, with unquoted reserved keywords and called names, outside name positions, upper-cased.
pub fn canonical_tokens(
    sql: &str,
    dialect: FingerprintDialect,
) -> Result<Vec<String>, FingerprintError> {
    let raws: Vec<&str> = significant_texts(sql, dialect)?;
    let uppers: Vec<String> = raws.iter().map(|raw| raw.to_ascii_uppercase()).collect();
    let names: Vec<bool> = name_positions(&uppers);
    let closers: Vec<Option<usize>> = matching_closers(&uppers);
    let mut texts: Vec<String> = Vec::with_capacity(raws.len());
    for (index, raw) in raws.iter().enumerate() {
        let folds = !names[index]
            && is_unquoted_word(raw)
            && (is_reserved_keyword(dialect, &uppers[index])
                || calls(raw, &uppers, &closers, index));
        texts.push(if folds {
            uppers[index].clone()
        } else {
            (*raw).to_owned()
        });
    }
    Ok(texts)
}
