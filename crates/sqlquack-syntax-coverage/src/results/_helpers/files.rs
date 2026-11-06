//! The link, list and review files, read and checked against the catalogue.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::de::DeserializeOwned;

use crate::formats::main::content_sha256::content_sha256;
use crate::formats::main::is_utc_timestamp::is_utc_timestamp;
use crate::results::_helpers::obligations::load_obligations;
use crate::results::constants::{
    FORM_LINKS_SUBJECT, FUNCTION_LINKS_SUBJECT, LINKS_FILE, NOT_TRANSLATABLE_FILE,
    NOT_TRANSLATABLE_SUBJECT, RESULTS_REVIEW_FILE,
};
use crate::results::models::{
    LinksFile, NotTranslatableFile, NotTranslatableGroup, ResultsFiles, ResultsReviewFile,
};

/// The sign-off scopes: every link read, or a sample of generated ones.
const SCOPES: [&str; 2] = ["full", "sample"];

/// A file's text, empty when absent, or the problem reading it.
fn read_text(path: &Path) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn parse<T: DeserializeOwned>(path: &Path, text: &str) -> Result<T, String> {
    toml::from_str(text).map_err(|error| format!("{}: {error}", path.display()))
}

/// A construct key: a kind, a colon and a name, without whitespace.
fn is_construct(key: &str) -> bool {
    key.split_once(':').is_some_and(|(kind, name)| {
        !kind.is_empty() && !name.is_empty() && !key.chars().any(char::is_whitespace)
    })
}

/// A lower-case slug of letters, digits and hyphens.
fn is_slug(id: &str) -> bool {
    !id.is_empty()
        && id.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

/// Every problem with the links: malformed keys, unknown forms, or a construct both linked and not.
fn check_links(links: &LinksFile, catalogued: &BTreeSet<&str>) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    let mut linked: BTreeSet<&str> = BTreeSet::new();
    for link in &links.links {
        let name = link.constructs.join(" + ");
        if link.constructs.is_empty() || link.forms.is_empty() {
            problems.push(format!("link {name}: needs constructs and forms"));
        }
        for construct in &link.constructs {
            if !is_construct(construct) {
                problems.push(format!("link {name}: {construct} is not a construct key"));
            }
            linked.insert(construct);
        }
        for form in &link.forms {
            if !catalogued.contains(form.as_str()) {
                problems.push(format!("link {name}: {form} is not a catalogued form"));
            }
        }
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for no_form in &links.no_forms {
        if no_form.reason.trim().is_empty() {
            problems.push(format!(
                "no_form {}: needs a reason",
                no_form.constructs.join(", ")
            ));
        }
        for construct in &no_form.constructs {
            if !is_construct(construct) {
                problems.push(format!("no_form: {construct} is not a construct key"));
            }
            if linked.contains(construct.as_str()) {
                problems.push(format!("no_form: {construct} also links to forms"));
            }
            if !seen.insert(construct) {
                problems.push(format!("no_form: {construct} listed twice"));
            }
        }
    }
    problems
}

/// Every problem with the list: ids, reasons, unknown forms, or a form in two groups.
fn check_list(list: &NotTranslatableFile, catalogued: &BTreeSet<&str>) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    let mut listed: BTreeMap<&str, &str> = BTreeMap::new();
    for group in &list.groups {
        if !is_slug(&group.id) || !ids.insert(&group.id) {
            problems.push(format!(
                "not-translatable {}: needs a distinct lower-case slug id",
                group.id
            ));
        }
        if group.reason.trim().is_empty() || group.forms.is_empty() {
            problems.push(format!(
                "not-translatable {}: needs a reason and forms",
                group.id
            ));
        }
        for form in &group.forms {
            if !catalogued.contains(form.as_str()) {
                problems.push(format!(
                    "not-translatable {}: {form} is not a catalogued form",
                    group.id
                ));
            }
            if let Some(other) = listed.insert(form, &group.id) {
                problems.push(format!(
                    "not-translatable {}: {form} is also in {other}",
                    group.id
                ));
            }
        }
    }
    problems
}

