use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::classification::models::{Construct, ConstructFamily};
use crate::diagnostics::_helpers::code_entry::code_entry;
use crate::diagnostics::constants::CODES;
use crate::diagnostics::main::{diagnose::diagnose, render_diagnostics::render_diagnostics};
use crate::parsing::main::parse_source::parse_source;
use crate::parsing::models::{SourceDialect, SourceLines, SourceLocation};
use crate::refusals::main::refuse_unparsed::refuse_unparsed;
use crate::refusals::models::{Refusal, RefusalCategory};

/// A refusal of `category` at characters `start..end` of `source`.
pub(crate) fn placed(
    source: &str,
    (start, end): (usize, usize),
    category: RefusalCategory,
) -> Refusal {
    Refusal {
        construct: Some(Construct::new(ConstructFamily::Function, "EDITDISTANCE")),
        location: Some(SourceLines::new(source).location(start, end)),
        category,
        reason: "no rule converts this call".to_owned(),
    }
}

/// The refusal the parser's failure on `source` gives.
pub(crate) fn unparsed(source: &str) -> Refusal {
    refuse_unparsed(
        &parse_source(source, SourceDialect::Snowflake).expect_err("test SQL does not parse"),
        &SourceLines::new(source),
    )
}

/// The rendered text of one refusal of `source`.
pub(crate) fn rendered(source: &str, refusal: Refusal) -> String {
    render_diagnostics(&diagnose(&[refusal], source), source, "query.sql")
}

/// A refusal with no place, such as one of the whole input.
pub(crate) fn unlocated(location: Option<SourceLocation>) -> Refusal {
    Refusal {
        construct: None,
        location,
        category: RefusalCategory::ConversionBudget,
        reason: "conversion took over 5 s".to_owned(),
    }
}

/// Issued lock lines the registry lacks; with `whole_registry`, also unlocked or malformed codes.
pub(crate) fn registry_mismatches(lock: &str, whole_registry: bool) -> Vec<String> {
    let issued: Vec<(&str, &str)> = lock
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .filter_map(|line| line.split_once(' '))
        .collect();
    let missing = issued
        .iter()
        .filter(|(code, name)| {
            !CODES
                .iter()
                .any(|entry| entry.code == *code && entry.name == *name)
        })
        .map(|(code, name)| format!("{code} {name}"));
    let unlocked = CODES
        .iter()
        .filter(|_| whole_registry)
        .filter(|entry| {
            !issued.contains(&(entry.code, entry.name))
                || entry.code.len() != 6
                || !entry.code.starts_with("SQ")
                || !entry.code[2..].chars().all(|digit| digit.is_ascii_digit())
                || entry.title.trim().is_empty()
                || entry.explanation.trim().is_empty()
        })
        .map(|entry| format!("registry {} {}", entry.code, entry.name));
    let keys: Vec<String> = CODES
        .iter()
        .flat_map(|entry| [Some(entry.code), Some(entry.name), entry.alias])
        .flatten()
        .map(str::to_ascii_lowercase)
        .collect();
    let distinct: BTreeSet<&String> = keys.iter().collect();
    let duplicated =
        (whole_registry && distinct.len() != keys.len()).then(|| "registry keys repeat".to_owned());
    missing.chain(unlocked).chain(duplicated).collect()
}

/// Every file under `path`, depth first.
fn files_under(path: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .flat_map(|entry| {
            let path = entry.path();
            std::iter::once(path.clone())
                .filter(|path| path.is_file())
                .chain(files_under(&path))
                .collect::<Vec<PathBuf>>()
        })
        .collect()
}

/// The `E_GUARD_*` names in the files under `roots`, relative to this crate, with no code.
pub(crate) fn unregistered_guard_names(roots: &[&str]) -> Vec<String> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let names: BTreeSet<String> = roots
        .iter()
        .flat_map(|root| files_under(&base.join(root)))
        .flat_map(|path| {
            std::fs::read_to_string(path)
                .unwrap_or_default()
                .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
                .filter(|part| part.starts_with("E_GUARD_") && part.len() > "E_GUARD_".len())
                .map(str::to_owned)
                .collect::<Vec<String>>()
        })
        .collect();
    assert!(names.len() >= 7, "the guard names were found: {names:?}");
    names
        .into_iter()
        .filter(|name| code_entry(name).is_none())
        .collect()
}
