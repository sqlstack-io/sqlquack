use std::path::Path;

use crate::check::models::LocalVerdict;
use crate::report::models::{Snapshot, SnapshotDiff};

/// A change from the base snapshot, and the diff it must produce.
pub(crate) struct DiffTestCase {
    pub(crate) description: &'static str,
    pub(crate) head: fn(Snapshot) -> Snapshot,
    pub(crate) expected_diff: SnapshotDiff,
}

/// Receipts and expectations set by `mutate`, a base and a head parser, and what the diff must flag.
pub(crate) struct ParserChangeTestCase {
    pub(crate) description: &'static str,
    pub(crate) mutate: fn(&Path),
    pub(crate) base: fn(&str) -> LocalVerdict,
    pub(crate) head: fn(&str) -> LocalVerdict,
    pub(crate) expected_parsed_cases_refused: Vec<&'static str>,
    pub(crate) expected_syntax_error_cases_accepted: Vec<&'static str>,
}
