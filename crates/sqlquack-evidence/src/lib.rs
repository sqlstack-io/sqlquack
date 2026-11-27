//! Evidence: cases, content hashes, pending files, recorded results, DuckDB replay and audits.

#![forbid(unsafe_code)]

pub mod audit;
pub mod cases;
pub mod constants;
pub mod coverage;
#[cfg(feature = "databricks")]
pub mod databricks;
pub mod dialects;
pub mod errors;
pub mod pending;
pub mod replay;
pub mod results;
pub mod verify;
