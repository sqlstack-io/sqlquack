//! `site_raised`: a raise in a helper macro's body, its ID taken from the call's site argument.

use crate::blocks::constants::RAISE_MARKER;
use crate::raises::_helpers::slots::tag_length;
use crate::raises::constants::RAISE_OPENING;

/// Raises `<tag>: <text>` as `<tag>#<id>: <text>`, the ID from SQL `site`, ending with SQL `tail`.
pub fn site_raised(message: &str, site: &str, tail: Option<&str>) -> String {
    let (tag, text) = message.split_at(tag_length(message).unwrap_or(0));
    let tail = tail
        .map(|sql| format!("' || {sql} || '"))
        .unwrap_or_default();
    format!(
        "{RAISE_OPENING}{}' || {site} || '{}{tail}{RAISE_MARKER}",
        tag.replace('\'', "''"),
        text.replace('\'', "''")
    )
}
