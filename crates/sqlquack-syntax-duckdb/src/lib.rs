//! SQLQuack's DuckDB 2.0.0 scanner and parser into the shared sqlquack-syntax tree.

#![forbid(unsafe_code)]

pub mod keywords;
pub mod parse;
pub mod scan;

pub use crate::keywords::main::classify::classify;
pub use crate::keywords::models::KeywordClasses;
pub use crate::parse::errors::{ParseError, StrictFailure};
pub use crate::parse::main::parse::parse;
pub use crate::parse::main::parse_strict::parse_strict;
pub use crate::scan::main::scan::scan;
pub use crate::scan::main::uncovered::uncovered;
pub use sqlquack_syntax::{Gap, Parsed};
