//! SQLQuack's ClickHouse SQL lexer and parser into the shared sqlquack-syntax tree.

#![forbid(unsafe_code)]

pub mod parse;
pub mod scan;

pub use crate::parse::errors::ParseError;
pub use crate::parse::main::parse::parse;
pub use crate::parse::main::parse_strict::parse_strict;
pub use crate::scan::main::lex::lex;
pub use crate::scan::main::scan::scan;
pub use crate::scan::models::{Token, TokenKind};
pub use sqlquack_syntax::{Gap, Parsed};
