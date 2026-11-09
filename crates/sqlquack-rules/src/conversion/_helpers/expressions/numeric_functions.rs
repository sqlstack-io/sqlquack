//! ABS, SIGN, ROUND, CEIL, FLOOR, TRUNC and the bitwise functions of NUMBER and FLOAT values.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_blocks::blocks::main::cast::cast;
use sqlquack_blocks::floats::main::exponential::exponential;
use sqlquack_blocks::floats::main::natural_logarithm::natural_logarithm;
use sqlquack_blocks::floats::main::power_of_ten::power_of_ten;
use sqlquack_blocks::floats::main::square_root::square_root;
use sqlquack_snowflake_types::typing::constants::MAX_PRECISION;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::casts::number_literal_text;
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::temporal_functions::date_trunc;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;

use crate::conversion::constants::{
    BITWISE, HALF_AWAY_FROM_ZERO, HALF_TO_EVEN, LN_NAME, LOGARITHM, NUMERIC_FUNCTION, POWER_BASE,
    POWER_BASE_TWO, POWER_RULE, ROUND_FUNCTION, ROUND_WITH_MODE_ARGUMENTS, ROUNDING, SIGN_RULE,
    SQRT_NAME,
};
use sqlquack_policy::policy::constants::NUMERIC_RANGE;

/// ABS of a NUMBER or FLOAT, of the operand's type.
pub(crate) fn absolute(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(NUMERIC_FUNCTION);
    let [value] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "ABS with these arguments".to_owned(),
        ));
    };
    let operand = convert_expression(value, site)?;
    match operand.data_type {
        SnowflakeType::Number { .. } | SnowflakeType::Float => Ok(Emitted::new(
            cast(
                &format!("abs({})", operand.sql),
                operand.data_type,
                site.context.log,
            )
            .map_err(|reason| outside_conditions(construct.clone(), reason))?,
            operand.data_type,
        )),
        other => Err(outside_conditions(
            construct.clone(),
            format!("ABS of {other}"),
        )),
    }
}

/// LN, EXP or SQRT of a NUMBER or FLOAT, a FLOAT, failing where Snowflake fails.
pub(crate) fn logarithm(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(LOGARITHM);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value] = arguments else {
        return Err(refuse(format!("{name} with these arguments")));
    };
    let operand = convert_expression(value, site)?;
    if !matches!(
        operand.data_type,
        SnowflakeType::Number { .. } | SnowflakeType::Float
    ) {
        return Err(refuse(format!("{name} of {}", operand.data_type)));
    }
    let sql = match name {
        LN_NAME => natural_logarithm(&operand.sql, site.context.log),
        SQRT_NAME => square_root(&operand.sql, site.context.log),
        _ => {
            diverge(site.context, NUMERIC_RANGE, construct)?;
            exponential(&operand.sql, site.context.log)
        }
    };
    Ok(Emitted::new(sql, SnowflakeType::Float))
}

