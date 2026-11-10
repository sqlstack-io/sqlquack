//! `decimal_arithmetic`: exact decimal `+`, `-` or `*`: operands widened to 38 digits, then the Snowflake type.

use sqlquack_diagnostics::provenance::models::ProvenanceLog;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::blocks::_helpers::numbers::widened_literal;
use crate::blocks::constants::DECIMAL_ARITHMETIC;
use crate::blocks::main::cast::cast;

/// Exact decimal `+`, `-` or `*`: operands widened to 38 digits, then the Snowflake type.
pub fn decimal_arithmetic(
    operator: &str,
    [left, right]: [(&str, u8); 2],
    result: SnowflakeType,
    log: &ProvenanceLog,
) -> Result<String, String> {
    log.building_block(DECIMAL_ARITHMETIC);
    let widened = |(sql, scale): (&str, u8)| {
        let sql = widened_literal(sql, 38, scale).unwrap_or(sql);
        format!("CAST({sql} AS DECIMAL(38,{scale}))")
    };
    cast(
        &format!("({} {operator} {})", widened(left), widened(right)),
        result,
        log,
    )
}
