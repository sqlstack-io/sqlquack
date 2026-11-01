//! `with_overrides`: conversion options with the user overrides a command reads.

use std::path::Path;

use sqlquack_rules::conversion::models::ConversionOptions;
use walkdir::WalkDir;

use crate::analysis::_helpers::override_definition::{load_override, verified_override};
use crate::analysis::constants::OVERRIDE_EXTENSION;

/// `options` with the overrides at `path`, a `.sql` file or a directory, and `NAME=<sha256>` verified.
pub fn with_overrides(
    mut options: ConversionOptions,
    (path, verified): (Option<&Path>, &[String]),
) -> Result<ConversionOptions, String> {
    for named in verified {
        let (name, sha256) = verified_override(named)?;
        options.verified_overrides.insert(name, sha256);
    }
    let Some(path) = path else {
        return Ok(options);
    };
    let mut files: Vec<std::path::PathBuf> = WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .map(walkdir::DirEntry::into_path)
        .filter(|file| {
            file.is_file()
                && file
                    .extension()
                    .is_some_and(|extension| extension == OVERRIDE_EXTENSION)
        })
        .collect();
    files.sort();
    for file in files {
        let text = std::fs::read_to_string(&file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        let name = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let (replaces, replacement) = load_override(name, &text)?;
        if let Some(earlier) = options.overrides.insert(replaces.clone(), replacement) {
            return Err(format!(
                "overrides {} and {name} both replace {replaces}",
                earlier.name
            ));
        }
    }
    Ok(options)
}
