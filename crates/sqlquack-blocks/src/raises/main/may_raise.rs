//! `may_raise`: whether SQL holds a raise, inline or in a helper macro it calls.

use crate::blocks::constants::RAISE_MARKER;
use crate::raises::_helpers::slots::raise_slots;

/// Whether `sql` holds a raise or calls a helper macro with a raise site.
pub fn may_raise(sql: &str) -> bool {
    sql.contains(RAISE_MARKER) || !raise_slots(sql).is_empty()
}
