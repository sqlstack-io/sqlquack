//! VARIANT values built from typed scalars, FLOAT among them, held as DuckDB `VARIANT`.

use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::held::main::variant_built::variant_built;
use sqlquack_duckdb::held::main::variant_decimal_leaf::variant_decimal_leaf;
use sqlquack_duckdb::held::main::variant_double_leaf::variant_double_leaf;
use sqlquack_duckdb::held::main::variant_integer_leaf::variant_integer_leaf;
use sqlquack_duckdb::held::main::variant_json::variant_json;
use sqlquack_duckdb::held::main::variant_json_leaf::variant_json_leaf;
use sqlquack_duckdb::semi_structured::constants::JSON_FIDELITY;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::objects::as_json;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Context, Emitted, VariantRoot};

/// Whether a builder of `values` holds its result as a DuckDB VARIANT: one keeps a FLOAT's type.
pub(crate) fn builds_held(values: &[Emitted]) -> bool {
    values
        .iter()
        .any(|value| value.data_type == SnowflakeType::Float || value.is_variant_held())
}

/// A builder's value as JSON; in a `held` build, leaves are guarded to keep their types.
pub(crate) fn leaf(
    emitted: Emitted,
    held: bool,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<String, Refusal> {
    let log = context.log;
    if emitted.is_variant_held() {
        return Ok(variant_json(&emitted.sql, log));
    }
    if !held {
        return as_json(emitted, construct, context);
    }
    match emitted.data_type {
        SnowflakeType::Float => {
            diverge(context, JSON_FIDELITY, construct)?;
            Ok(variant_double_leaf(&emitted.sql, log))
        }
        SnowflakeType::Number { scale: 0, .. } => {
            diverge(context, JSON_FIDELITY, construct)?;
            Ok(variant_integer_leaf(&emitted.sql, log))
        }
        SnowflakeType::Number { .. } => {
            diverge(context, JSON_FIDELITY, construct)?;
            context.typed.enter_decimal();
            Ok(variant_decimal_leaf(&emitted.sql, log))
        }
        SnowflakeType::SemiStructured => {
            diverge(context, JSON_FIDELITY, construct)?;
            Ok(variant_json_leaf(&emitted.sql, log))
        }
        _ => as_json(emitted, construct, context),
    }
}

/// The values of a builder as JSON leaves, and whether the build is held.
pub(crate) fn leaves(
    values: Vec<Emitted>,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(Vec<String>, bool), Refusal> {
    let held = builds_held(&values);
    let leaves = values
        .into_iter()
        .map(|value| leaf(value, held, construct, context))
        .collect::<Result<Vec<String>, Refusal>>()?;
    Ok((leaves, held))
}

/// A builder's JSON `sql`, as a VARIANT-held value when `held`.
pub(crate) fn built(sql: String, held: bool, root: VariantRoot, context: &Context<'_>) -> Emitted {
    match held {
        true => Emitted::variant_held(variant_built(&sql, context.log), root),
        false => Emitted::new(sql, SnowflakeType::SemiStructured),
    }
}

/// A VARIANT operand as JSON: a VARIANT-held value read through DuckDB's JSON, with its root.
pub(crate) fn read_json(emitted: &Emitted, context: &Context<'_>) -> String {
    match emitted.is_variant_held() {
        true => variant_json(&emitted.sql, context.log),
        false => emitted.sql.clone(),
    }
}
