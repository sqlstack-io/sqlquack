use arrow::array::ArrayRef;

use crate::comparison::models::{MismatchKind, RowOrder};

/// One result column for a test: name, declared or exported type, and values.
pub(crate) struct TestColumn {
    pub(crate) name: &'static str,
    pub(crate) snowflake_type: &'static str,
    pub(crate) values: ArrayRef,
}

pub(crate) struct ReplayTestCase {
    pub(crate) description: &'static str,
    pub(crate) recorded: Vec<TestColumn>,
    pub(crate) replayed: Vec<TestColumn>,
    pub(crate) order: RowOrder,
    pub(crate) float64_ulps: u32,
    pub(crate) expected_mismatch: Option<MismatchKind>,
}

pub(crate) struct RecordingsTestCase {
    pub(crate) description: &'static str,
    pub(crate) stored: Vec<TestColumn>,
    pub(crate) fresh: Vec<TestColumn>,
    pub(crate) expected_mismatch: Option<MismatchKind>,
}
