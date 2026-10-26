//! HASH, a known divergence: a hash that keeps Snowflake's equalities, not its values.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::hashing::main::hash_value::hash_value;
use sqlquack_duckdb::semi_structured::main::json_hash_key::json_hash_key;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::outside_conditions;
use crate::conversion::constants::{HASH_DIVERGENCE, HASH_RESULT_PRECISION, HASH_RULE};

/// HASH of one or more values, each held so that values Snowflake finds equal hash equally.
pub(crate) fn hash_call(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(HASH_RULE);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    if arguments.is_empty() {
        return Err(refuse("HASH without arguments, or of *".to_owned()));
    }
    diverge(site.context, HASH_DIVERGENCE, construct)?;
    let mut keys: Vec<String> = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let operand = convert_expression(argument, site)?;
        keys.push(match operand.data_type {
            SnowflakeType::Varchar
            | SnowflakeType::Null
            | SnowflakeType::Boolean
            | SnowflakeType::Date => operand.sql,
            SnowflakeType::Number { scale: 0, .. } => format!("CAST({} AS HUGEINT)", operand.sql),
            SnowflakeType::SemiStructured => json_hash_key(&operand.sql, site.context.log),
            other => {
                return Err(refuse(format!(
                    "HASH of {other}, whose equal values no recording shows hashing alike"
                )));
            }
        });
    }
    Ok(Emitted::new(
        hash_value(&keys, site.context.log),
        SnowflakeType::Number {
            precision: HASH_RESULT_PRECISION,
            scale: 0,
        },
    ))
}
