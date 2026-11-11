//! Development check of each dialect's documented syntax coverage, the translation results, the nesting-depth probe, the instruction-count guard and the corpora's parse verdicts; never a runtime dependency.

#![forbid(unsafe_code)]

pub mod check;
pub mod cli;
pub mod depth;
pub mod formats;
pub mod instructions;
pub mod report;
pub mod results;
pub mod translation;
pub mod verdicts;
