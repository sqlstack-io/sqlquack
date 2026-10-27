//! `identify_raises`: gives each raise waiting for an ID the ID of the expression it is in.

use sqlquack_core::failures::constants::RAISE_ID_MARKER;

use crate::raises::_helpers::slots::{content_id, raise_slots};

/// `sql` with each unidentified raise given the hash of its tag and the ID-free SQL, and the IDs.
pub fn identify_raises(sql: &str) -> (String, Vec<String>) {
    let slots = raise_slots(sql);
    if !slots.iter().any(|slot| slot.unidentified) {
        return (sql.to_owned(), Vec::new());
    }
    let mut content = String::with_capacity(sql.len());
    let mut copied = 0;
    for slot in &slots {
        content.push_str(&sql[copied..slot.marker.start]);
        copied = slot.marker.end;
    }
    content.push_str(&sql[copied..]);
    let mut identified = String::with_capacity(sql.len());
    let mut ids: Vec<String> = Vec::new();
    copied = 0;
    for slot in slots.iter().filter(|slot| slot.unidentified) {
        let id = content_id(&content, &sql[slot.tag.clone()]);
        identified.push_str(&sql[copied..slot.marker.start]);
        identified.push(RAISE_ID_MARKER);
        identified.push_str(&id);
        copied = slot.marker.end;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    identified.push_str(&sql[copied..]);
    (identified, ids)
}
