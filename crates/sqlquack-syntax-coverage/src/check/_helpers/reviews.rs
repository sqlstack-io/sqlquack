//! Which reviews are current, and what they exclude.

use std::collections::{BTreeMap, BTreeSet};

use crate::check::models::{FamilyContext, Reason};
use crate::formats::constants::{CATALOGUE_DIR, OBLIGATIONS_DIR, SOURCES_FILE};
use crate::formats::models::{Review, ReviewVerdict, Source, SourceStatus};

/// Whether `review` is approved and current: by the form's definition (D118), else its files.
fn current(
    review: &Review,
    hashes: &BTreeMap<String, String>,
    parser_tree: Option<&str>,
    form: Option<(&str, &str, &str)>,
) -> bool {
    if let Some((form, fingerprint, definition)) = form
        && let Some(recorded) = review.definitions.get(form)
    {
        return review.verdict == ReviewVerdict::Approved
            && recorded == definition
            && review.fingerprints.get(form).map(String::as_str) == Some(fingerprint);
    }
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
    let recorded: Option<bool> = form.and_then(|(form, fingerprint, _)| {
        review
            .fingerprints
            .get(form)
            .map(|recorded| recorded == fingerprint)
    });
    let tree: bool = match review.family.as_str() {
        "" => true,
        _ => recorded
            .unwrap_or(parser_tree.is_some() && parser_tree == review.parser_tree.as_deref()),
    };
    review.verdict == ReviewVerdict::Approved && files && tree
}

/// Whether `review` names `form`: in its `forms` list, or as a review of the form's whole family.
fn names(review: &Review, form: &str) -> bool {
    form.split('.').nth(1) == Some(review.family.as_str())
        && review
            .forms
            .as_ref()
            .is_none_or(|forms| forms.iter().any(|listed| listed == form))
}

/// Why `form`, with its fingerprint and definition hash, lacks a current review, if it does.
pub(crate) fn review_reason(
    form: &str,
    (fingerprint, definition): (&str, &str),
    context: &FamilyContext,
) -> Option<Reason> {
    let (reviews, hashes, parser_tree) = (context.reviews, context.hashes, context.parser_tree);
    let mut named = reviews
        .iter()
        .filter(|review| names(review, form))
        .peekable();
    if named.peek().is_none() {
        return Some(Reason::Unreviewed);
    }
    (!named.any(|review| {
        current(
            review,
            hashes,
            parser_tree,
            Some((form, fingerprint, definition)),
        )
    }))
    .then_some(Reason::StaleReview)
}

/// The form and source ids that current approved reviews exclude.
pub(crate) fn current_exclusions(
    reviews: &[Review],
    hashes: &BTreeMap<String, String>,
    parser_tree: Option<&str>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let exclusions = reviews
        .iter()
        .filter(|review| current(review, hashes, parser_tree, None))
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
