//! `load_domains`: every declared rule domain, from a directory of TOML files.

use std::path::Path;

use crate::coverage::constants::{DOMAIN_EXTENSION, DOMAINS_VERSION};
use crate::coverage::models::{DomainFile, RuleDomain};
use crate::errors::EvidenceError;

/// The rule domains declared in `directory`'s TOML files, in file and declaration order.
pub fn load_domains(directory: &Path) -> Result<Vec<RuleDomain>, EvidenceError> {
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(directory)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", directory.display())))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == DOMAIN_EXTENSION)
        })
        .collect();
    paths.sort();
    let mut domains: Vec<RuleDomain> = Vec::new();
    let mut problems: Vec<String> = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(&path)
            .map_err(|error| EvidenceError::Io(format!("{}: {error}", path.display())))?;
        match toml::from_str::<DomainFile>(&text) {
            Ok(file) if file.version == DOMAINS_VERSION => domains.extend(file.rules),
            Ok(file) => problems.push(format!(
                "{}: domain version {} is not {DOMAINS_VERSION}",
                path.display(),
                file.version
            )),
            Err(error) => problems.push(format!("{}: {error}", path.display())),
        }
    }
    let mut ids: Vec<&str> = domains.iter().map(|domain| domain.id.as_str()).collect();
    ids.sort_unstable();
    for pair in ids.windows(2).filter(|pair| pair[0] == pair[1]) {
        problems.push(format!("rule {} is declared twice", pair[0]));
    }
    match problems.is_empty() {
        true => Ok(domains),
        false => Err(EvidenceError::InvalidCases(problems)),
    }
}
