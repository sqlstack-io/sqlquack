//! The instruction-count guard's baseline path, workload size and measured functions.

/// The committed counts and tolerance, relative to the repository.
pub const BASELINE_FILE: &str = "coverage/instruction-counts.toml";
/// The function whose instructions the parse workload counts, as callgrind names it.
pub const PARSE_FUNCTION: &str =
    "sqlquack_syntax_snowflake::modes::main::parse_strict::parse_strict";
/// The function whose instructions the analysis workload counts, as callgrind names it.
pub const ANALYSIS_FUNCTION: &str = "sqlquack_semantics::lineage::main::statement_lineage_and_findings::statement_lineage_and_findings";
/// How many statements each workload reads.
pub const STATEMENTS: usize = 48;
/// Measure columns of the narrowest statement; each next width adds `WIDTH_STEP`, cycling.
pub const MIN_WIDTH: usize = 4;
pub const WIDTH_STEP: usize = 6;
/// How many widths the statements cycle through.
pub const WIDTHS: usize = 6;
/// Every `AGGREGATE_EVERY`th statement ends in a grouped aggregate instead of selecting every column.
pub const AGGREGATE_EVERY: usize = 3;
/// The comment `instruction-counts --update` writes above the counts.
pub const BASELINE_HEADER: &str = "# Instructions callgrind counts in two fixed Snowflake workloads, in the gate profile.
#
# `make instruction-counts` (in check-ci) runs each workload under callgrind, counting only while its
# function runs: `parse` strict-parses 48 wide, CTE-layered models; `analysis` runs lineage and
# findings in one analysis over the same statements. Only the program's own code counts; shared
# libraries, the C allocator and memcpy among them, are left out, because their counts change with
# the C library. A count more than `tolerance_percent` over its baseline fails as a slowdown; one
# more than that under fails too, so a gain is recorded and cannot be spent unnoticed. The counts
# hold for one architecture and one compiler: a run on another `arch`, or a build whose
# `rustc --version` differs from `rustc`, fails without comparing. Record new counts with
# `make instruction-baseline` in the change that moves them, a toolchain bump included.

";
