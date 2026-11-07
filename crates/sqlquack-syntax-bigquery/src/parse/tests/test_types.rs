//! Test case types for the BigQuery GoogleSQL parser.

use crate::parse::errors::StrictFailure;
use sqlquack_syntax::Gap;

/// What GoogleSQL's parser does with an input.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ExpectedVerdict {
    /// It parses; `opaque` when a statement was kept as a gap.
    Accepted { opaque: bool },
    /// It fails; `syntax` when it is a syntax refusal (`Syntax error: ...`).
    Rejected { syntax: bool },
}

/// An input and the verdict GoogleSQL gives it.
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

/// An input's strict statement and gap counts, or `Err(Some(syntax))` for a parse error, `Err(None)` uncovered.
pub(crate) struct StrictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outcome: Result<(usize, usize), Option<bool>>,
}

/// An input and the gaps lenient parsing keeps for it.
pub(crate) struct GapTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_gaps: Vec<Gap>,
}

/// A strict failure and its parse error's `syntax` flag, `None` when it carries no parse error.
pub(crate) struct StrictFailureTestCase {
    pub(crate) description: &'static str,
    pub(crate) failure: StrictFailure,
    pub(crate) expected_syntax: Option<bool>,
}

/// Two inputs, and whether they parse to the same statements (or the same error).
pub(crate) struct TreePairTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) other_sql: &'static str,
    pub(crate) expected_same: bool,
}

/// Two MATCH_RECOGNIZE patterns, each with its options text, and whether they parse the same.
pub(crate) struct MatchRecognizeTestCase {
    pub(crate) description: &'static str,
    pub(crate) pattern: (&'static str, &'static str),
    pub(crate) other_pattern: (&'static str, &'static str),
    pub(crate) expected_same: bool,
}
