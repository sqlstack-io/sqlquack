//! SQLQuack's tokenizer, parser and syntax tree for Snowflake and DuckDB.

#![forbid(unsafe_code)]

pub mod expressions;
pub(crate) mod functions;
pub mod guard;
pub mod lossless;
pub mod modes;
pub mod parser;
pub mod tokens;
pub mod traversal;

pub use crate::expressions::tree::models::Expression;
pub use crate::expressions::types::models::DataType;
pub use crate::guard::models::ComplexityGuardOptions;
pub use crate::guard::models::ParseOptions;
pub use crate::lossless::errors::TokenizeFailure;
pub use crate::lossless::main::tokenize::tokenize;
pub use crate::lossless::models::LosslessToken;
pub use crate::lossless::models::TokenKind;
pub use crate::modes::errors::StrictFailure;
pub use crate::modes::main::parse_strict::parse_strict;
pub use crate::modes::main::parse_tolerant::parse_tolerant;
pub use crate::modes::models::TolerantParse;
pub use crate::modes::models::TolerantStatement;
pub use crate::parser::main::parse_statements::parse_statements;
pub use crate::parser::main::parse_statements_with::parse_statements_with;
pub use crate::tokens::errors::Error;
pub use crate::tokens::models::CallMode;
pub use crate::tokens::models::Dialect;
pub use crate::tokens::types::Result;
pub use crate::traversal::types::ExpressionWalk;
