//! Census options and reports.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use serde::Serialize;

use sqlquack_core::parsing::models::SourceDialect;
use sqlquack_core::provenance::models::Provenance;
use sqlquack_rules::conversion::models::ConversionOptions;
use sqlquack_semantics::types::models::Schema;

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
    /// Constructs whose refusals the what-if counts treat as resolved.
    pub assume_supported: Vec<String>,
    /// The project's cursor type, whose bound replaces cursor intrinsics; unknown without it.
    pub cursor_type: Option<CursorType>,
}

/// A project's incremental cursor type, which decides how SQLBuild renders cursor bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum CursorType {
    /// Timestamp bounds, rendered as text.
    Timestamp,
    /// Integer bounds, rendered as integers.
    Integer,
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
    /// Upper bounds on the files converting once constructs are supported, and a greedy plan.
    pub plan: PlanReport,
}

/// Upper bounds on what converts once more constructs are supported, from collected refusals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PlanReport {
    /// The constructs assumed supported, as given or expanded from rules.
    pub assumed: Vec<String>,
    /// Upper bound on unit-testable files with the assumed constructs supported.
    pub unit_testable_upper_bound: usize,
    /// Upper bound on strict files with the assumed constructs supported.
    pub strict_upper_bound: usize,
    /// Of the unit-testable bound, files that may hold refusals collecting did not reach.
    pub unit_testable_possibly_incomplete: usize,
    /// Assumed constructs that match no collected root, which the bounds therefore ignore.
    pub unmatched: Vec<String>,
    /// The roots still blocking files, by the files they block, most first.
    pub next: Vec<NextRoot>,
    /// Greedy steps from the assumed constructs, each adding the roots gaining the most files.
    pub steps: Vec<PlanStep>,
}

/// A root still blocking files after the assumed constructs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NextRoot {
    /// The root.
    pub root: RootKey,
    /// Files it blocks among those still not unit-testable.
    pub files: usize,
    /// Of those, files it alone blocks.
    pub sole: usize,
}

/// One greedy step: roots supported together, and the cumulative upper bounds after it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PlanStep {
    /// The roots this step supports, together.
    pub roots: Vec<RootKey>,
    /// Upper bound on unit-testable files after this and every earlier step.
    pub unit_testable_upper_bound: usize,
    /// Upper bound on strict files after this and every earlier step.
    pub strict_upper_bound: usize,
    /// Of the unit-testable bound, files that may hold refusals collecting did not reach.
    pub unit_testable_possibly_incomplete: usize,
}

/// Refused files traced to the refusals at the root of the models they read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct BlockerReport {
    /// Each refused file, by model name, or by number when names are not asked for.
    pub files: Vec<BlockedFile>,
    /// Each root refusal and the files it keeps from converting, most sole files first.
    pub roots: Vec<RootBlocker>,
    /// Small sets of roots and the files fixing all of them would unblock, most files first.
    pub combinations: Vec<RootCombination>,
    /// Refused files whose complete roots are exactly one refusal, which one fix would unblock.
    pub single_root_files: usize,
    /// Refused files that may hold refusals collecting did not reach, themselves or upstream.
    pub possibly_incomplete_files: usize,
    /// Converted files reading a model that does not convert end to end, typed by the schema file.
    pub upstream_refused: Vec<UpstreamRefusedFile>,
}

/// A converted file reading models that do not convert end to end.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UpstreamRefusedFile {
    /// The model the file defines, or its number when names are not asked for.
    pub model: String,
    /// `upstream_refused:<model>` per refused model upstream, `upstream_missing:<relation>` per absent one.
    pub labels: Vec<String>,
}

/// A set of roots and the refused files whose roots it covers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RootCombination {
    /// The roots.
    pub roots: Vec<RootKey>,
    /// Files with complete roots, all in the set, reading no refused model.
    pub direct: usize,
    /// Files with complete roots, all in the set, reading a refused model.
    pub downstream: usize,
    /// `direct` and `downstream` together.
    pub files: usize,
    /// Possibly incomplete files whose collected roots are all in the set.
    pub uncertain: usize,
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
    /// Whether the file or a model upstream may hold refusals that collecting did not reach.
    pub possibly_incomplete: bool,
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
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
    /// Files with complete roots whose only root it is: fixing it alone makes them convert.
    pub sole: usize,
    /// Possibly incomplete files whose only collected root it is.
    pub sole_uncertain: usize,
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
    /// Unit-testable files: no refusal, reading models as the schema file or their conversion types.
    pub convertible: usize,
    /// Convertible files whose every model upstream converts too, so they run end to end.
    pub convertible_strict: usize,
    /// Unit-testable files relying on no named assumption.
    pub convertible_no_assumptions: usize,
    /// Strict files relying, themselves and every model upstream, on no named assumption.
    pub convertible_strict_no_assumptions: usize,
    /// Unit-testable files relying on each named assumption themselves.
    pub assumptions: BTreeMap<String, usize>,
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

/// Each file's model reads by file index: all of them, the untyped refused ones, and absent ones.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModelReads {
    /// Every model the file reads.
    pub(crate) reads: Vec<BTreeSet<usize>>,
    /// Models it reads that neither convert nor have schema columns.
    pub(crate) needs: Vec<BTreeSet<usize>>,
    /// Relations its model references name that the scan holds no file for.
    pub(crate) missing: Vec<BTreeSet<String>>,
}

/// SQL with its sqlbuild reference calls replaced, and what was replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rewritten {
    /// The parseable SQL.
    pub(crate) sql: String,
    /// The placeholder relation each reference call became, in upper case.
    pub(crate) references: BTreeSet<String>,
    /// Of those, the ones a model reference call became.
    pub(crate) model_references: BTreeSet<String>,
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
