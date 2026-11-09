//! SQLQuack's Snowflake parser and polyglot's parse modes, into the shared syntax tree.

#![forbid(unsafe_code)]

pub mod modes;
pub mod parser;

pub use crate::modes::errors::StrictFailure;
pub use crate::modes::main::parse_strict::parse_strict;
pub use crate::modes::main::parse_tolerant::parse_tolerant;
pub use crate::modes::models::TolerantParse;
pub use crate::modes::models::TolerantStatement;
pub use crate::parser::main::dialect_tokens::dialect_tokens;
pub use crate::parser::main::parse_expressions::parse_expressions;
pub use crate::parser::main::parse_statements::parse_statements;
pub use crate::parser::main::parse_statements_with::parse_statements_with;
pub use crate::parser::main::tokenize::tokenize;
