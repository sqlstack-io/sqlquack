//! Test case types for the DuckDB parser.

use sqlquack_syntax::Gap;

use crate::parse::errors::StrictFailure;

/// What DuckDB 2.0.0's parser does with an input.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ExpectedVerdict {
    /// It parses; `opaque` when a statement was kept as a gap.
    Accepted { opaque: bool },
    /// It fails; `syntax` when the grammar or the scanner refuses it, not a transformer check.
    Rejected { syntax: bool },
}

/// An input and the verdict DuckDB gives it.
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

/// An input's strict statement and gap counts, or the parse error's `syntax` flag when refused.
pub(crate) struct StrictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_outcome: Result<(usize, usize), Option<bool>>,
}

/// An input and the gaps, by construct and byte range, that parsing it records.
pub(crate) struct GapTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_gaps: &'static [Gap],
}

/// A strict failure and the `syntax` flag of the parse error it carries, if it carries one.
pub(crate) struct FailureTestCase {
    pub(crate) description: &'static str,
    pub(crate) failure: StrictFailure,
    pub(crate) expected_syntax: Option<bool>,
}

/// A corpus file, relative to the crate, its row count and the ids that fail strict parsing.
pub(crate) struct CorpusTestCase {
    pub(crate) description: &'static str,
    pub(crate) corpus: &'static str,
    pub(crate) expected_rows: usize,
    pub(crate) expected_refused: &'static [&'static str],
}

/// A corpus whose rows carry DuckDB's recorded verdict, and how many rows reach a gap.
pub(crate) struct RecordedCorpusTestCase {
    pub(crate) description: &'static str,
    pub(crate) corpus: &'static str,
    pub(crate) expected_rows: usize,
    pub(crate) expected_gapped: usize,
}

/// A generated input, too long to write out, and the message prefix it is refused with, or `None` when it parses.
pub(crate) struct NestingTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: fn() -> String,
    pub(crate) expected_refusal: Option<&'static str>,
}
