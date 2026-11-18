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

/// A refused input and the byte offset its error is at.
pub(crate) struct RefusalTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_position: usize,
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

/// Texts that write one form in different ways, whose trees must all differ.
pub(crate) struct DistinctTreesTestCase {
    pub(crate) description: &'static str,
    pub(crate) written: &'static [&'static str],
    pub(crate) expected_distinct_trees: usize,
}

/// A form of `TREED_FORMS`, written as `sql`, before `ALTER TABLE t1 ALTER COLUMN c1` in one compound and in two statements.
pub(crate) struct TreedFormTestCase {
    pub(crate) description: &'static str,
    pub(crate) construct: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_in_compound: ExpectedVerdict,
    pub(crate) expected_as_script: ExpectedVerdict,
}

/// A treed form written around `{E}`, a left-deep sum the tree depth limit would refuse were it kept.
pub(crate) struct DeepValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) shape: &'static str,
    pub(crate) expected_verdict: ExpectedVerdict,
}
