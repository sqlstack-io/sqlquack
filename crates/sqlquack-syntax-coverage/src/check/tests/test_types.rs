use std::path::Path;

use crate::check::models::{FormState, LocalVerdict, Reason};

/// A change to a tree where every form verifies, and what the checker must then say.
pub(crate) struct CheckTestCase {
    pub(crate) description: &'static str,
    pub(crate) mutate: fn(&Path),
    /// The current parser.
    pub(crate) parse: fn(&str) -> LocalVerdict,
    /// The parser tree the check runs against; none for an uncommitted parser.
    pub(crate) parser_tree: Option<&'static str>,
    pub(crate) form: &'static str,
    pub(crate) expected_state: FormState,
    /// A reason the form must carry; none when it must carry none.
    pub(crate) expected_reason: Option<Reason>,
    /// A fragment of a reference problem; none when there must be no problem.
    pub(crate) expected_problem: Option<&'static str>,
}
