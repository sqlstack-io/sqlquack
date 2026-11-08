//! `directory_corpus`: every `.sql` file below a directory.

use std::path::Path;

use sqlquack_syntax::Dialect;
use walkdir::WalkDir;

use crate::corpora::constants::SQL_EXTENSION;
use crate::corpora::models::CorpusEntry;

/// One entry per `.sql` file below `root`, in path order, parsed in `dialect`.
pub fn directory_corpus(root: &Path, dialect: Dialect) -> Result<Vec<CorpusEntry>, String> {
    let mut entries: Vec<CorpusEntry> = Vec::new();
    for item in WalkDir::new(root).sort_by_file_name() {
        let item = item.map_err(|error| format!("cannot walk {}: {error}", root.display()))?;
        let path = item.path();
        if !item.file_type().is_file() || path.extension().is_none_or(|ext| ext != SQL_EXTENSION) {
            continue;
        }
        let sql = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let from = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        entries.push(CorpusEntry { dialect, sql, from });
    }
    Ok(entries)
}
