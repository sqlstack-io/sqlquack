//! Development check of each dialect's documented syntax coverage, the translation results and the nesting-depth probe; never a runtime dependency.

#![forbid(unsafe_code)]

pub mod check;
pub mod cli;
pub mod depth;
pub mod formats;
pub mod report;
pub mod results;
pub mod translation;
