//! SQL layout that changes only token spacing and keyword case, ported from SQLClean's formatter.

#![forbid(unsafe_code)]

pub mod format;

pub use crate::format::main::format_sql::format_sql;
pub use crate::format::models::{FormatOptions, FormatRefusal, Formatted};
