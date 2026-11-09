use crate::schemas::models::Holding;
use crate::snowflake::typing::models::SnowflakeType;
use crate::snowflake::typing::types::ColumnSpec;

pub(crate) struct SchemaColumnsTestCase {
    pub(crate) description: &'static str,
    pub(crate) column: ColumnSpec,
    pub(crate) expected_marks: (bool, bool),
    pub(crate) expected_refusal: bool,
}

pub(crate) struct UtcOffsetTestCase {
    pub(crate) description: &'static str,
    pub(crate) values: Vec<(SnowflakeType, bool)>,
    pub(crate) result: SnowflakeType,
    pub(crate) expected_utc_offset: bool,
}

pub(crate) struct StoredColumnsTestCase {
    pub(crate) description: &'static str,
    pub(crate) written: Holding,
    pub(crate) expected_read: Holding,
}
