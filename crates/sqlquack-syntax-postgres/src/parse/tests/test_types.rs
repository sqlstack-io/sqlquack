//! Test case types for the PostgreSQL parser.

/// What PostgreSQL 18.6's raw parser does with an input.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ExpectedVerdict {
    /// It parses; `opaque` when a statement became a `Command` for a gap.
    Accepted { opaque: bool },
    /// It fails; `syntax` when the SQLSTATE is 42601.
    Rejected { syntax: bool },
}

/// An input and the verdict PostgreSQL gave it.
pub(crate) struct VerdictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_verdict: ExpectedVerdict,
}

/// An input whose tree must hold a node of `expected_node` kind at `pointer` (a JSON pointer).
pub(crate) struct ShapeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_node: &'static str,
}

/// An input whose node at `pointer` must record `expected_span`: start, end, line and column.
pub(crate) struct SpanTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_span: [usize; 4],
}
