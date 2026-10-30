use crate::types::models::Holding;

pub(crate) struct StoredColumnsTestCase {
    pub(crate) description: &'static str,
    pub(crate) written: Holding,
    pub(crate) expected_read: Holding,
}
