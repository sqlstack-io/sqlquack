//! SQLQuack's SQL Server 2022 Transact-SQL scanner, lossless tokenizer and parser.

#![forbid(unsafe_code)]

pub mod keywords;
pub mod parse;
pub mod scan;

pub use crate::keywords::main::is_reserved::is_reserved;
pub use crate::keywords::main::keyword_status::keyword_status;
pub use crate::parse::errors::{ParseError, StrictFailure};
pub use crate::parse::main::parse::parse;
pub use crate::parse::main::parse_strict::parse_strict;
pub use crate::parse::models::{Gap, Parsed};
pub use crate::scan::errors::ScanError;
pub use crate::scan::main::scan::scan;
pub use crate::scan::main::tokenize::tokenize;
pub use crate::scan::models::KeywordStatus;
