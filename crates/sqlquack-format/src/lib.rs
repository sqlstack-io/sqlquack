//! Readable layout for SQL that changes only the whitespace between its tokens.

#![forbid(unsafe_code)]

pub mod layout;

pub use crate::layout::main::pretty_sql::pretty_sql;
