//! Development check of documented Snowflake syntax coverage; never a runtime dependency.

#![forbid(unsafe_code)]

pub mod check;
pub mod cli;
pub mod formats;
pub mod report;
