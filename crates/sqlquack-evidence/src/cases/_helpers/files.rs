//! Reading case files.

use std::path::Path;

use walkdir::WalkDir;

use crate::cases::models::CaseFile;
use crate::errors::EvidenceError;

pub(crate) fn read_case_files(directory: &Path) -> Result<Vec<(String, CaseFile)>, EvidenceError> {
    let mut files: Vec<(String, CaseFile)> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for entry in WalkDir::new(directory).sort_by_file_name() {
        let entry = entry.map_err(|error| EvidenceError::Io(error.to_string()))?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().is_none_or(|extension| extension != "toml")
        {
            continue;
        }
        let relative = path
            .strip_prefix(directory)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(path)
            .map_err(|error| EvidenceError::Io(format!("{}: {error}", path.display())))?;
        match toml::from_str::<CaseFile>(&text) {
            Ok(file) => files.push((relative, file)),
            Err(error) => problems.push(format!("{relative}: {error}")),
        }
    }
    if problems.is_empty() {
        Ok(files)
    } else {
        Err(EvidenceError::InvalidCases(problems))
    }
}
