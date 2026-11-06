//! What reading a project's files keeps track of.

use std::path::PathBuf;

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
