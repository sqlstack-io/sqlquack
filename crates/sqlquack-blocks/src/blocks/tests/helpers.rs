use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

/// NUMBER(`precision`, `scale`).
pub(crate) const fn number(precision: u8, scale: u8) -> SnowflakeType {
    SnowflakeType::Number { precision, scale }
}
