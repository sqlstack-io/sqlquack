//! The tags and ID markers of the raises in emitted SQL.

use sqlquack_diagnostics::failures::constants::{
    ERROR_TAG_PREFIX, RAISE_ID_DIGITS, RAISE_ID_MARKER, TAG_END, UNIDENTIFIED_RAISE,
};

use crate::macros::constants::HELPER_CALL_PREFIX;
use crate::raises::constants::{FNV_OFFSET, FNV_PRIME, RAISE_OPENING};
use crate::raises::models::RaiseSlot;

/// The length of the tag `text` starts with: `sqlquack:<errno>` or a lower-case divergence name.
pub(crate) fn tag_length(text: &str) -> Option<usize> {
    let (prefix, allowed): (usize, fn(u8) -> bool) = match text.starts_with(ERROR_TAG_PREFIX) {
        true => (ERROR_TAG_PREFIX.len(), |byte| byte.is_ascii_digit()),
        false => (0, |byte| byte.is_ascii_lowercase() || byte == b'_'),
    };
    let run = text[prefix..]
        .bytes()
        .take_while(|byte| allowed(*byte))
        .count();
    (run > 0).then_some(prefix + run)
}

/// Every raise in `sql` with a tag and an ID marker, and every helper call's raise site, in order.
pub(crate) fn raise_slots(sql: &str) -> Vec<RaiseSlot> {
    let mut slots: Vec<RaiseSlot> = Vec::new();
    for (index, _) in sql.match_indices(RAISE_OPENING) {
        let start = index + RAISE_OPENING.len();
        let Some(length) = tag_length(&sql[start..]) else {
            continue;
        };
        let marker = start + length;
        if let Some(slot) = slot_at(sql, start..marker) {
            slots.push(slot);
        }
    }
    for (index, _) in sql.match_indices(HELPER_CALL_PREFIX) {
        let name = index + HELPER_CALL_PREFIX.len();
        let after_name = name
            + sql[name..]
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
        if sql[after_name..].starts_with("('")
            && let Some(slot) = site_at(sql, after_name + 2)
        {
            slots.push(slot);
        }
    }
    slots.sort_by_key(|slot| slot.marker.start);
    slots
}

/// The raise site starting at `start`, `#?` or `#<id>` closed by a quote, untagged.
fn site_at(sql: &str, start: usize) -> Option<RaiseSlot> {
    let rest = sql[start..].strip_prefix(RAISE_ID_MARKER)?;
    let unidentified = rest.starts_with(UNIDENTIFIED_RAISE);
    let length = match unidentified {
        true => UNIDENTIFIED_RAISE.len(),
        false => {
            let id = rest.get(..RAISE_ID_DIGITS)?;
            if !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            RAISE_ID_DIGITS
        }
    };
    if !rest[length..].starts_with('\'') {
        return None;
    }
    Some(RaiseSlot {
        tag: start..start,
        marker: start..start + RAISE_ID_MARKER.len_utf8() + length,
        unidentified,
    })
}

/// The ID marker after the tag `tag` of `sql`, when one follows it and ends the tag.
fn slot_at(sql: &str, tag: std::ops::Range<usize>) -> Option<RaiseSlot> {
    let rest = sql[tag.end..].strip_prefix(RAISE_ID_MARKER)?;
    let unidentified = rest.starts_with(UNIDENTIFIED_RAISE);
    let length = match unidentified {
        true => UNIDENTIFIED_RAISE.len(),
        false => {
            let id = rest.get(..RAISE_ID_DIGITS)?;
            if !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return None;
            }
            RAISE_ID_DIGITS
        }
    };
    if !rest[length..].starts_with(TAG_END) {
        return None;
    }
    let marker = tag.end..tag.end + RAISE_ID_MARKER.len_utf8() + length;
    Some(RaiseSlot {
        tag,
        marker,
        unidentified,
    })
}

/// The hash of an expression's SQL without raise IDs, `content`, that each tag's ID continues.
pub(crate) fn content_hash(content: &str) -> u64 {
    fnv(FNV_OFFSET, content.bytes().chain([0]))
}

/// The ID of a raise tagged `tag` in an expression whose `content_hash` is `content`.
pub(crate) fn content_id(content: u64, tag: &str) -> String {
    let hash = fnv(content, tag.bytes());
    let digits = u32::try_from(RAISE_ID_DIGITS * 4).unwrap_or(u64::BITS);
    format!(
        "{:0width$x}",
        hash >> (u64::BITS - digits),
        width = RAISE_ID_DIGITS
    )
}

/// FNV-1a over `bytes`, from `hash`.
fn fnv(hash: u64, bytes: impl Iterator<Item = u8>) -> u64 {
    bytes.fold(hash, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}
