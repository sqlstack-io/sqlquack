//! Test case types for the ClickHouse parser.

/// What ClickHouse 26.1's multi-statement parser (`clickhouse format -n`) does with an input.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ExpectedVerdict {
    /// It parses; `gap` names the construct when a statement became a `Command` for it.
    Accepted { gap: Option<&'static str> },
    /// It fails; `syntax` when the error is code 62 (`SYNTAX_ERROR`).
    Rejected { syntax: bool },
}

/// An input and the verdict ClickHouse gave it.
pub(crate) struct VerdictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_verdict: ExpectedVerdict,
}

/// An input whose node at `pointer` (a JSON pointer) must serialise to `expected_json`.
pub(crate) struct ValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_json: &'static str,
}

/// The stack each depth case parses on: 2 MB, a common thread default.
pub(crate) const SMALL_STACK: usize = 2 * 1024 * 1024;

/// An input nested `depth` levels by `build`, and its statement count or refusal message.
pub(crate) struct DepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) build: fn(usize) -> String,
    pub(crate) depth: usize,
    pub(crate) expected_outcome: Result<usize, &'static str>,
}
