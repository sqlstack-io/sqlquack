//! What the screen parses and splits text with.

use serde_json::Value;

/// A text's written tree, or none when the parser refuses it.
pub type TreeOf = dyn Fn(&str) -> Option<Value>;
/// A text's token byte ranges as the lexer reads them.
pub type TokensOf = dyn Fn(&str) -> Vec<(usize, usize)>;
