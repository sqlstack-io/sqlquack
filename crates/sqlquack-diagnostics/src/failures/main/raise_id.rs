//! `raise_id`: the ID of the expression a raised error comes from.

use crate::failures::constants::{RAISE_ID_DIGITS, RAISE_ID_MARKER, TAG_END};

/// The first `#<id>: ` raise ID in an error message, when it has one.
pub fn raise_id(message: &str) -> Option<&str> {
    for (index, _) in message.match_indices(RAISE_ID_MARKER) {
        let start = index + RAISE_ID_MARKER.len_utf8();
        let Some(id) = message.get(start..start + RAISE_ID_DIGITS) else {
            continue;
        };
        let hexadecimal = id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if hexadecimal && message[start + RAISE_ID_DIGITS..].starts_with(TAG_END) {
            return Some(id);
        }
    }
    None
}
