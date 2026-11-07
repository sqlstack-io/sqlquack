//! SQLQuack's PostgreSQL 18 scanner and parser into the shared sqlquack-syntax tree.

#![forbid(unsafe_code)]

pub mod keywords;
pub mod parse;
pub mod scan;

pub use crate::parse::errors::{ParseError, StrictFailure};
pub use crate::parse::main::format_type::format_type;
pub use crate::parse::main::parse::parse;
pub use crate::parse::main::parse_strict::parse_strict;
pub use crate::parse::main::parse_type::parse_type;
pub use crate::parse::models::{Gap, Parsed};
