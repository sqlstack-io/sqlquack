//! SQLite's types: only a literal's or a fixed-affinity cast's storage class, as values vary by row.

use sqlquack_syntax::DataType;
use sqlquack_syntax::expressions::names::models::Literal;

use crate::lineage::main::rendered_type::rendered_type;

const INTEGER: &str = "INTEGER";
const REAL: &str = "REAL";
const TEXT: &str = "TEXT";
const BLOB: &str = "BLOB";
/// The longest decimal integer literal that always fits SQLite's 64-bit INTEGER.
const SAFE_INTEGER_DIGITS: usize = 18;

/// The storage class SQLite gives a literal, when it has one.
pub(crate) fn sqlite_literal_type(literal: &Literal) -> Option<&'static str> {
    match literal {
        Literal::String(_) => Some(TEXT),
        Literal::HexString(_) => Some(BLOB),
        Literal::Number(text) if text.contains(['.', 'e', 'E']) => Some(REAL),
        Literal::Number(text)
            if text.len() <= SAFE_INTEGER_DIGITS && text.bytes().all(|b| b.is_ascii_digit()) =>
        {
            Some(INTEGER)
        }
        _ => None,
    }
}

/// A cast's storage class by affinity; none for NUMERIC affinity, INTEGER or REAL by value.
pub(crate) fn sqlite_cast_spelling(data_type: &DataType) -> Option<String> {
    let written: String = rendered_type(data_type)?.to_ascii_uppercase();
    let class: &str = if written.contains("INT") {
        INTEGER
    } else if ["CHAR", "CLOB", "TEXT"]
        .iter()
        .any(|part| written.contains(part))
    {
        TEXT
    } else if written.contains(BLOB) {
        BLOB
    } else if ["REAL", "FLOA", "DOUB"]
        .iter()
        .any(|part| written.contains(part))
    {
        REAL
    } else {
        return None;
    };
    Some(class.to_owned())
}
