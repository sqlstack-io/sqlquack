//! The conversion pipeline: parse, classify, convert or refuse. Every conversion targets DuckDB.

#![forbid(unsafe_code)]

pub mod analysis;
#[cfg(feature = "snowflake")]
pub mod snowflake;
