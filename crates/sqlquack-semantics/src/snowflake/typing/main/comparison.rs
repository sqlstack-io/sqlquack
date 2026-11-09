//! `comparison`: whether two operands compare directly.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;

use crate::snowflake::typing::constants::COMPARISON;
use crate::snowflake::typing::models::SnowflakeType;

/// Whether two operands compare directly; the result is BOOLEAN.
pub fn comparison(
    left: SnowflakeType,
    right: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<SnowflakeType, String> {
    log.typing(COMPARISON);
    let comparable = matches!(
        (left, right),
        (SnowflakeType::Null, _)
            | (_, SnowflakeType::Null)
            | (
                SnowflakeType::Number { .. } | SnowflakeType::Float,
                SnowflakeType::Number { .. } | SnowflakeType::Float
            )
            | (SnowflakeType::Varchar, SnowflakeType::Varchar)
            | (SnowflakeType::Boolean, SnowflakeType::Boolean)
            | (SnowflakeType::Date, SnowflakeType::Date)
            | (SnowflakeType::Time { .. }, SnowflakeType::Time { .. })
            | (
                SnowflakeType::TimestampNtz { .. },
                SnowflakeType::TimestampNtz { .. }
            )
            | (
                SnowflakeType::TimestampTz { .. } | SnowflakeType::TimestampLtz { .. },
                SnowflakeType::TimestampTz { .. } | SnowflakeType::TimestampLtz { .. }
            )
    );
    if comparable {
        Ok(SnowflakeType::Boolean)
    } else {
        Err(format!(
            "comparing {left} with {right} needs an implicit conversion"
        ))
    }
}
