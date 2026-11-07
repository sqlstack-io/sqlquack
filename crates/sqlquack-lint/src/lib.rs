//! SQL lint Rules over SQLQuack's tokens and syntax trees, ported from SQLClean's lint engine.

#![forbid(unsafe_code)]

pub mod constants;
pub mod lint;
pub mod quality;
pub(crate) mod tree;

pub use crate::lint::main::batch_engine::lint_batch_json;
pub use crate::lint::main::catalogue::catalogue;
pub use crate::lint::main::cross_join_subject::cross_join_subject;
pub use crate::lint::main::default_rules::default_rules;
pub use crate::lint::main::engine::lint_json;
pub use crate::lint::main::fill_finding_path::fill_finding_path;
pub use crate::lint::main::lint::lint;
pub use crate::lint::main::parse_sql::parse_sql;
pub use crate::lint::main::validate_dialect::validate_dialect;
