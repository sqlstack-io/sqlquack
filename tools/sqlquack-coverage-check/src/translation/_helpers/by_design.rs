//! The by-design list and its sign-offs: only a signed-off entry removes a form from the base.

use std::collections::BTreeSet;
use std::path::Path;

use serde::de::DeserializeOwned;

use crate::formats::main::content_sha256::content_sha256;
use crate::formats::main::is_utc_timestamp::is_utc_timestamp;
use crate::translation::constants::{BY_DESIGN_FILE, BY_DESIGN_REVIEW_FILE};
use crate::translation::models::{ByDesignEntry, ByDesignFile, ByDesignReviewFile};

/// A TOML file, its default when absent, or the problem reading it.
fn read_optional<T: DeserializeOwned + Default>(path: &Path) -> Result<T, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

/// A construct key as convert reports it: a kind, a colon, and a name, without whitespace.
fn is_construct(key: &str) -> bool {
    key.split_once(':').is_some_and(|(kind, name)| {
        !kind.is_empty() && !name.is_empty() && !key.chars().any(char::is_whitespace)
    })
}

/// Whether the auditor signed off `entry` with its current reason.
fn has_current_signoff(entry: &ByDesignEntry, review: &ByDesignReviewFile) -> bool {
    let reason: String = content_sha256(entry.reason.as_bytes());
    review
        .signoffs
        .iter()
        .any(|signoff| signoff.construct == entry.construct && signoff.reason_sha256 == reason)
}

/// The constructs with a current sign-off, or every format problem in either file.
pub(crate) fn signed_off(directory: &Path) -> Result<BTreeSet<String>, Vec<String>> {
    let entries: ByDesignFile =
        read_optional(&directory.join(BY_DESIGN_FILE)).map_err(|problem| vec![problem])?;
    let review: ByDesignReviewFile =
        read_optional(&directory.join(BY_DESIGN_REVIEW_FILE)).map_err(|problem| vec![problem])?;
    let mut problems: Vec<String> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for entry in &entries.entries {
        let key: &str = &entry.construct;
        if !is_construct(key) {
            problems.push(format!(
                "by_design {key}: not a construct key such as fn:NAME"
            ));
        }
        if !seen.insert(key) {
            problems.push(format!("by_design {key}: listed twice"));
        }
        if entry.reason.trim().is_empty() || entry.duckdb_note.trim().is_empty() {
            problems.push(format!("by_design {key}: needs a reason and a duckdb_note"));
        }
    }
    for signoff in &review.signoffs {
        if !is_utc_timestamp(&signoff.reviewed_utc) {
            problems.push(format!(
                "signoff {}: reviewed_utc is not UTC",
                signoff.construct
            ));
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    Ok(entries
        .entries
        .iter()
        .filter(|entry| has_current_signoff(entry, &review))
        .map(|entry| entry.construct.clone())
        .collect())
}
