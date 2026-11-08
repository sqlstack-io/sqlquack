//! `classify`: which of the grammar's keyword classes a word belongs to.

use crate::keywords::constants::{
    BARE_LABEL, COLUMN_NAME, FUNC_NAME, RESERVED, TYPE_NAME, UNRESERVED,
};
use crate::keywords::models::KeywordClasses;

/// The keyword classes of `word`, compared without case.
pub fn classify(word: &str) -> KeywordClasses {
    let upper: String = word.to_ascii_uppercase();
    let word: &str = upper.as_str();
    KeywordClasses {
        reserved: RESERVED.contains(&word),
        unreserved: UNRESERVED.contains(&word),
        column_name: COLUMN_NAME.contains(&word),
        func_name: FUNC_NAME.contains(&word),
        type_name: TYPE_NAME.contains(&word),
        bare_label: BARE_LABEL.contains(&word),
    }
}
