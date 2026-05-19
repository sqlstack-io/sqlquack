use std::path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub code: &'static str,
    pub path: path::PathBuf,
    pub line: Option<usize>,
    pub message: String,
}

impl Violation {
    pub(crate) fn new(
        code: &'static str,
        path: &path::Path,
        line: Option<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: path.to_path_buf(),
            line,
            message: message.into(),
        }
    }
}
