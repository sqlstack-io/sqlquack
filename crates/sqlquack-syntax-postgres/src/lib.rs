//! SQLQuack's PostgreSQL 18 scanner and parser into the shared sqlquack-syntax tree.

#![forbid(unsafe_code)]

pub mod keywords;
pub mod parse;
pub mod scan;

pub use crate::parse::errors::ParseError;
pub use crate::parse::main::parse::parse;
pub use crate::parse::models::{Gap, Parsed};
