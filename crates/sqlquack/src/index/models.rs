//! Conversion records and the index built from them.

use serde::Serialize;
use sqlquack_core::provenance::models::Provenance;

/// Where a conversion record comes from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(tag = "kind", content = "name", rename_all = "snake_case")]
pub enum RecordSource {
    /// An evidence case, by `<file>#<id>` reference.
    Case(String),
    /// A corpus SQL file, by path.
    File(String),
}

/// One conversion: what it relied on, or what it refused.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConversionRecord {
    /// The case or file converted.
    pub source: RecordSource,
    /// Everything a successful conversion relied on.
    pub provenance: Option<Provenance>,
    /// Refused construct keys, sorted; empty when the conversion succeeded.
    pub refused_constructs: Vec<String>,
}

/// The cases and corpus files that rely on one identifier.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Dependents {
    /// Case references, sorted.
    pub cases: Vec<String>,
    /// Corpus file paths, sorted.
    pub files: Vec<String>,
}

/// A building block or typing function: the rules that call it, and their dependents.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UsedEntry {
    /// The identifier, such as `duckdb.cast`.
    pub id: String,
    /// The rules that call it, sorted.
    pub rules: Vec<String>,
    /// The cases and files whose conversions called it.
    #[serde(flatten)]
    pub dependents: Dependents,
}

/// A rule: what it calls, and the cases and files whose conversions applied it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RuleEntry {
    /// The rule identifier.
    pub id: String,
    /// Building blocks and typing functions the rule calls, sorted.
    pub uses: Vec<String>,
    /// The cases and files whose conversions applied it.
    #[serde(flatten)]
    pub dependents: Dependents,
}

/// An equivalence or refused construct, and the cases and files it concerns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KeyedEntry {
    /// The equivalence identifier, or the refused construct key (`statement` for none).
    pub id: String,
    /// The cases and files it concerns.
    #[serde(flatten)]
    pub dependents: Dependents,
}

/// Record totals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct IndexTotals {
    /// Cases indexed.
    pub cases: usize,
    /// Corpus files indexed.
    pub files: usize,
    /// Records that converted.
    pub converted: usize,
    /// Records that were refused.
    pub refused: usize,
}

/// Every identifier mapped to its dependents, each list sorted, so two indexes diff line by line.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ProvenanceIndex {
    /// Record totals.
    pub totals: IndexTotals,
    /// Building blocks, by identifier.
    pub building_blocks: Vec<UsedEntry>,
    /// Typing functions, by identifier.
    pub typing: Vec<UsedEntry>,
    /// Rules, by identifier.
    pub rules: Vec<RuleEntry>,
    /// Proven equivalences, by identifier, with the records that relied on them.
    pub equivalences: Vec<KeyedEntry>,
    /// Refused constructs, by key.
    pub refusals: Vec<KeyedEntry>,
}
