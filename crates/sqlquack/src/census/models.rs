//! Census options and reports.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::models::Provenance;
use sqlquack_rules::conversion::models::{ConversionOptions, Schema};

/// What a census reads and reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CensusOptions {
    /// The dialect the SQL is written in.
    pub dialect: SourceDialect,
    /// Rule counts at which the convertible share is reported.
    pub coverage_points: Vec<usize>,
    /// Typed columns of the relations the files read.
    pub schema: Schema,
    /// Type each converted file as the relation its name references, and convert again.
    pub chain_models: bool,
    /// The known divergences conversion may rely on.
    pub conversion: ConversionOptions,
}

/// A census report: counts and construct keys only, never SQL text or file names.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CensusReport {
    /// The source dialect.
    pub dialect: SourceDialect,
    /// File and construct totals.
    pub totals: CensusTotals,
    /// Files with at least one refusal of each category.
    pub refusals: RefusalTotals,
    /// Every construct, by the number of files using it, most used first.
    pub constructs: Vec<ConstructCount>,
    /// Files that would convert as the top constructs get rules.
    pub coverage: Vec<CoveragePoint>,
    /// Converted files by the rules, building blocks and typing functions they relied on.
    pub provenance: ProvenanceCounts,
}

/// Converted files per rule, building block and typing function, each sorted by identifier.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ProvenanceCounts {
    /// Files per rule.
    pub rules: BTreeMap<String, usize>,
    /// Files per DuckDB building block.
    pub building_blocks: BTreeMap<String, usize>,
    /// Files per Snowflake typing function.
    pub typing: BTreeMap<String, usize>,
    /// Files per known divergence, by `<divergence> at <construct>`.
    pub divergences: BTreeMap<String, usize>,
}

/// File and construct totals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CensusTotals {
    /// SQL files read.
    pub files: usize,
    /// Files that parsed.
    pub parsed: usize,
    /// Files that did not parse.
    pub unparsed: usize,
    /// Files with no statement.
    pub empty: usize,
    /// Statements parsed.
    pub statements: usize,
    /// Distinct constructs used.
    pub distinct_constructs: usize,
    /// Distinct constructs per family.
    pub constructs_by_family: BTreeMap<String, usize>,
}

/// Files with at least one refusal of each category.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RefusalTotals {
    /// Files with a construct no rule converts.
    pub missing_rule: usize,
    /// Files with a construct DuckDB cannot express.
    pub not_expressible: usize,
    /// Files that did not parse.
    pub parse_fidelity: usize,
    /// Files where a rule refused the operand types.
    pub type_condition: usize,
    /// Files with a relation or column name that does not resolve.
    pub unresolved_name: usize,
    /// Refusals by category and construct, counting each file once per pair, most frequent first.
    pub by_construct: Vec<RefusalCount>,
    /// Files with no refusal at all.
    pub convertible: usize,
}

/// How many files use one construct.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConstructCount {
    /// The construct key.
    pub construct: String,
    /// Files using it.
    pub files: usize,
}

/// Files that convert once the first `rules` constructs have rules, in two orders.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CoveragePoint {
    /// Constructs with rules.
    pub rules: usize,
    /// Convertible files when rules follow file frequency.
    pub frequency_order: usize,
    /// Convertible files when each next rule is the one that completes the most files.
    pub greedy_order: usize,
    /// `frequency_order` as a share of all files.
    pub frequency_share: f64,
    /// `greedy_order` as a share of all files.
    pub greedy_share: f64,
}

/// Files refused for one category and construct.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RefusalCount {
    /// The refusal category.
    pub category: String,
    /// The construct key, or `statement` when the refusal names none.
    pub construct: String,
    /// Files with such a refusal.
    pub files: usize,
}

/// One corpus file: what its conversion relied on, or the constructs refused in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CorpusFile {
    /// The file, as found under the given paths.
    pub path: PathBuf,
    /// For a file that converts: everything its conversion relied on.
    pub provenance: Option<Provenance>,
    /// The refused construct keys, sorted; empty when the file converts.
    pub refused_constructs: Vec<String>,
}
