//! `load_translation_files`: the by-design list's signed-off constructs, format-checked.

use std::collections::BTreeSet;
use std::path::Path;

use crate::translation::_helpers::by_design::signed_off;

/// The constructs with a current sign-off under `directory`, or every format problem.
pub fn load_translation_files(directory: &Path) -> Result<BTreeSet<String>, Vec<String>> {
    signed_off(directory)
}
