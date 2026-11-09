//! `override_macros`: the macros of the user overrides converted SQL calls.

use std::collections::{BTreeMap, BTreeSet};

use sqlquack_convert::snowflake::conversion::main::override_macro::override_macro;
use sqlquack_convert::snowflake::conversion::models::ConversionOptions;
use sqlquack_policy::policy::constants::USER_OVERRIDE;

/// The macro of each override whose `user_override:<NAME>` label `divergences` holds.
pub fn override_macros(
    divergences: &BTreeMap<String, BTreeSet<String>>,
    options: &ConversionOptions,
) -> Vec<String> {
    let prefix = format!("{USER_OVERRIDE}:");
    let used: BTreeSet<&str> = divergences
        .keys()
        .filter_map(|label| label.strip_prefix(&prefix))
        .collect();
    options
        .overrides
        .values()
        .filter(|replacement| used.contains(replacement.name.as_str()))
        .map(override_macro)
        .collect()
}
