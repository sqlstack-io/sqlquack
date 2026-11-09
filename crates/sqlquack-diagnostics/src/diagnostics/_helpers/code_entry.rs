//! `code_entry`: a registered code by its code, name or alias.

use crate::diagnostics::constants::CODES;
use crate::diagnostics::models::CodeEntry;

/// The registry entry whose code, name or alias is `key`, ignoring ASCII case.
pub(crate) fn code_entry(key: &str) -> Option<&'static CodeEntry> {
    CODES.iter().find(|entry| {
        entry.code.eq_ignore_ascii_case(key)
            || entry.name.eq_ignore_ascii_case(key)
            || entry
                .alias
                .is_some_and(|alias| alias.eq_ignore_ascii_case(key))
    })
}
