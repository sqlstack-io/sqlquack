use std::path::Path;

use crate::check::models::LocalVerdict;
use crate::report::models::{ReceiptDisagreements, Snapshot, SnapshotDiff};

/// A change from the base snapshot, and the diff it must produce.
pub(crate) struct DiffTestCase {
    pub(crate) description: &'static str,
    pub(crate) head: fn(Snapshot) -> Snapshot,
    pub(crate) expected_diff: SnapshotDiff,
}

/// A change to both snapshots, and the diff it must produce.
pub(crate) struct BaseDiffTestCase {
    pub(crate) description: &'static str,
    pub(crate) base: fn(Snapshot) -> Snapshot,
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
    pub(crate) expected_documented_cases_accepted: Vec<&'static str>,
}

/// Claims set on both snapshots, the disagreements listed now and on the target branch, and the diff.
pub(crate) struct DisagreementTestCase {
    pub(crate) description: &'static str,
    pub(crate) both: fn(Snapshot) -> Snapshot,
    pub(crate) listed: fn() -> ReceiptDisagreements,
    pub(crate) target: fn() -> ReceiptDisagreements,
    pub(crate) expected_diff: SnapshotDiff,
}

/// A head parser on a documented syntax-error case, and whether the gate must fail.
pub(crate) struct GateTestCase {
    pub(crate) description: &'static str,
    pub(crate) head: fn(&str) -> LocalVerdict,
    pub(crate) expected_unlisted_syntax_errors_accepted: Vec<&'static str>,
    pub(crate) expected_fails: bool,
}
