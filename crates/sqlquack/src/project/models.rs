//! What reading a project's files keeps track of.

use std::path::PathBuf;

use serde_json::Value;

/// One table or function a file declares, kept to name both files of a clash.
#[derive(Debug)]
pub struct Declaration {
    /// `table` or `function`.
    pub kind: &'static str,
    /// The name as declared.
    pub name: String,
    /// The file declaring it.
    pub path: PathBuf,
}

/// The tables one source file declares, and a warning for each source it leaves out.
#[derive(Debug, Default)]
pub struct SourceTables {
    /// Upper-case table names and their `[column, type]` pairs.
    pub tables: Vec<(String, Value)>,
    /// One message per source left out, naming the file, the source and the column.
    pub warnings: Vec<String>,
}
