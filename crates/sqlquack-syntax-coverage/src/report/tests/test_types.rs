use crate::report::models::{Snapshot, SnapshotDiff};

/// A change from the base snapshot, and the diff it must produce.
pub(crate) struct DiffTestCase {
    pub(crate) description: &'static str,
    pub(crate) head: fn(Snapshot) -> Snapshot,
    pub(crate) expected_diff: SnapshotDiff,
}
