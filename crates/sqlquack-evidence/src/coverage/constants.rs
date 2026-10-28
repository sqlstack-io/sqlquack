//! Coverage file names, families, kinds and group matchers.

/// The directory holding the declared rule domains, one TOML file per family of rules.
pub const DOMAINS_DIR: &str = "coverage/domains";
/// The hazard catalogue.
pub const HAZARDS_FILE: &str = "coverage/hazards.toml";
/// The catalogue format version this build reads.
pub const CATALOGUE_VERSION: u32 = 3;
/// The domain format version this build reads.
pub const DOMAINS_VERSION: u32 = 1;
/// The argument kind of a literal.
pub const LITERAL_KIND: &str = "literal";
/// The argument kind of a computed value: a column or an expression.
pub const VALUE_KIND: &str = "value";
/// A facet's value when a shape records none.
pub const NO_FACET: &str = "none";
/// The input column cases group by, when present; otherwise the whole input is one group.
pub const GROUP_COLUMN: &str = "g";
/// How a source names a column.
pub const COLUMN_SOURCE: &str = "column:";
/// How a source names a literal.
pub const LITERAL_SOURCE: &str = "literal:";
/// A NULL cell.
pub const NULL_CELL: &str = "NULL";
/// The pair hazard of two cells differing only by case.
pub const CASE_ONLY_PAIR: &str = "case_only";
/// A position a one-wise combination leaves open.
pub const ANY_POSITION: &str = "_";
/// The extension of a domain file.
pub const DOMAIN_EXTENSION: &str = "toml";
/// How many gaps of each kind the text report lists per rule; the JSON report lists all.
pub const COVERAGE_GAPS_LISTED: usize = 3;
