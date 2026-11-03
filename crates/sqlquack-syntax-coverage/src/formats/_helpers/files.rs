//! Reading the coverage files; every failure becomes a named problem.

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::formats::_helpers::hashes::sha256_hex;
use crate::formats::constants::FAMILIES;

/// A file's text and the SHA-256 of its bytes.
pub(crate) fn read_text(path: &Path) -> Result<(String, String), String> {
    let bytes: Vec<u8> =
        std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let text: String =
        String::from_utf8(bytes).map_err(|_| format!("{}: not UTF-8", path.display()))?;
    let hash: String = sha256_hex(text.as_bytes());
    Ok((text, hash))
}

/// A TOML file and the SHA-256 of its bytes.
pub(crate) fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<(T, String), String> {
    let (text, hash) = read_text(path)?;
    toml::from_str(&text)
        .map(|value| (value, hash))
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// Each line as one record; a blank line is a problem, so a line number names one record.
pub(crate) fn read_jsonl<T: DeserializeOwned>(path: &Path) -> (Vec<T>, Vec<String>) {
    let text: String = match read_text(path) {
        Ok((text, _)) => text,
        Err(problem) => return (Vec::new(), vec![problem]),
    };
    let mut records: Vec<T> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        match serde_json::from_str(line) {
            Ok(record) => records.push(record),
            Err(error) => problems.push(format!("{}:{}: {error}", path.display(), index + 1)),
        }
    }
    (records, problems)
}

/// The files in `directory` with `extension`, sorted, then every other entry.
pub(crate) fn listed(directory: &Path, extension: &str) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut others: Vec<PathBuf> = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return (paths, others);
    };
    for entry in entries.filter_map(Result::ok) {
        let path: PathBuf = entry.path();
        if path.is_file() && path.extension().is_some_and(|found| found == extension) {
            paths.push(path);
        } else {
            others.push(path);
        }
    }
    paths.sort();
    others.sort();
    (paths, others)
}

/// The problem a stray directory entry causes.
pub(crate) fn stray(path: &Path, extension: &str) -> String {
    format!("{}: not a .{extension} file", path.display())
}

/// One file per family in `directory`; a missing family or a file naming no family is a problem.
pub(crate) fn family_files(
    root: &Path,
    directory: &str,
    extension: &str,
) -> (Vec<(String, PathBuf)>, Vec<String>) {
    let (paths, others) = listed(&root.join(directory), extension);
    let mut problems: Vec<String> = others.iter().map(|path| stray(path, extension)).collect();
    let mut found: Vec<(String, PathBuf)> = Vec::new();
    for path in paths {
        let stem: String = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();
        if FAMILIES.contains(&stem.as_str()) {
            found.push((stem, path));
        } else {
            problems.push(format!("{directory}/{stem}.{extension}: names no family"));
        }
    }
    for family in FAMILIES {
        if !found.iter().any(|(stem, _)| stem == family) {
            problems.push(format!("{directory}/{family}.{extension}: missing"));
        }
    }
    (found, problems)
}
