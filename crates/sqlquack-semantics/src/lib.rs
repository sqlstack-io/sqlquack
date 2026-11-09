//! SQL semantics for SQLQuack: scopes, name resolution, types and lineage, per dialect.

#![forbid(unsafe_code)]

pub mod dialects;
pub mod lineage;
#[cfg(feature = "snowflake")]
pub mod snowflake;

pub use crate::lineage::main::query_lineage::query_lineage;
pub use crate::lineage::main::query_lineage_and_findings::query_lineage_and_findings;
pub use crate::lineage::main::query_name_findings::query_name_findings;
pub use crate::lineage::main::statement_lineage::statement_lineage;
pub use crate::lineage::main::statement_name_findings::statement_name_findings;
