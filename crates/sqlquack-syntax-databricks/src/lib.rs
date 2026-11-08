//! SQLQuack's Databricks SQL scanner and parser into the shared sqlquack-syntax tree.

#![forbid(unsafe_code)]

pub mod keywords;
pub mod parse;
pub mod scan;

pub use crate::parse::errors::{ParseError, StrictFailure};
pub use crate::parse::main::parse::parse;
pub use crate::parse::main::parse_strict::parse_strict;
pub use sqlquack_syntax::{Gap, Parsed};
