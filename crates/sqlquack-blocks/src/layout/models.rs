//! The state the layout keeps while it walks the tokens.

/// The clause a query is in, which decides where its commas break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Clause {
    /// The select list: one item per line.
    Select,
    /// The common table expressions: one per line.
    With,
    /// Any other clause: commas stay inline.
    Other,
}

/// One pair of parentheses being laid out, or the statement itself.
#[derive(Debug)]
pub(crate) struct Frame {
    /// Whether it holds a query rather than an expression.
    pub(crate) query: bool,
    /// Whether its lines break; a short query stays on one line.
    pub(crate) broken: bool,
    /// Whether nothing inside it may break, because an enclosing query stays on one line.
    pub(crate) inline: bool,
    /// Whether a function call encloses it within its query, so a `CASE` in it stays inline.
    pub(crate) call: bool,
    /// The indentation level of its clauses.
    pub(crate) base: usize,
    /// The indentation level of its closing parenthesis.
    pub(crate) close: usize,
    /// The clause it is in.
    pub(crate) clause: Clause,
    /// Whether the select list's first item is still to come.
    pub(crate) item_pending: bool,
}

/// One `CASE` being laid out.
#[derive(Debug)]
pub(crate) struct Case {
    /// Whether it breaks at `WHEN`, `ELSE` and `END`.
    pub(crate) broken: bool,
    /// The indentation level of the line it starts on.
    pub(crate) indent: usize,
    /// How many frames were open where it starts.
    pub(crate) depth: usize,
}

/// Where the layout is: open frames and `CASE`s, the line's indentation and a pending break.
#[derive(Debug)]
pub(crate) struct Walk {
    /// The open parentheses, outermost first, the statement's own frame at the bottom.
    pub(crate) frames: Vec<Frame>,
    /// The open `CASE`s, outermost first.
    pub(crate) cases: Vec<Case>,
    /// The indentation level of the current line.
    pub(crate) line_indent: usize,
    /// A line break the previous token asked for: the token it comes before, and its level.
    pub(crate) forced: Option<(usize, usize)>,
    /// The SQL laid out so far.
    pub(crate) laid_out: String,
}
