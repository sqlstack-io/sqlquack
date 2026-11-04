//! Which reviews are current, and what they exclude.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::models::Reason;
use crate::formats::constants::{CATALOGUE_DIR, OBLIGATIONS_DIR, SOURCES_FILE};
use crate::formats::models::{Review, ReviewVerdict};

/// Whether `review` is approved and bound to the current files and parser tree.
fn current(review: &Review, hashes: &BTreeMap<String, String>, parser_tree: Option<&str>) -> bool {
    let bound = |hash: &Option<String>, path: String| {
        hash.as_ref()
            .is_some_and(|hash| hashes.get(&path) == Some(hash))
    };
    let files: bool = match review.family.as_str() {
        "" => bound(&review.sources_sha256, SOURCES_FILE.to_owned()),
        family => {
            bound(
                &review.catalogue_sha256,
                format!("{CATALOGUE_DIR}/{family}.toml"),
            ) && bound(
                &review.obligations_sha256,
                format!("{OBLIGATIONS_DIR}/{family}.toml"),
            )
        }
    };
    review.verdict == ReviewVerdict::Approved
        && files
        && parser_tree == Some(review.parser_tree.as_str())
}

/// Why `family` lacks a current review, if it does.
pub(crate) fn review_reason(
    family: &str,
    reviews: &[Review],
    hashes: &BTreeMap<String, String>,
    parser_tree: Option<&str>,
) -> Option<Reason> {
    let mut named = reviews
        .iter()
        .filter(|review| review.family == family)
        .peekable();
    if named.peek().is_none() {
        return Some(Reason::Unreviewed);
    }
    (!named.any(|review| current(review, hashes, parser_tree))).then_some(Reason::StaleReview)
}

/// The form and source ids that current approved reviews exclude.
pub(crate) fn current_exclusions(
    reviews: &[Review],
    hashes: &BTreeMap<String, String>,
    parser_tree: Option<&str>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let exclusions = reviews
        .iter()
        .filter(|review| current(review, hashes, parser_tree))
        .flat_map(|review| &review.exclusions);
    let forms: BTreeSet<String> = exclusions.clone().filter_map(|e| e.form.clone()).collect();
    let sources: BTreeSet<String> = exclusions.filter_map(|e| e.source.clone()).collect();
    (forms, sources)
}
