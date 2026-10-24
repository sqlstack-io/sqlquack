//! Evidence layout and formats.

/// Authored case files, relative to the repository root.
pub const CASES_DIR: &str = "cases";

/// Recorded results: append-only, one `<hash>.json` and usually one `<hash>.arrow` per case.
pub const RESULTS_DIR: &str = "evidence/results";

/// Pending case files for the recorder; derived, never committed.
pub const PENDING_DIR: &str = "evidence/pending";

/// Pending audit cases; derived, never committed.
pub const AUDIT_PENDING_DIR: &str = "evidence/audit/pending";

/// Fresh audit recordings; never committed.
pub const AUDIT_RESULTS_DIR: &str = "evidence/audit/results";

/// The version of the case, pending and result file formats.
pub const EVIDENCE_FORMAT: u32 = 1;

/// The source dialect name recorded in hashes and pending files.
pub const SOURCE_DIALECT: &str = "snowflake";

/// The input column holding the row number; expression results are ordered by it.
pub const ORDINAL_COLUMN: &str = "ordinal";

/// The single result column of expression cases.
pub const VALUE_COLUMN: &str = "value";

/// The relation that holds the case input rows.
pub const INPUT_RELATION: &str = "input";

/// The extension of authored case files.
pub const CASE_FILE_EXTENSION: &str = "toml";

/// The extension of metadata and pending files.
pub const METADATA_EXTENSION: &str = "json";

/// The length of a case hash in hexadecimal digits.
pub const CASE_HASH_LENGTH: usize = 64;
