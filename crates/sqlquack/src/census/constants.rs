//! Census defaults and the placeholders that stand in for sqlbuild references.

/// Rule counts at which the convertible share is reported.
pub const COVERAGE_POINTS: &[usize] = &[10, 25, 50, 75, 100, 150, 200, 250];

/// Constructs listed in the text report unless asked otherwise.
pub const DEFAULT_TEXT_LIMIT: usize = 50;

/// Stack for each census worker. Polyglot's trees and their JSON form are deep for generated SQL.
pub const WORKER_STACK_BYTES: usize = 256 * 1024 * 1024;

/// The table that stands in for sqlbuild model, source, seed and fixture references.
pub const RELATION_PLACEHOLDER: &str = "__sqlquack_relation";

/// The table function that stands in for sqlbuild table-function references.
pub const TABLE_FUNCTION_PLACEHOLDER: &str = "__sqlquack_table_fn";

/// The column that stands in for sqlbuild cursor bounds.
pub const CURSOR_PLACEHOLDER: &str = "__sqlquack_cursor";

/// sqlbuild reference calls replaced by a relation.
pub const RELATION_CALLS: &[&str] = &[
    "__ref",
    "__source",
    "__seed",
    "__dbt_ref",
    "__empty_fixture",
];

/// sqlbuild cursor intrinsics replaced by a column.
pub const CURSOR_CALLS: &[&str] = &["__cursor_start", "__cursor_end"];

/// When set, census writes each converted file's known divergences there as `<path>.json`.
pub const GUARDS_DIR_VARIABLE: &str = "SQLQUACK_GUARDS_DIR";

/// How often census reports progress on stderr.
pub const PROGRESS_INTERVAL_SECONDS: u64 = 5;

/// Files the census lists, slowest first, when it finishes.
pub const SLOWEST_FILES: usize = 10;

/// The fewest roots a combination holds.
pub const PAIR: usize = 2;

/// The most roots a combination holds.
pub const LARGEST_COMBINATION: usize = 3;

/// Root combinations reported for each size, pairs and triples.
pub const COMBINATIONS_PER_SIZE: usize = 15;

/// The roots with most sole files, paired with each other as combination candidates.
pub const COMBINATION_SEED_PAIRS: usize = 20;

/// The roots with most sole files, added to each two-root file's pair as triple candidates.
pub const COMBINATION_SEED_TRIPLES: usize = 12;

/// How a blocker names an unresolved relation that is neither a model nor in the schema.
pub const RELATION_ROOT_PREFIX: &str = "relation:";

/// How a blocker names a model that converts but was read before chaining typed it.
pub const MODEL_ROOT_PREFIX: &str = "model:";

/// The root of models refused only because they read each other.
pub const CYCLE_ROOT: &str = "cycle";

/// A `model:` root without its name, when names are not asked for.
pub const UPSTREAM_MODEL_ROOT: &str = "upstream model";

/// The label of a converted file reading a model that does not convert end to end.
pub const UPSTREAM_REFUSED_LABEL: &str = "upstream_refused:";

/// A `relation:` root without its name, when names are not asked for.
pub const UNKNOWN_RELATION_ROOT: &str = "unknown relation";

/// How many of a file's refusals the text report places before counting the rest.
pub const PLACED_PER_FILE: usize = 5;

/// Greedy plan steps reported.
pub const PLAN_STEPS: usize = 10;

/// The most roots one plan step supports together.
pub const PLAN_SET_SIZE: usize = 3;

/// Roots still blocking files, after the assumed constructs, listed in the plan.
pub const NEXT_ROOTS: usize = 20;

/// Roots naming a relation or model, not a construct, which no plan step supports.
pub const PLANNABLE_ROOT_EXCLUDED: &[&str] = &[RELATION_ROOT_PREFIX, MODEL_ROOT_PREFIX];

/// How `--assume-supported` names a rule rather than a construct.
pub const RULE_ID_PREFIX: &str = "rule.";
