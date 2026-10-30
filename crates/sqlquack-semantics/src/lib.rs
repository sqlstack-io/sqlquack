//! SQL semantics for SQLQuack: scopes, name resolution, types and lineage, per dialect.

#![forbid(unsafe_code)]

pub mod dialects;
pub mod lineage;
pub mod names;
pub mod scopes;
pub mod types;
