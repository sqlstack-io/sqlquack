//! `sited`: a helper macro's body whose inline raises take their ID from its site argument.

use sqlquack_diagnostics::failures::constants::{RAISE_ID_MARKER, TAG_END, UNIDENTIFIED_RAISE};

use crate::macros::constants::SITE_PARAMETER;
use crate::raises::constants::UNIDENTIFIED_SITE;

/// `body` with each `<tag>#?: <text>` raise and each nested helper call taking the call's site.
pub fn sited(body: &str) -> String {
    body.replace(
        &format!("{RAISE_ID_MARKER}{UNIDENTIFIED_RAISE}{TAG_END}"),
        &format!("' || {SITE_PARAMETER} || '{TAG_END}"),
    )
    .replace(UNIDENTIFIED_SITE, SITE_PARAMETER)
}
