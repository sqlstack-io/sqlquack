//! Review export files.

/// One exported file: the records of one case file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewFile {
    /// The path under the export directory, such as `numeric/division.slt`.
    pub path: String,
    /// The file's text.
    pub text: String,
}
