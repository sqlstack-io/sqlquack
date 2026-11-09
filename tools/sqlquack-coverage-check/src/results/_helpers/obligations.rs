//! The per-family obligation files, read fail-closed, and each obligation's state.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::results::constants::{
    BOUNDARY_SUBKINDS, OBLIGATION_CASES_PREFIX, OBLIGATION_FILE_EXTENSION, OBLIGATIONS_DIR,
    RETIRED_OBLIGATIONS_FILE, TRANSLATION_CASES,
};
use crate::results::models::{
    CaseFileHeader, ObligationKind, ObligationState, ObligationsFile, RecordedStatement,
    RetiredFile, TranslationObligation,
};

/// Every obligation under `directory/obligations` and every retired id, or every problem.
pub(crate) fn load_obligations(
    (directory, repository): (&Path, &Path),
    catalogued: &BTreeMap<&str, &str>,
) -> Result<(Vec<TranslationObligation>, BTreeSet<String>), Vec<String>> {
    let folder = directory.join(OBLIGATIONS_DIR);
    let entries = std::fs::read_dir(&folder)
        .map_err(|error| vec![format!("{}: {error}", folder.display())])?;
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries {
        paths.push(
            entry
                .map_err(|error| vec![format!("{}: {error}", folder.display())])?
                .path(),
        );
    }
    paths.sort();
    let families: BTreeSet<&str> = catalogued.values().copied().collect();
    let mut problems: Vec<String> = Vec::new();
    let mut obligations: Vec<TranslationObligation> = Vec::new();
    for path in paths {
        let label = path.display().to_string();
        let family = match (path.extension(), path.file_stem()) {
            (Some(extension), Some(stem)) if extension == OBLIGATION_FILE_EXTENSION => {
                stem.to_string_lossy()
            }
            _ => {
                problems.push(format!("{label}: not a <family>.toml obligation file"));
                continue;
            }
        };
        if !families.contains(family.as_ref()) {
            problems.push(format!("{label}: {family} is not a catalogue family"));
            continue;
        }
        let parsed = std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                toml::from_str::<ObligationsFile>(&text).map_err(|error| error.to_string())
            });
        match parsed {
            Ok(file) => {
                for obligation in file.obligations {
                    problems.extend(
                        obligation_problems(&obligation, (&family, catalogued))
                            .into_iter()
                            .map(|problem| format!("obligation {}: {problem}", obligation.id)),
                    );
                    obligations.push(obligation);
                }
            }
            Err(error) => problems.push(format!("{label}: {error}")),
        }
    }
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for obligation in &obligations {
        if !ids.insert(&obligation.id) {
            problems.push(format!("obligation {}: declared twice", obligation.id));
        }
    }
    let (retired, retired_problems) = load_retired(directory, &ids);
    problems.extend(retired_problems);
    problems.extend(construct_problems(repository, &obligations));
    match problems.is_empty() {
        true => Ok((obligations, retired)),
        false => Err(problems),
    }
}

/// Every cited case whose file is missing or whose `construct` is not the obligation's form.
fn construct_problems(repository: &Path, obligations: &[TranslationObligation]) -> Vec<String> {
    let mut constructs: BTreeMap<&str, Result<String, String>> = BTreeMap::new();
    let mut problems: Vec<String> = Vec::new();
    for obligation in obligations {
        for case in &obligation.cases {
            let file = case.split_once('#').map_or(case.as_str(), |(file, _)| file);
            let construct = constructs
                .entry(file)
                .or_insert_with(|| case_construct(&repository.join(file)));
            match construct {
                Ok(construct) if *construct == obligation.form => {}
                Ok(construct) => problems.push(format!(
                    "obligation {}: cites {case}, a case of {construct}",
                    obligation.id
                )),
                Err(problem) => problems.push(format!("obligation {}: {problem}", obligation.id)),
            }
        }
    }
    problems
}

/// The `construct` a case file names, or why it cannot be read.
fn case_construct(path: &Path) -> Result<String, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    toml::from_str::<CaseFileHeader>(&text)
        .map(|header| header.construct)
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// The retired ids, and every problem with them: reuse, a blank reason, an unresolved replacement.
fn load_retired(directory: &Path, live: &BTreeSet<&str>) -> (BTreeSet<String>, Vec<String>) {
    let path = directory.join(RETIRED_OBLIGATIONS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (BTreeSet::new(), Vec::new());
        }
        Err(error) => {
            return (
                BTreeSet::new(),
                vec![format!("{}: {error}", path.display())],
            );
        }
    };
    let file: RetiredFile = match toml::from_str(&text) {
        Ok(file) => file,
        Err(error) => {
            return (
                BTreeSet::new(),
                vec![format!("{}: {error}", path.display())],
            );
        }
    };
    let mut problems: Vec<String> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for retired in &file.retired {
        let id = retired.id.as_str();
        if !seen.insert(id) {
            problems.push(format!("retired obligation {id}: retired twice"));
        }
        if live.contains(id) {
            problems.push(format!(
                "retired obligation {id}: the id is reused in obligations/"
            ));
        }
        if retired.reason.trim().is_empty() || retired.form.trim().is_empty() {
            problems.push(format!(
                "retired obligation {id}: needs its form and a reason"
            ));
        }
        for replacement in &retired.replaced_by {
            if !live.contains(replacement.as_str()) {
                problems.push(format!(
                    "retired obligation {id}: replaced_by {replacement} is not a live obligation"
                ));
            }
        }
    }
    let ids: BTreeSet<String> = file.retired.into_iter().map(|retired| retired.id).collect();
    (ids, problems)
}

