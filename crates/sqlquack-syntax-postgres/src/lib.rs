//! SQLQuack's PostgreSQL 18 scanner and parser into the shared sqlquack-syntax tree.

#![forbid(unsafe_code)]

pub mod keywords;
pub mod parse;
pub mod scan;
