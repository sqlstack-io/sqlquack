//! ABS, SIGN, ROUND, CEIL, FLOOR, TRUNC and the bitwise functions of NUMBER and FLOAT values.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::cast::cast;
use sqlquack_snowflake::typing::constants::MAX_PRECISION;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::casts::number_literal_text;
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::outside_conditions;
use crate::conversion::constants::{BITWISE, NUMERIC_FUNCTION, ROUNDING, SIGN_RULE};

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
    let (value, places) = match (name, arguments) {
        (_, [value]) => (value, 0_i32),
        ("ROUND", [value, places]) => (
            value,
            literal_places(places)
                .ok_or_else(|| refuse("ROUND to places other than a literal integer".to_owned()))?,
        ),
        _ => return Err(refuse(format!("{name} with these arguments"))),
    };
    let operand = convert_expression(value, site)?;
    let function = name.to_ascii_lowercase();
    match operand.data_type {
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

fn places_argument(name: &str, places: i32) -> String {
    match name {
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
