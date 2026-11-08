//! A keyword as the tokenizer hands it to the grammar.

/// A keyword: the grammar token it is, and whether `%fallback ID` lets it stand for a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keyword {
    /// The grammar token without its `TK_` prefix, such as `SELECT`, `JOIN_KW` or `CTIME_KW`.
    pub token: &'static str,
    /// Whether the parser reads it as `ID` where the keyword itself has no action.
    pub fallback: bool,
}
