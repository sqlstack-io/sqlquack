use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

pub(crate) struct CastTestCase {
    pub(crate) description: &'static str,
    pub(crate) sql: &'static str,
    pub(crate) target: SnowflakeType,
    pub(crate) expected_sql: &'static str,
}

pub(crate) struct ArithmeticTestCase {
    pub(crate) description: &'static str,
    pub(crate) operands: [(&'static str, u8); 2],
    pub(crate) expected_sql: &'static str,
}
