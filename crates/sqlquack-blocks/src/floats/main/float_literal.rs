//! `float_literal`: a text literal cast to FLOAT, folded to the double it converts to.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::blocks::constants::{
    CONVERSION_WHITESPACE_CHARS, NON_FINITE_FLOAT_WORDS, TEXT_TO_FLOAT,
};

/// The double the text converts to as text_to_float reads it; `None` when that fails or overflows.
pub fn float_literal(text: &str, log: &ProvenanceLog) -> Option<String> {
    let trimmed = text.trim_matches(CONVERSION_WHITESPACE_CHARS);
    let lower = trimmed.to_ascii_lowercase();
    let word = NON_FINITE_FLOAT_WORDS
        .iter()
        .find(|(word, _)| trimmed.is_ascii() && *word == lower)
        .map(|(_, duckdb)| *duckdb);
    let value = match word {
        Some(duckdb) => duckdb,
        None if is_decimal_text(trimmed) && trimmed.parse::<f64>().is_ok_and(f64::is_finite) => {
            trimmed
        }
        None => return None,
    };
    log.building_block(TEXT_TO_FLOAT);
    Some(format!("CAST('{value}' AS DOUBLE)"))
}

/// Whether `text` is all of DECIMAL_TEXT_PATTERN: a signed mantissa with an optional exponent.
fn is_decimal_text(text: &str) -> bool {
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (unsigned, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            (digits(whole) && (fraction.is_empty() || digits(fraction)))
                || (whole.is_empty() && digits(fraction))
        }
        None => digits(mantissa),
    };
    mantissa_ok
        && exponent
            .is_none_or(|exponent| digits(exponent.strip_prefix(['+', '-']).unwrap_or(exponent)))
}
