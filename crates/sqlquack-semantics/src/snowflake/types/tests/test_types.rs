use crate::snowflake::types::models::ColumnSpec;
use crate::snowflake::typing::models::SnowflakeType;

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
