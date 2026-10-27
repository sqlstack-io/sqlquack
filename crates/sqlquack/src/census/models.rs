//! Census options and reports.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

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
    /// Name models and relations in the blockers; without it they are numbered or folded.
    pub names: bool,
}

/// A census report: counts and construct keys, and model names in blockers only when asked.
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
    /// What keeps each refused file from converting, traced through the models it reads.
    pub blockers: BlockerReport,
}

/// Refused files traced to the refusals at the root of the models they read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BlockerReport {
    /// Each refused file, by model name, or by number when names are not asked for.
    pub files: Vec<BlockedFile>,
    /// Each root refusal and the files it keeps from converting, most files first.
    pub roots: Vec<RootBlocker>,
    /// Refused files with exactly one root refusal, which one fix would unblock.
    pub single_root_files: usize,
}

/// One refused file and what blocks it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BlockedFile {
    /// The model the file defines: its file name without extension.
    pub model: String,
    /// The file's own refusals, not caused by a refused model it reads.
    pub own: Vec<RootKey>,
    /// Refused models the file reads, each with its root refusals.
    pub upstream: Vec<UpstreamBlock>,
    /// Relations the file reads that are neither a model of the project nor in the schema.
    pub unknown_relations: Vec<String>,
    /// Every root refusal blocking the file, its own and its upstream models'.
    pub roots: Vec<RootKey>,
    /// The file's refusals in source order, each with where in the file it is.
    pub placed: Vec<PlacedRefusal>,
}

/// A refusal of one file and its line and column in the file, when it has a position.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct PlacedRefusal {
    /// One-based line.
    pub line: Option<usize>,
    /// One-based column.
    pub column: Option<usize>,
    /// The refusal category.
    pub category: String,
    /// The construct, or `statement`.
    pub construct: String,
}

/// A refused model a file reads, and the root refusals behind it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpstreamBlock {
    /// The model's name.
    pub model: String,
    /// Its root refusals.
    pub roots: Vec<RootKey>,
}

/// A refusal as a category and the construct it names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RootKey {
    /// The refusal category.
    pub category: String,
    /// The construct, `statement`, or `relation:<name>` for an unknown relation.
    pub construct: String,
}

/// One root refusal and the files fixing it would help.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RootBlocker {
    /// The refusal category.
    pub category: String,
    /// The construct.
    pub construct: String,
    /// Files it refuses directly.
    pub direct: usize,
    /// Files downstream of those, through the models they read.
    pub downstream: usize,
    /// `direct` and `downstream` together.
    pub total: usize,
    /// Files whose only root refusal it is.
    pub sole: usize,
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
    /// Files with a statement whose conversion took too long or emitted too much SQL.
    pub conversion_budget: usize,
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

/// Files inspected so far in the current pass, shared with whoever reports progress.
#[derive(Debug, Default)]
pub struct CensusProgress {
    /// The pass: 1 for the first, one more for each retry of chained models.
    pub pass: AtomicUsize,
    /// Files inspected in this pass.
    pub done: AtomicUsize,
    /// Files this pass inspects.
    pub total: AtomicUsize,
    /// Files inspected over every pass.
    pub inspected: AtomicUsize,
}

/// A census and the time each file took, which the report itself never names.
#[derive(Clone, Debug, PartialEq)]
pub struct CensusRun {
    /// The report.
    pub report: CensusReport,
    /// Each file with the time spent converting it over every pass, slowest first.
    pub timings: Vec<(PathBuf, Duration)>,
}

/// SQL with its sqlbuild reference calls replaced, and what was replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rewritten {
    /// The parseable SQL.
    pub(crate) sql: String,
    /// The placeholder relation each reference call became, in upper case.
    pub(crate) references: BTreeSet<String>,
    /// Each replacement, in order.
    pub(crate) replacements: Vec<Replacement>,
}

/// One reference call replaced: its bytes in the rewritten and in the original SQL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Replacement {
    /// The replacement's bytes in the rewritten SQL.
    pub(crate) rewritten: std::ops::Range<usize>,
    /// The call's bytes in the original SQL.
    pub(crate) original: std::ops::Range<usize>,
}