/// Every sign-off with a malformed time or scope.
fn check_review(review: &ResultsReviewFile) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    for signoff in &review.signoffs {
        if !is_utc_timestamp(&signoff.reviewed_utc) {
            problems.push(format!(
                "signoff {}: reviewed_utc is not UTC",
                signoff.subject
            ));
        }
        if !SCOPES.contains(&signoff.scope.as_str()) {
            problems.push(format!(
                "signoff {}: scope is full or sample",
                signoff.subject
            ));
        }
    }
    problems
}

/// The files under `directory`, or every problem with them; ids must be catalogued forms.
pub(crate) fn load_files(
    directory: &Path,
    families: &BTreeMap<&str, &str>,
) -> Result<ResultsFiles, Vec<String>> {
    let catalogued: &BTreeSet<&str> = &families.keys().copied().collect();
    let read = |name: &str| {
        let path = directory.join(name);
        read_text(&path).map(|text| (path, text))
    };
    let ((links_path, links_text), (list_path, list_text), (review_path, review_text)) = match (
        read(LINKS_FILE),
        read(NOT_TRANSLATABLE_FILE),
        read(RESULTS_REVIEW_FILE),
    ) {
        (Ok(links), Ok(list), Ok(review)) => (links, list, review),
        (links, list, review) => {
            return Err([links.err(), list.err(), review.err()]
                .into_iter()
                .flatten()
                .collect());
        }
    };
    let links: LinksFile = parse(&links_path, &links_text).map_err(|problem| vec![problem])?;
    let not_translatable: NotTranslatableFile =
        parse(&list_path, &list_text).map_err(|problem| vec![problem])?;
    let review: ResultsReviewFile =
        parse(&review_path, &review_text).map_err(|problem| vec![problem])?;
    let problems: Vec<String> = [
        check_links(&links, catalogued),
        check_list(&not_translatable, catalogued),
        check_review(&review),
    ]
    .concat();
    if !problems.is_empty() {
        return Err(problems);
    }
    let obligations = load_obligations(directory, families)?;
    Ok(ResultsFiles {
        links,
        not_translatable,
        review,
        links_sha256: content_sha256(links_text.as_bytes()),
        obligations,
    })
}

/// The SHA-256 a group's sign-off binds: its reason, then each form on its own line.
pub(crate) fn group_sha256(group: &NotTranslatableGroup) -> String {
    let mut text: String = group.reason.clone();
    for form in &group.forms {
        text.push('\n');
        text.push_str(form);
    }
    content_sha256(text.as_bytes())
}

/// Whether the auditor signed off `subject` at `sha256`.
pub(crate) fn signed(review: &ResultsReviewFile, subject: &str, sha256: &str) -> bool {
    review
        .signoffs
        .iter()
        .any(|signoff| signoff.subject == subject && signoff.sha256 == sha256)
}

/// Each sign-off subject, signed or not: the two link indexes and every list group.
pub(crate) fn subjects(files: &ResultsFiles, function_links_sha256: &str) -> Vec<(String, bool)> {
    let mut subjects: Vec<(String, bool)> = vec![
        (
            FORM_LINKS_SUBJECT.to_owned(),
            signed(&files.review, FORM_LINKS_SUBJECT, &files.links_sha256),
        ),
        (
            FUNCTION_LINKS_SUBJECT.to_owned(),
            signed(&files.review, FUNCTION_LINKS_SUBJECT, function_links_sha256),
        ),
    ];
    for group in &files.not_translatable.groups {
        let subject = format!("{NOT_TRANSLATABLE_SUBJECT}{}", group.id);
        let current = signed(&files.review, &subject, &group_sha256(group));
        subjects.push((subject, current));
    }
    subjects
}
