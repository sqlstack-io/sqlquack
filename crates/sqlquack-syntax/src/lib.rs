//! SQLQuack's shared syntax tree, tokens, spans and diagnostics for every dialect parser. It holds no tokenizer
//! or parser: those are in `sqlquack-tokenize` and the parser crates, so a crate that depends on this one alone
//! can read trees but never re-read SQL.

#![forbid(unsafe_code)]

pub mod expressions;
pub mod functions;
pub mod guard;
pub mod tokens;
pub mod traversal;

pub use crate::expressions::tree::models::Expression;
pub use crate::expressions::tree::models::Gap;
pub use crate::expressions::tree::models::Parsed;
pub use crate::expressions::types::models::DataType;
pub use crate::functions::typed::main::canonical_call_name::canonical_call_name;
pub use crate::guard::models::ComplexityGuardOptions;
pub use crate::guard::models::ParseOptions;
pub use crate::tokens::errors::Error;
pub use crate::tokens::models::CallMode;
pub use crate::tokens::models::Dialect;
pub use crate::tokens::types::Result;
pub use crate::traversal::types::ExpressionWalk;
