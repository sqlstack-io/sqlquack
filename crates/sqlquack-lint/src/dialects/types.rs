//! Types of the dialect profiles.

use sqlquack_syntax::Expression;
use sqlquack_syntax::ParseOptions;
use sqlquack_syntax::tokens::models::Token;

/// The tokens of some SQL as a dialect reads them, or why it cannot.
pub(crate) type ReadTokens = fn(&str) -> Result<Vec<Token>, String>;
/// The statements of some SQL as a dialect parses them, under the shared parser's guards, or why it cannot.
pub(crate) type ReadStatements = fn(&str, &ParseOptions) -> Result<Vec<Expression>, String>;
/// The expressions starting a predicate, from its text and its tokens, or why they do not parse.
pub(crate) type ReadPredicates = fn(&str, &[Token]) -> Result<Vec<Expression>, String>;
#[cfg(feature = "native-parsers")]
/// A dialect's own strict parse of some SQL; on refusal, its byte position and parse message, if any.
pub(crate) type StrictParse = fn(&str) -> Result<Vec<Expression>, (usize, Option<String>)>;
