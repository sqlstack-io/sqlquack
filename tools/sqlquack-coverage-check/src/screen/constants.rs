//! The screen's allowlist file and what its comparison ignores.

/// The collapses a root accepts, each with its reason, relative to the root.
pub const ALLOWLIST_FILE: &str = "screen.toml";
/// Keywords whose spellings the grammar may tell apart, each with the one it is swapped for.
pub const SWAPPED_KEYWORDS: [(&str, &str); 2] = [("TEMP", "TEMPORARY"), ("TEMPORARY", "TEMP")];
/// Tree keys that record where text was or what comments said, not what it wrote.
pub const IGNORED_KEYS: [&str; 2] = ["span", "trailing_comments"];
