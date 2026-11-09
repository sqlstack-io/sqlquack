//! `load_hazards`: the hazard catalogue.

use std::path::Path;

use crate::coverage::constants::CATALOGUE_VERSION;
use crate::coverage::models::HazardCatalogue;
use crate::errors::EvidenceError;

/// The hazard catalogue at `path`, when its version is the one this build reads.
pub fn load_hazards(path: &Path) -> Result<HazardCatalogue, EvidenceError> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| EvidenceError::Io(format!("{}: {error}", path.display())))?;
    let catalogue: HazardCatalogue = toml::from_str(&text).map_err(|error| {
        EvidenceError::InvalidCases(vec![format!("{}: {error}", path.display())])
    })?;
    match catalogue.version == CATALOGUE_VERSION {
        true => Ok(catalogue),
        false => Err(EvidenceError::InvalidCases(vec![format!(
            "{}: catalogue version {} is not {CATALOGUE_VERSION}",
            path.display(),
            catalogue.version
        )])),
    }
}
