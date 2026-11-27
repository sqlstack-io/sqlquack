//! `with_overrides`: conversion options with the user overrides a command reads.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sqlquack_convert::snowflake::conversion::models::ConversionOptions;
use walkdir::WalkDir;

use crate::analysis::_helpers::override_definition::{load_override, verified_override};
use crate::analysis::constants::OVERRIDE_EXTENSION;

/// `options` with the overrides at `path`, a `.sql` file or a directory, and `NAME=<sha256>` verified; errors name the files.
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
    let mut read_from: BTreeMap<String, PathBuf> = BTreeMap::new();
    for file in files {
        let text = std::fs::read_to_string(&file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        let name = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let (replaces, replacement) =
            load_override(name, &text).map_err(|error| format!("{}: {error}", file.display()))?;
        if let Some(earlier) = read_from.insert(replaces.clone(), file.clone()) {
            return Err(format!(
                "overrides {} and {} both replace {replaces}",
                earlier.display(),
                file.display()
            ));
        }
        options.overrides.insert(replaces, replacement);
    }
    Ok(options)
}
