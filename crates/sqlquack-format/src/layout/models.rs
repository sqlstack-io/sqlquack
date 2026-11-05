//! The SQL being laid out, and the text laid out so far.

use sqlquack_syntax::tokens::models::TokenType;

/// One token of the SQL, with whether whitespace preceded it.
#[derive(Debug, Clone)]
pub(crate) struct Word<'sql> {
    /// The token's exact text.
    pub(crate) text: &'sql str,
    /// The significant token type; None for a comment.
    pub(crate) kind: Option<TokenType>,
    /// Whether a line comment, which must end its line.
    pub(crate) line_comment: bool,
    /// Whether whitespace separated it from the token before.
    pub(crate) spaced: bool,
}

/// The tokens of the SQL, and where each bracket or CASE closes.
#[derive(Debug)]
pub(crate) struct Words<'sql> {
    pub(crate) words: Vec<Word<'sql>>,
    /// For a `(` its `)`, for a CASE its END.
    pub(crate) closer: Vec<Option<usize>>,
}

/// The words being laid out, and the text laid out so far.
#[derive(Debug)]
pub(crate) struct Printer<'w, 'sql> {
    pub(crate) words: &'w Words<'sql>,
    pub(crate) output: Output,
}

/// The formatted text so far.
#[derive(Debug, Default)]
pub(crate) struct Output {
    pub(crate) text: String,
    /// Characters on the current line.
    pub(crate) column: usize,
    /// The indent level of the current line.
    pub(crate) level: usize,
    /// Nothing but indentation on the current line yet.
    pub(crate) line_start: bool,
    /// The last word was a line comment, so the next word starts a line.
    pub(crate) after_line_comment: bool,
}

/// How a query clause lays out what follows its keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClauseKind {
    /// Words before the first clause keyword, such as `CREATE TABLE t AS`.
    Prefix,
    /// `WITH`: common table expressions, separated by a blank line.
    With,
    /// `SELECT`: one item per line when there are two or more.
    Select,
    /// `GROUP BY`, `ORDER BY`, `WINDOW`: items on the lines below.
    ListBelow,
    /// `WHERE`, `HAVING`, `QUALIFY`: one condition on the line, several on the lines below.
    Condition,
    /// A join: the relation on the line, `ON` on the next.
    Join,
    /// `UNION`, `INTERSECT`, `EXCEPT`, on a line of their own.
    SetOperation,
    /// `FROM`, `LIMIT` and the rest: on the keyword's line.
    Inline,
}

/// One clause of a query: its keyword words, then its body up to the next clause.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Clause {
    pub(crate) kind: ClauseKind,
    pub(crate) start: usize,
    pub(crate) body: usize,
    pub(crate) end: usize,
}
