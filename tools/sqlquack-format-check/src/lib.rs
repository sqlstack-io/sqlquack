//! Development check that formatting leaves DuckDB's output columns unchanged; never a runtime dependency.

#![forbid(unsafe_code)]

pub mod columns;

pub use crate::columns::main::compare_columns::compare_columns;
pub use crate::columns::models::ColumnsOutcome;
