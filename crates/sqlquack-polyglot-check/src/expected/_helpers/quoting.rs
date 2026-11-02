//! Statement markers and trees that differ only by quoting polyglot drops.

use serde_json::Value;

use crate::expected::constants::{NAME_KEY, OK_PREFIX, QUOTED_KEY};

/// `sql` without whitespace, in upper case, as markers are matched.
pub(crate) fn compact(sql: &str) -> String {
    sql.chars()
        .filter(|character| !character.is_whitespace())
        .map(|character| character.to_ascii_uppercase())
        .collect()
}

/// Whether both rendered parses are trees that differ only by names polyglot leaves unquoted.
pub(crate) fn differ_only_by_kept_quoting(polyglot: &str, sqlquack: &str) -> bool {
    let (Some(theirs), Some(ours)) = (
        polyglot.strip_prefix(OK_PREFIX),
        sqlquack.strip_prefix(OK_PREFIX),
    ) else {
        return false;
    };
    match (
        serde_json::from_str::<Value>(theirs),
        serde_json::from_str::<Value>(ours),
    ) {
        (Ok(theirs), Ok(ours)) => theirs != ours && only_kept_quoting(&theirs, &ours),
        (Err(_), _) | (_, Err(_)) => false,
    }
}

/// Whether `theirs` equals `ours` except for names polyglot marks unquoted and SQLQuack quoted.
fn only_kept_quoting(theirs: &Value, ours: &Value) -> bool {
    match (theirs, ours) {
        (Value::Object(theirs), Value::Object(ours)) => {
            let name = ours.contains_key(NAME_KEY) && ours.contains_key(QUOTED_KEY);
            theirs.len() == ours.len()
                && theirs.iter().all(|(key, their_value)| match ours.get(key) {
                    None => false,
                    Some(our_value) if their_value == our_value => true,
                    Some(our_value) => {
                        (name
                            && key == QUOTED_KEY
                            && *their_value == Value::Bool(false)
                            && *our_value == Value::Bool(true))
                            || only_kept_quoting(their_value, our_value)
                    }
                })
        }
        (Value::Array(theirs), Value::Array(ours)) => {
            theirs.len() == ours.len()
                && theirs
                    .iter()
                    .zip(ours)
                    .all(|(their_value, our_value)| only_kept_quoting(their_value, our_value))
        }
        _ => theirs == ours,
    }
}
