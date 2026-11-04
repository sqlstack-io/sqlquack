//! Which reviews are current, and what they exclude.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::models::Reason;
use crate::formats::constants::{CATALOGUE_DIR, OBLIGATIONS_DIR, SOURCES_FILE};
use crate::formats::models::{Review, ReviewVerdict, Source, SourceStatus};

/// Whether `review` is approved and bound to the current files, and a family review to the tree.
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
    let tree: bool = match review.family.as_str() {
        "" => true,
        _ => parser_tree.is_some() && parser_tree == review.parser_tree.as_deref(),
    };
    review.verdict == ReviewVerdict::Approved && files && tree
}

/// Why `form` of `family` lacks a current review, if it does. A review with `forms` names only
/// those forms; one without names the whole family.
pub(crate) fn review_reason(
    form: &str,
    family: &str,
    reviews: &[Review],
    hashes: &BTreeMap<String, String>,
    parser_tree: Option<&str>,
) -> Option<Reason> {
    let mut named = reviews
        .iter()
        .filter(|review| {
            review.family == family
                && review
                    .forms
                    .as_ref()
                    .is_none_or(|forms| forms.iter().any(|listed| listed == form))
        })
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

/// Pages without forms of their own or a restating form that no current review excludes.
pub(crate) fn unexcluded_pages(sources: &[Source], excluded: &BTreeSet<String>) -> Vec<String> {
    sources
        .iter()
        .filter(|source| {
            !matches!(source.status, SourceStatus::Forms | SourceStatus::Restated)
                && !excluded.contains(&source.id)
        })
        .map(|source| source.id.clone())
        .collect()
}
