//! Tables declared in a project's source YAML files, with the file named in errors and warnings.

use std::path::Path;

use sqlquack_semantics::types::main::source_tables::source_tables;
use sqlquack_semantics::types::models::SourceTables;

/// Each table the file at `path` declares, and warnings naming the file.
pub(crate) fn source_relations(path: &Path) -> Result<SourceTables, String> {
    let failed = |reason: String| format!("{}: {reason}", path.display());
    let text = std::fs::read_to_string(path).map_err(|error| failed(error.to_string()))?;
    let mut found = source_tables(&text).map_err(failed)?;
    found.warnings = found.warnings.into_iter().map(failed).collect();
    Ok(found)
}
