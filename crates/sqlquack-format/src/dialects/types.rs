//! Types of the dialect profiles.

use sqlquack_syntax::Expression;

use crate::dialects::models::{Comment, Lexeme, StatementSpan};

/// A dialect's own scanner: the tokens and comments of some SQL, or why it cannot read them.
pub(crate) type NativeLexer = fn(&str) -> Result<(Vec<Lexeme>, Vec<Comment>), String>;
/// Gives shared tokens the types a dialect's own scanner reads them as, or refuses when they differ.
pub(crate) type KeywordTyper = fn(&str, Vec<Lexeme>) -> Result<Vec<Lexeme>, String>;
/// A dialect's own parser: the statements of some SQL and where it read each, or why it cannot.
pub(crate) type NativeParser = fn(&str) -> Result<(Vec<Expression>, Vec<StatementSpan>), String>;
