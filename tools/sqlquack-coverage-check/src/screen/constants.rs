//! The screen's allowlist file and what its comparison ignores.

/// The collapses a root accepts, each with its reason, relative to the root.
pub const ALLOWLIST_FILE: &str = "screen.toml";
/// Keyword spellings the grammar lists as alternatives of one token, each group swapped in turn.
pub const SYNONYMS: [&[&str]; 29] = [
    &["TEMP", "TEMPORARY"],
    &["DATABASE", "SCHEMA"],
    &["DATABASES", "SCHEMAS"],
    &["DBPROPERTIES", "PROPERTIES"],
    &["FROM", "IN"],
    &["RLIKE", "REGEXP"],
    &["EXCEPT", "MINUS"],
    &["ANY", "SOME"],
    &["DESC", "DESCRIBE"],
    &["ALTER", "CHANGE"],
    &["INT", "INTEGER"],
    &["BIGINT", "LONG"],
    &["SMALLINT", "SHORT"],
    &["TINYINT", "BYTE"],
    &["FLOAT", "REAL"],
    &["DECIMAL", "DEC", "NUMERIC"],
    &["CHAR", "CHARACTER"],
    &["YEAR", "YEARS"],
    &["MONTH", "MONTHS"],
    &["WEEK", "WEEKS"],
    &["DAY", "DAYS"],
    &["HOUR", "HOURS"],
    &["MINUTE", "MINUTES"],
    &["SECOND", "SECONDS"],
    &["MILLISECOND", "MILLISECONDS"],
    &["MICROSECOND", "MICROSECONDS"],
    &["<>", "!="],
    &["=", "=="],
    &["NOT", "!"],
];
/// Tree keys that record where text was or what comments said, not what it wrote.
pub const IGNORED_KEYS: [&str; 2] = ["span", "trailing_comments"];
