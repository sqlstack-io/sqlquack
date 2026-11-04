//! Syntax-coverage paths, closed vocabularies and id shapes.

/// The coverage root, relative to the repository.
pub const COVERAGE_ROOT: &str = "coverage/syntax";
/// Pinned documentation pages.
pub const SOURCES_FILE: &str = "sources.toml";
/// The documentation indexes the pages were read from.
pub const INDEXES_FILE: &str = "indexes.toml";
/// Renamed form ids; optional until the first rename.
pub const ALIASES_FILE: &str = "aliases.toml";
/// One TOML file per family.
pub const CATALOGUE_DIR: &str = "catalogue";
/// One TOML file per family.
pub const OBLIGATIONS_DIR: &str = "obligations";
/// One JSON Lines file per family.
pub const CASES_DIR: &str = "cases";
/// Normalized receipts in JSON Lines files.
pub const RECEIPTS_DIR: &str = "receipts";
/// Review records in TOML files; optional until the first review.
pub const REVIEWS_DIR: &str = "reviews";
/// Obligations Snowflake answers only with an internal error, one optional TOML file per family.
pub const UNOBSERVABLE_DIR: &str = "unobservable";
/// Every family, each with a catalogue, an obligation and a case file.
pub const FAMILIES: [&str; 10] = [
    "query",
    "dml",
    "ddl",
    "transaction_session",
    "security_admin",
    "utility",
    "scripting",
    "functions",
    "expressions",
    "classes",
];
/// `sf`, the family and the page slug.
pub const FORM_ID_MIN_PARTS: usize = 3;
/// Every form id starts with this.
pub const FORM_PREFIX: &str = "sf";
/// Every source id starts with this.
pub const SOURCE_PREFIX: &str = "doc";
/// Separates a definition's source from its name.
pub const DEFINITION_SEPARATOR: &str = "::";
/// Joins a form's productions before hashing.
pub const GRAMMAR_JOIN: &str = " || ";
/// A prose form's grammar hash is this followed by its heading.
pub const PROSE_HASH_PREFIX: &str = "prose:";
/// The obligation marker and its digits, as in `o001`.
pub const OBLIGATION_MARKER: char = 'o';
pub const OBLIGATION_DIGITS: usize = 3;
/// The obligation number never used.
pub const OBLIGATION_ZERO: &str = "000";
/// Members of an interaction obligation.
pub const INTERACTION_MEMBERS: usize = 2;
/// The format every receipt declares.
pub const RECEIPT_FORMAT: &str = "sqlquack-syntax-receipt/1";
/// The format every coverage snapshot, `baseline.json` included, declares.
pub const SNAPSHOT_FORMAT: &str = "sqlquack-syntax-coverage-snapshot/1";
/// The committed baseline the CI gate diffs against, relative to the coverage root.
pub const BASELINE_FILE: &str = "baseline.json";
/// The run-report list of receipt lines the importer withdrew, each by `receipt_sha256`.
pub const REPORT_WITHDRAWN: &str = "withdrawn";
/// The field of a withdrawn entry naming the receipt line's SHA-256.
pub const REPORT_RECEIPT_SHA256: &str = "receipt_sha256";
/// The importer's run report beside each receipts file.
pub const RECEIPT_REPORT_SUFFIX: &str = ".report.json";
/// The longest case id.
pub const CASE_ID_LIMIT: usize = 200;
/// Every second-precision timestamp has this shape, each `0` standing for any digit.
pub const UTC_SHAPE: &str = "0000-00-00T00:00:00Z";
/// The byte in `UTC_SHAPE` standing for any digit.
pub const DIGIT_PLACEHOLDER: u8 = b'0';
/// The UTC offsets an RFC 3339 receipt timestamp may end with.
pub const UTC_SUFFIXES: [&str; 2] = ["+00:00", "Z"];
/// Hex characters in a SHA-256 digest.
pub const SHA256_LENGTH: usize = 64;
/// Hex characters in a Git tree id.
pub const GIT_TREE_LENGTH: usize = 40;
/// The only URL scheme a source may use.
pub const HTTPS: &str = "https://";
/// Text that would identify an account, host or person; never allowed in a receipt.
pub const RECEIPT_FORBIDDEN_TEXT: [&str; 3] = ["@", "snowflakecomputing", "://"];

/// What precedes the line number in a DCM inner error, as in `syntax error line 1 at position 61`.
pub const DCM_LINE: &str = "line ";

/// What separates an inner error's line from its position.
pub const DCM_POSITION: &str = " at position ";
