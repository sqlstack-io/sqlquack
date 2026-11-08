//! Databricks type spellings compared by the type they denote.

use crate::dialects::constants::{DATABRICKS_DECIMAL, DATABRICKS_DEFAULT_DECIMAL};
use crate::dialects::main::canonical_type::canonical_type;

/// Databricks' synonyms, each folded to one spelling.
const SYNONYMS: [(&str, &str); 9] = [
    ("INTEGER", "INT"),
    ("LONG", "BIGINT"),
    ("SHORT", "SMALLINT"),
    ("BYTE", "TINYINT"),
    ("REAL", "FLOAT"),
    ("DEC", "DECIMAL"),
    ("NUMERIC", "DECIMAL"),
    ("TIMESTAMP_LTZ", "TIMESTAMP"),
    ("DOUBLE PRECISION", "DOUBLE"),
];

/// Whether two Databricks type spellings denote one type: synonyms and DECIMAL's defaults.
pub(crate) fn same_databricks_type(left: &str, right: &str) -> bool {
    databricks_type(left) == databricks_type(right)
}

/// A Databricks type spelling with synonyms folded and DECIMAL's precision and scale filled in.
fn databricks_type(spelling: &str) -> String {
    let words: String = spelling.split_whitespace().collect::<Vec<&str>>().join(" ");
    let upper: String = words.to_ascii_uppercase();
    let (head, arguments): (&str, &str) = match upper.find('(') {
        Some(open) => (upper[..open].trim_end(), &upper[open..]),
        None => (upper.as_str(), ""),
    };
    let head: &str = SYNONYMS
        .iter()
        .find(|(synonym, _)| *synonym == head)
        .map_or(head, |(_, folded)| folded);
    let arguments: String = arguments.chars().filter(|c| !c.is_whitespace()).collect();
    if head == DATABRICKS_DECIMAL {
        let (precision, scale) = DATABRICKS_DEFAULT_DECIMAL;
        let inner: &str = arguments.trim_start_matches('(').trim_end_matches(')');
        return match inner.split_once(',') {
            _ if inner.is_empty() => format!("DECIMAL({precision},{scale})"),
            Some((p, s)) => format!("DECIMAL({p},{s})"),
            None => format!("DECIMAL({inner},0)"),
        };
    }
    canonical_type(&format!("{head}{arguments}"))
}
