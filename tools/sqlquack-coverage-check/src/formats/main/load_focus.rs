//! `load_focus`: a dialect's focus file, checked against the loaded catalogue; fails closed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::formats::_helpers::files::read_toml;
use crate::formats::_helpers::focus::{entry_problems, header_problems};
use crate::formats::models::{FocusFile, FormStatus, SyntaxCoverage};

/// The focus file at `path`, or every problem in its header and entries.
pub fn load_focus(path: &Path, coverage: &SyntaxCoverage) -> Result<FocusFile, Vec<String>> {
    let (focus, _): (FocusFile, String) = read_toml(path).map_err(|problem| vec![problem])?;
    let place: String = path.display().to_string();
    let statuses: BTreeMap<&str, FormStatus> = coverage
        .catalogue
        .values()
        .flatten()
        .map(|form| (form.id.as_str(), form.status))
        .collect();
    let mut problems: Vec<String> = header_problems(&focus, &place);
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for entry in &focus.entries {
        problems.extend(entry_problems(entry, &statuses, &seen, &place));
        seen.insert(&entry.form);
    }
    match problems.is_empty() {
        true => Ok(focus),
        false => Err(problems),
    }
}
