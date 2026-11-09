//! `sited`: a helper macro's body whose inline raises take their ID from its site argument.

use sqlquack_core::failures::constants::{RAISE_ID_MARKER, TAG_END, UNIDENTIFIED_RAISE};

use crate::macros::constants::SITE_PARAMETER;

/// `body` with each `<tag>#?: <text>` raise taking its ID from the call's site, as `site_raised` does.
pub fn sited(body: &str) -> String {
    body.replace(
        &format!("{RAISE_ID_MARKER}{UNIDENTIFIED_RAISE}{TAG_END}"),
        &format!("' || {SITE_PARAMETER} || '{TAG_END}"),
    )
}