/// POWER(10, n) or POWER(2, n) of a whole NUMBER exponent, as a FLOAT; other bases refuse.
pub(crate) fn power(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(POWER_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [base, exponent] = arguments else {
        return Err(refuse("POWER with these arguments".to_owned()));
    };
    let base = convert_expression(base, site)?;
    let two = base.literal == Some(Literal::Number(POWER_BASE_TWO.to_owned()));
    if base.literal != Some(Literal::Number(POWER_BASE.to_owned())) && !two {
        return Err(refuse(
            "POWER of a base other than the constant 10 or 2, whose rounding no recording shows"
                .to_owned(),
        ));
    }
    let exponent = convert_expression(exponent, site)?;
    match (exponent.data_type, two) {
        (SnowflakeType::Number { scale: 0, .. } | SnowflakeType::Null, true) => Ok(Emitted::new(
            format!("pow(CAST(2 AS DOUBLE), CAST({} AS DOUBLE))", exponent.sql),
            SnowflakeType::Float,
        )),
        (SnowflakeType::Number { scale: 0, .. } | SnowflakeType::Null, false) => Ok(Emitted::new(
            power_of_ten(&exponent.sql, site.context.log),
            SnowflakeType::Float,
        )),
        (other, _) => Err(refuse(format!(
            "POWER({}, n) of a {other} exponent",
            if two { POWER_BASE_TWO } else { POWER_BASE }
        ))),
    }
}

/// BITAND, BITOR and BITXOR of integer NUMBERs, as 128-bit two's complement.
pub(crate) fn bitwise(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(BITWISE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [left, right] = arguments else {
        return Err(refuse(format!("{name} with these arguments")));
    };
    let mut integers: Vec<String> = Vec::with_capacity(2);
    let mut widest: u8 = 1;
    for argument in [left, right] {
        let operand = convert_expression(argument, site)?;
        match operand.data_type {
            SnowflakeType::Number {
                scale: 0,
                precision,
            } => {
                widest = widest.max(precision);
                integers.push(format!("CAST({} AS HUGEINT)", operand.sql));
            }
            SnowflakeType::Null => integers.push(format!("CAST({} AS HUGEINT)", operand.sql)),
            other => return Err(refuse(format!("{name} of {other}"))),
        }
    }
    let symbol = match name {
        "BITAND" => "&",
        "BITOR" => "|",
        _ => "xor",
    };
    let combined = match symbol {
        "xor" => format!("xor({}, {})", integers[0], integers[1]),
        _ => format!("({} {symbol} {})", integers[0], integers[1]),
    };
    let widened = match name {
        "BITOR" => (widest + 1).min(MAX_PRECISION),
        _ => widest,
    };
    let result = SnowflakeType::Number {
        precision: widened,
        scale: 0,
    };
    Ok(Emitted::new(
        cast(&combined, result, site.context.log).map_err(refuse)?,
        result,
    ))
}

/// ROUND to literal places, CEIL, FLOOR and TRUNC of a NUMBER or FLOAT, typed as recorded.
pub(crate) fn rounding(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(ROUNDING);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let places_of = |places: &Value| {
        literal_places(places)
            .ok_or_else(|| refuse(format!("{name} to places other than a literal integer")))
    };
    let (value, places, half_to_even) = match (name, arguments) {
        (_, [value]) => (value, 0_i32, false),
        ("TRUNC" | "TRUNCATE", [value, unit]) if text_literal(unit, site)? => {
            return date_trunc(unit, value, construct, site);
        }
        ("ROUND" | "TRUNC" | "TRUNCATE", [value, places]) => (value, places_of(places)?, false),
        ("ROUND", [value, places, mode]) => {
            let half_to_even = match convert_expression(mode, site)?.literal {
                Some(Literal::Text(mode)) if mode == HALF_TO_EVEN => true,
                Some(Literal::Text(mode)) if mode == HALF_AWAY_FROM_ZERO => false,
                _ => {
                    return Err(refuse(
                        "ROUND with a rounding mode no recording shows".to_owned(),
                    ));
                }
            };
            (value, places_of(places)?, half_to_even)
        }
        _ => return Err(refuse(format!("{name} with these arguments"))),
    };
    let operand = convert_expression(value, site)?;
    let function = match (name, half_to_even) {
        (_, true) => "round_even".to_owned(),
        ("TRUNCATE", false) => "trunc".to_owned(),
        (_, false) => name.to_ascii_lowercase(),
    };
    match operand.data_type {
        SnowflakeType::Float if arguments.len() == ROUND_WITH_MODE_ARGUMENTS => Err(refuse(
            "ROUND of a FLOAT with a rounding mode, which no recording shows".to_owned(),
        )),
        SnowflakeType::Float if name == ROUND_FUNCTION && places != 0 => Ok(Emitted::new(
            format!(
                "(round(CAST({} AS DOUBLE) * CAST(1e{places} AS DOUBLE)) / CAST(1e{places} AS DOUBLE) \
                 + 0.0)",
                operand.sql
            ),
            SnowflakeType::Float,
        )),
        SnowflakeType::Float if places > 0 => Ok(Emitted::new(
            format!(
                "(trunc(CAST({} AS DOUBLE) * CAST(1e{places} AS DOUBLE)) / CAST(1e{places} AS DOUBLE) \
                 + 0.0)",
                operand.sql
            ),
            SnowflakeType::Float,
        )),
        SnowflakeType::Float if places < 0 => Ok(Emitted::new(
            format!(
                "(trunc(CAST({} AS DOUBLE) / CAST(1e{} AS DOUBLE)) * CAST(1e{} AS DOUBLE) + 0.0)",
                operand.sql, -places, -places
            ),
            SnowflakeType::Float,
        )),
        SnowflakeType::Float => Ok(Emitted::new(
            format!(
                "(CAST({function}({}{}) AS DOUBLE) + 0.0)",
                operand.sql,
                places_argument(name, places)
            ),
            SnowflakeType::Float,
        )),
        SnowflakeType::Number { precision, scale } => {
            let result = match i32::from(scale) > places {
                true => SnowflakeType::Number {
                    precision: (precision + 1).min(MAX_PRECISION),
                    scale: u8::try_from(places.max(0)).unwrap_or(0),
                },
                false => operand.data_type,
            };
            let sql = format!(
                "{function}({}{})",
                operand.sql,
                places_argument(name, places)
            );
            Ok(Emitted::new(
                cast(&sql, result, site.context.log).map_err(refuse)?,
                result,
            ))
        }
        other => Err(refuse(format!("{name} of {other}"))),
    }
}

/// SIGN: -1, 0 or 1, as NUMBER(2,0) for NUMBERs and FLOAT for FLOATs.
pub(crate) fn sign(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(SIGN_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [value] = arguments else {
        return Err(refuse("SIGN with these arguments".to_owned()));
    };
    let operand = convert_expression(value, site)?;
    let result = match operand.data_type {
        SnowflakeType::Float => SnowflakeType::Float,
        SnowflakeType::Number { .. } => SnowflakeType::Number {
            precision: 2,
            scale: 0,
        },
        other => return Err(refuse(format!("SIGN of {other}"))),
    };
    Ok(Emitted::new(
        cast(&format!("sign({})", operand.sql), result, site.context.log).map_err(refuse)?,
        result,
    ))
}

/// Whether `node` is literal text, as TRUNC's unit is.
fn text_literal(node: &Value, site: Site<'_>) -> Result<bool, Refusal> {
    Ok(matches!(
        convert_expression(node, site)?.literal,
        Some(Literal::Text(_))
    ))
}

fn places_argument(name: &str, places: i32) -> String {
    match name {
        "ROUND" | "TRUNC" | "TRUNCATE" if places != 0 => format!(", {places}"),
        "ROUND" => format!(", {places}"),
        _ => String::new(),
    }
}

/// A literal integer number of places, possibly negative.
fn literal_places(node: &Value) -> Option<i32> {
    match number_literal_text(node)?.parse::<i32>() {
        Ok(places) if places.abs() <= i32::from(MAX_PRECISION) => Some(places),
        Ok(_) | Err(_) => None,
    }
}
