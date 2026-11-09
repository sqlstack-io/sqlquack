//! Diagnostics, the failure taxonomy, refusals, constructs and conversion provenance for SQLQuack.

#![forbid(unsafe_code)]

pub mod classification;
pub mod diagnostics;
pub mod failures;
pub mod provenance;
pub mod refusals;
#[cfg(feature = "snowflake")]
pub mod snowflake;
