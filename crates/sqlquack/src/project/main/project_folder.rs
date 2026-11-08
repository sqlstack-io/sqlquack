//! `project_folder`: a project folder beside the policy file, when it exists.

use std::path::{Path, PathBuf};

/// `configured` or `default` under `root`; None when the default is absent, an error when the configured one is.
pub fn project_folder(
    root: &Path,
    configured: Option<&str>,
    default: &str,
) -> Result<Option<PathBuf>, String> {
    let folder = root.join(configured.unwrap_or(default));
    match (folder.is_dir(), configured) {
        (true, _) => Ok(Some(folder)),
        (false, None) => Ok(None),
        (false, Some(_)) => Err(format!(
            "[project] names the folder {}, which does not exist",
            folder.display()
        )),
    }
}
