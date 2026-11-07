//! Test case types for the PostgreSQL parser.

use crate::parse::errors::ParseError;

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

/// A type string and 18.6's `format_type` name, or `Err(syntax)` when it refuses it (42601 or not).
pub(crate) struct TypeTextTestCase {
    pub(crate) description: &'static str,
    pub(crate) text: &'static str,
    pub(crate) expected_name: Result<&'static str, bool>,
}

/// An input's strict statement count, or `Err(Some(syntax))` for a parse error, `Err(None)` uncovered.
pub(crate) struct StrictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outcome: Result<usize, Option<bool>>,
}

/// An input whose node at `pointer` (a JSON pointer) must serialise to `expected_json`.
pub(crate) struct ValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_json: &'static str,
}

/// Token ranges over `sql`, and the first byte they and the blanks between them leave uncovered.
pub(crate) struct GapTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) ranges: &'static [(usize, usize)],
    pub(crate) faulted: bool,
    pub(crate) expected_gap: Option<usize>,
}

/// A nesting built from its parts, the entry point that reads it, and its `(message, syntax)`.
pub(crate) struct NestingTestCase {
    pub(crate) description: &'static str,
    pub(crate) prefix: &'static str,
    pub(crate) open: &'static str,
    pub(crate) innermost: &'static str,
    pub(crate) close: &'static str,
    pub(crate) read: fn(&str) -> Option<ParseError>,
    pub(crate) levels: usize,
    pub(crate) expected_error: Option<(&'static str, bool)>,
}
