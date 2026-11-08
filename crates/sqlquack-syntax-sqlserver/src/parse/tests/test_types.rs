//! Test case types for the SQL Server parser.

/// What SQL Server 2022 does with an input under PARSEONLY, as the parser reports it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ExpectedVerdict {
    /// The engine accepts and the parser reads the whole text.
    Tree,
    /// The engine accepts and the parser declines, naming the construct.
    Gap(&'static str),
    /// The engine raises an error; `syntax` when the parser's refusal is a syntax error.
    Refused { syntax: bool },
}

/// An input and the verdict SQL Server gave it.
pub(crate) struct VerdictTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_verdict: ExpectedVerdict,
}

/// An input whose identifier nodes, each spanning its own text, number `expected_identifiers`.
pub(crate) struct SpanTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) expected_identifiers: usize,
}

/// An input nested past or within the limit, and the gaps it parses with on a 2 MB thread.
pub(crate) struct DepthTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_gaps: Result<Vec<&'static str>, String>,
}

pub(crate) struct ParseTimeTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: String,
    pub(crate) expected_budget_seconds: u64,
    pub(crate) expected_gaps: Result<Vec<&'static str>, String>,
}

/// An input whose node at `pointer` (a JSON pointer) must serialise to `expected_json`.
pub(crate) struct ValueTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) pointer: &'static str,
    pub(crate) expected_json: &'static str,
}
