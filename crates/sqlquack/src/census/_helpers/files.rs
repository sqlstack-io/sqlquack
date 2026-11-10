//! The SQL files under the census paths.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::census::errors::CensusError;

const SQL_EXTENSION: &str = "sql";

pub(crate) fn collect_sql_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, CensusError> {
    let mut files: Vec<PathBuf> = Vec::new();
    for path in paths {
        if path.is_file() {
            files.push(path.clone());
            continue;
        }
        if !path.is_dir() {
            return Err(CensusError::Read {
                path: path.clone(),
                message: "no such file or directory".to_owned(),
            });
        }
        for entry in WalkDir::new(path).follow_links(true) {
            let entry = entry.map_err(|error| CensusError::Read {
                path: error.path().map_or_else(|| path.clone(), Path::to_path_buf),
                message: error.to_string(),
            })?;
            if entry.file_type().is_file() && is_sql(entry.path()) {
                files.push(entry.into_path());
            }
        }
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        return Err(CensusError::NoInputs);
    }
    Ok(files)
}

/// The model a file defines: its file name without extension.
pub(crate) fn model_name(path: &Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

fn is_sql(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(SQL_EXTENSION))
}
