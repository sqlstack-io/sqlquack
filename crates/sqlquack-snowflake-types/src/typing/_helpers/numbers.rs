//! NUMBER arithmetic shared by the typing functions.

use crate::typing::constants::{DIVISION_EXTRA_SCALE, MAX_PRECISION, SCALE_CAP};
use crate::typing::models::SnowflakeType;

pub(crate) fn number(precision: u32, scale: u32) -> SnowflakeType {
    SnowflakeType::Number {
        precision: u8::try_from(precision.min(u32::from(MAX_PRECISION))).unwrap_or(MAX_PRECISION),
        scale: u8::try_from(scale.min(u32::from(MAX_PRECISION))).unwrap_or(MAX_PRECISION),
    }
}

pub(crate) fn capped(leading: u32, scale: u32) -> SnowflakeType {
    number((leading + scale).min(u32::from(MAX_PRECISION)), scale)
}

/// Leading digits and scale of a NUMBER.
pub(crate) type NumberParts = (u32, u32);

pub(crate) fn number_parts(data_type: SnowflakeType) -> Option<NumberParts> {
    match data_type {
        SnowflakeType::Number { precision, scale } => {
            Some((u32::from(precision - scale), u32::from(scale)))
        }
        _ => None,
    }
}

/// Exact or FLOAT operands only; anything else has no proven arithmetic type.
pub(crate) fn arithmetic_operands(
    left: SnowflakeType,
    right: SnowflakeType,
    operator: &str,
) -> Result<Option<(NumberParts, NumberParts)>, String> {
    match (left, right) {
        (SnowflakeType::Float, SnowflakeType::Float | SnowflakeType::Number { .. })
        | (SnowflakeType::Number { .. }, SnowflakeType::Float) => Ok(None),
        _ => match (number_parts(left), number_parts(right)) {
            (Some(left), Some(right)) => Ok(Some((left, right))),
            _ => Err(format!(
                "`{operator}` of {left} and {right} has no proven type"
            )),
        },
    }
}

pub(crate) fn division_scale(scale: u32) -> u32 {
    scale.max((scale + u32::from(DIVISION_EXTRA_SCALE)).min(u32::from(SCALE_CAP)))
}

pub(crate) fn unify_pair(
    left: SnowflakeType,
    right: SnowflakeType,
) -> Result<SnowflakeType, String> {
    match (left, right) {
        (SnowflakeType::Null, other) | (other, SnowflakeType::Null) => Ok(other),
        (SnowflakeType::Float, SnowflakeType::Float | SnowflakeType::Number { .. })
        | (SnowflakeType::Number { .. }, SnowflakeType::Float) => Ok(SnowflakeType::Float),
        (
            SnowflakeType::Number {
                precision: left_precision,
                scale: left_scale,
            },
            SnowflakeType::Number {
                precision: right_precision,
                scale: right_scale,
            },
        ) => Ok(capped(
            u32::from((left_precision - left_scale).max(right_precision - right_scale)),
            u32::from(left_scale.max(right_scale)),
        )),
        (
            SnowflakeType::TimestampNtz {
                precision: left_precision,
            },
            SnowflakeType::TimestampNtz {
                precision: right_precision,
            },
        ) => Ok(SnowflakeType::TimestampNtz {
            precision: left_precision.max(right_precision),
        }),
        (SnowflakeType::TimestampNtz { precision: 9 }, SnowflakeType::Date)
        | (SnowflakeType::Date, SnowflakeType::TimestampNtz { precision: 9 }) => {
            Ok(SnowflakeType::TimestampNtz { precision: 9 })
        }
        (
            SnowflakeType::TimestampNtz { precision: 9 },
            zoned @ SnowflakeType::TimestampLtz { precision: 9 },
        )
        | (
            zoned @ SnowflakeType::TimestampLtz { precision: 9 },
            SnowflakeType::TimestampNtz { precision: 9 },
        )
        | (
            SnowflakeType::TimestampNtz { precision: 9 },
            zoned @ SnowflakeType::TimestampTz { precision: 9 },
        )
        | (
            zoned @ SnowflakeType::TimestampTz { precision: 9 },
            SnowflakeType::TimestampNtz { precision: 9 },
        ) => Ok(zoned),
        (SnowflakeType::Varchar, SnowflakeType::Varchar)
        | (SnowflakeType::Boolean, SnowflakeType::Boolean)
        | (SnowflakeType::Date, SnowflakeType::Date) => Ok(left),
        _ if left == right && left != SnowflakeType::SemiStructured => Ok(left),
        _ => Err(format!("no proven common type for {left} and {right}")),
    }
}
