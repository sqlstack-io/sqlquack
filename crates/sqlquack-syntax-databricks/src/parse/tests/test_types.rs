//! Test case types for the Databricks SQL parser.

use sqlquack_syntax::Gap;

use crate::parse::errors::StrictFailure;

/// What Databricks' parser does with an input.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ExpectedVerdict {
    /// It parses; `opaque` when a statement was kept as a gap.
    Accepted { opaque: bool },
    /// It fails; `syntax` when it is a syntax refusal (SQLSTATE 42601).
    Rejected { syntax: bool },
}

/// An input and the verdict Databricks gives it.
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

/// An input's strict statement and gap counts, or whether its failure is a syntax refusal.
pub(crate) struct StrictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outcome: Result<(usize, usize), Option<bool>>,
}

/// An input and the gaps its parse must name.
pub(crate) struct GapTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_gaps: Vec<Gap>,
}

/// A strict failure and whether its parse error is a syntax refusal, or none for no parse error.
pub(crate) struct StrictFailureTestCase {
    pub(crate) description: &'static str,
    pub(crate) failure: StrictFailure,
    pub(crate) expected_syntax: Option<bool>,
}

/// Nested chains: `levels` parentheses each closing `length` links, and the refusal expected.
pub(crate) struct ChainTestCase {
    pub(crate) description: &'static str,
    pub(crate) levels: usize,
    pub(crate) length: usize,
    pub(crate) expected_refusal: Option<&'static str>,
}

/// A generated input too long to write out, and its refusal message, or `None` when it parses.
pub(crate) struct NestingTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: fn() -> String,
    pub(crate) expected_refusal: Option<&'static str>,
}
