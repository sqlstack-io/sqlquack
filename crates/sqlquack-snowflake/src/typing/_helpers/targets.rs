//! Parsing the arguments of a Snowflake type name.

use crate::typing::_helpers::numbers::number;
use crate::typing::constants::{DEFAULT_FRACTIONAL_PRECISION, MAX_PRECISION};
use crate::typing::models::{CastTarget, SnowflakeType};

pub(crate) fn parse_arguments(rest: &str, name: &str) -> Result<Vec<u32>, String> {
    let Some(inside) = rest.strip_suffix(')') else {
        return Err(format!("malformed type `{name}`"));
    };
    inside
        .split(',')
        .map(|argument| {
            argument
                .parse::<u32>()
                .map_err(|_| format!("malformed type argument in `{name}`"))
        })
        .collect()
}

pub(crate) fn number_target(arguments: &[u32], name: &str) -> Result<CastTarget, String> {
    let (precision, scale) = match arguments {
        [] => (u32::from(MAX_PRECISION), 0),
        [precision] => (*precision, 0),
        [precision, scale] => (*precision, *scale),
        _ => return Err(format!("too many arguments in `{name}`")),
    };
    if precision == 0 || precision > u32::from(MAX_PRECISION) || scale > precision {
        return Err(format!("invalid precision or scale in `{name}`"));
    }
    Ok(CastTarget {
        data_type: number(precision, scale),
        max_length: None,
    })
}

pub(crate) fn text_target(
    arguments: &[u32],
    default: Option<u32>,
    name: &str,
) -> Result<CastTarget, String> {
    let max_length = match arguments {
        [] => default,
        [length] => Some(*length),
        _ => return Err(format!("too many arguments in `{name}`")),
    };
    Ok(CastTarget {
        data_type: SnowflakeType::Varchar,
        max_length,
    })
}

/// BINARY or BINARY(n): bytes, at most `n` of them.
pub(crate) fn binary_target(arguments: &[u32], name: &str) -> Result<CastTarget, String> {
    match arguments {
        [] => Ok(CastTarget {
            data_type: SnowflakeType::Binary,
            max_length: None,
        }),
        [length] => Ok(CastTarget {
            data_type: SnowflakeType::Binary,
            max_length: Some(*length),
        }),
        _ => Err(format!("too many arguments in `{name}`")),
    }
}

pub(crate) fn fractional(arguments: &[u32], name: &str) -> Result<u8, String> {
    match arguments {
        [] => Ok(DEFAULT_FRACTIONAL_PRECISION),
        [precision] if *precision <= u32::from(DEFAULT_FRACTIONAL_PRECISION) => {
            Ok(u8::try_from(*precision).unwrap_or(DEFAULT_FRACTIONAL_PRECISION))
        }
        _ => Err(format!("invalid fractional precision in `{name}`")),
    }
}
