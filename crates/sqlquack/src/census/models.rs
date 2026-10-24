//! Census options and reports.

use std::collections::BTreeMap;

use serde::Serialize;
use sqlquack_core::parsing::models::SourceDialect;

/// What a census reads and reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CensusOptions {
    /// The dialect the SQL is written in.
    pub dialect: SourceDialect,
    /// Rule counts at which the convertible share is reported.
    pub coverage_points: Vec<usize>,
}

/// A census report: counts and construct keys only, never SQL text or file names.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CensusReport {
    /// The source dialect.
    pub dialect: SourceDialect,
    /// File and construct totals.
    pub totals: CensusTotals,
    /// Files by the source print-back check's worst statement.
    pub fidelity: FidelityTotals,
    /// Files with at least one refusal of each category.
    pub refusals: RefusalTotals,
    /// Every construct, by the number of files using it, most used first.
    pub constructs: Vec<ConstructCount>,
    /// Files that would convert as the top constructs get rules.
    pub coverage: Vec<CoveragePoint>,
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

/// Files by their worst source print-back outcome.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FidelityTotals {
    /// Every statement printed back to the same tree.
    pub identical: usize,
    /// A print-back canonicalised the tree once, then settled.
    pub canonicalised: usize,
    /// A print-back kept changing the tree.
    pub unstable: usize,
    /// A tree did not print, or its print did not parse.
    pub failed: usize,
}

/// Files with at least one refusal of each category.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct RefusalTotals {
    /// Files with a construct no rule converts.
    pub missing_rule: usize,
    /// Files with a construct DuckDB cannot express.
    pub not_expressible: usize,
    /// Files that did not parse or did not print back faithfully.
    pub parse_fidelity: usize,
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
