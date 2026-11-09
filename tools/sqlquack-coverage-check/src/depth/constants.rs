//! The deep-nesting probe's ladder, bounds and allowlist path.

use std::time::Duration;

use crate::formats::models::LocalParser;

/// The known failures of today's parsers, relative to the repository.
pub const ALLOWLIST_FILE: &str = "coverage/deep-nesting.toml";
/// The nesting depths each construct is parsed at, shallowest first; a construct stops at its first failure.
pub const DEPTH_LADDER: [usize; 6] = [50, 200, 1_000, 3_000, 10_000, 100_000];
/// The time one probe parse may take before it fails as a timeout.
pub const PROBE_BOUND: Duration = Duration::from_secs(15);
/// The time one parse and its analysis may take: linear analysis ends far inside it, quadratic not.
pub const ANALYSIS_BOUND: Duration = Duration::from_millis(500);
/// The depths each analysed construct is analysed at, up to about the deepest any parser reads.
pub const ANALYSIS_LADDER: [usize; 4] = [50, 200, 450, 900];
/// The stack of the thread each probe parse runs on, the smallest an in-process caller is assumed to give.
pub const PROBE_STACK_BYTES: usize = 2 * 1024 * 1024;
/// How many runs of `[]` a mixed type nests between its constructor levels.
pub const MIXED_SUFFIX_RUN: usize = 60;
/// How many links each parenthesised level of a nested chain holds.
pub const NESTED_CHAIN_LINKS: usize = 60;
/// Every native parser this checker links, each probed in turn.
pub const PROBED_PARSERS: [LocalParser; 7] = [
    LocalParser::Snowflake,
    LocalParser::Postgres,
    LocalParser::BigQuery,
    LocalParser::DuckDb,
    LocalParser::Databricks,
    LocalParser::SqlServer,
    LocalParser::Sqlite,
];