/// Whether `text` holds something other than whitespace.
fn written(text: Option<&String>) -> bool {
    text.is_some_and(|text| !text.trim().is_empty())
}

/// Every problem with one obligation in the file of `family`.
fn obligation_problems(
    obligation: &TranslationObligation,
    (family, catalogued): (&str, &BTreeMap<&str, &str>),
) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    match catalogued.get(obligation.form.as_str()) {
        None => problems.push(format!(
            "{} is not a catalogued form still in its source; retarget or remove the obligation",
            obligation.form
        )),
        Some(owner) if *owner != family => problems.push(format!(
            "{} belongs to {owner}, not {family}",
            obligation.form
        )),
        Some(_) => {}
    }
    let number = obligation
        .id
        .strip_prefix(obligation.form.as_str())
        .and_then(|rest| rest.strip_prefix(".t"));
    if !number
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
    {
        problems.push(format!("the id is not {}.t<digits>", obligation.form));
    }
    let boundary = obligation.kind == ObligationKind::Boundary;
    match (&obligation.subkind, boundary) {
        (Some(subkind), true) if BOUNDARY_SUBKINDS.contains(&subkind.as_str()) => {}
        (Some(subkind), true) => problems.push(format!("subkind {subkind} is not a boundary kind")),
        (None, true) => problems.push("a boundary obligation needs a subkind".to_owned()),
        (Some(_), false) => problems.push("only a boundary obligation has a subkind".to_owned()),
        (None, false) => {}
    }
    let prefix = format!("{OBLIGATION_CASES_PREFIX}{TRANSLATION_CASES}{family}/");
    let mut cited: BTreeSet<&str> = BTreeSet::new();
    for case in &obligation.cases {
        if !case.starts_with(&prefix) || !case.contains(".toml#") {
            problems.push(format!("case {case} is not {prefix}<file>.toml#<id>"));
        }
        if !cited.insert(case) {
            problems.push(format!("cites {case} twice"));
        }
    }
    let empty = obligation.cases.is_empty();
    let (held, reason) = (obligation.held.as_ref(), obligation.reason.as_ref());
    let shape = match (obligation.required, empty) {
        (true, false) if held.is_none() && reason.is_none() => None,
        (true, false) => Some("a required obligation with cases has neither held nor reason"),
        (true, true) if reason.is_some() => {
            Some("a required obligation has no reason; held says why it has no case")
        }
        (true, true) if !written(held) => {
            Some("a required obligation with no cases needs a non-empty held reason")
        }
        (true, true) => None,
        (false, _) if held.is_some() => Some("held is for required obligations"),
        (false, false) => Some("an obligation that is not required cites no cases"),
        (false, true) if !written(reason) => {
            Some("an obligation that is not required needs a non-empty reason")
        }
        (false, true) => None,
    };
    problems.extend(shape.map(str::to_owned));
    problems
}

/// Each obligation's state; a cited case no statement records is a problem.
pub(crate) fn obligation_states(
    obligations: &[TranslationObligation],
    statements: &[RecordedStatement],
) -> Result<BTreeMap<String, ObligationState>, Vec<String>> {
    let outcomes: BTreeMap<&str, _> = statements
        .iter()
        .map(|statement| (statement.reference.as_str(), statement.outcome))
        .collect();
    let mut problems: Vec<String> = Vec::new();
    let mut states: BTreeMap<String, ObligationState> = BTreeMap::new();
    for obligation in obligations {
        let mut met = !obligation.cases.is_empty();
        for case in &obligation.cases {
            let reference = case.strip_prefix(OBLIGATION_CASES_PREFIX).unwrap_or(case);
            match outcomes.get(reference) {
                Some(outcome) => met &= outcome.matches(),
                None => problems.push(format!(
                    "obligation {}: no recorded statement is the case {case}",
                    obligation.id
                )),
            }
        }
        let state = match (obligation.required, obligation.held.is_some(), met) {
            (false, _, _) => ObligationState::Unmeasured,
            (true, true, _) => ObligationState::Held,
            (true, false, true) => ObligationState::Met,
            (true, false, false) => ObligationState::Unmet,
        };
        states.insert(obligation.id.clone(), state);
    }
    match problems.is_empty() {
        true => Ok(states),
        false => Err(problems),
    }
}

/// The recorded translation cases no obligation cites, by reference.
pub(crate) fn uncited_cases(
    obligations: &[TranslationObligation],
    statements: &[RecordedStatement],
) -> Vec<String> {
    let cited: BTreeSet<&str> = obligations
        .iter()
        .flat_map(|obligation| &obligation.cases)
        .map(|case| case.strip_prefix(OBLIGATION_CASES_PREFIX).unwrap_or(case))
        .collect();
    statements
        .iter()
        .map(|statement| statement.reference.as_str())
        .filter(|reference| reference.starts_with(TRANSLATION_CASES) && !cited.contains(reference))
        .map(str::to_owned)
        .collect()
}
