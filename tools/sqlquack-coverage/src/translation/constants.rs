//! Paths, bounds and the refusal categories the measure reads.

use std::time::Duration;

/// The translation files, beside `coverage/syntax`.
pub const TRANSLATION_DIR: &str = "translation";
/// Constructs refused by design, each with its reason and DuckDB note.
pub const BY_DESIGN_FILE: &str = "by-design.toml";
/// The auditor's sign-off of each by-design entry.
pub const BY_DESIGN_REVIEW_FILE: &str = "reviews/auditor-by-design.toml";
/// One census sidecar per family, under the coverage root.
pub const CENSUS_DIR: &str = "census";
/// The project folder of source files `sqlquack convert --project` reads.
pub const SOURCES_FOLDER: &str = "sources";
/// The source file holding one case's census relations.
pub const CENSUS_SOURCE_FILE: &str = "census.yml";
/// The time one conversion may take before the case counts as not finished.
pub const CONVERSION_BOUND: Duration = Duration::from_secs(120);
/// The conversion's virtual memory bound in KiB, 4 GiB.
pub const CONVERSION_MEMORY_KIB: u64 = 4 * 1024 * 1024;
/// The refusal category of a name no relation in scope declares.
pub const UNRESOLVED_NAME: &str = "unresolved name";
/// The refusal category of a divergence the policy refuses, here only value divergences.
pub const DIVERGENCE_REFUSED: &str = "a known divergence the policy refuses";
/// What results-match reports until Snowflake rows are recorded for syntax cases.
pub const NO_RECORDED_RESULT: &str = "not measured: no recorded result";
