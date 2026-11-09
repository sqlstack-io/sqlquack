//! Typed VARIANT leaves: DATEs and timestamps Snowflake keeps typed, which DuckDB writes as text.

use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::types::models::TypedLeaves;

use sqlquack_blocks::semi_structured::constants::SOURCE_VARIANT_JSON;

use crate::snowflake::conversion::_helpers::support::divergence::diverge;
use crate::snowflake::conversion::_helpers::support::scope::{Context, Emitted};

use sqlquack_semantics::snowflake::typing::models::SnowflakeType;
use sqlquack_semantics::snowflake::typing::types::ColumnSpec;

/// A converted query's SQL and result columns, or its refusal.
pub(crate) type ConvertedQuery = Result<(String, Vec<ColumnSpec>), Refusal>;

/// A converted query, refused when it compares by text where a typed leaf entered.
pub(crate) fn checked(typed: &TypedLeaves, converted: ConvertedQuery) -> ConvertedQuery {
    let (sql, columns) = converted?;
    match typed.refusal() {
        Some(refusal) => Err(refusal),
        None => Ok((sql, typed.marked(columns))),
    }
}

/// A comparison by text at `construct`, of values proven `(untainted, sourced)`, as recorded.
pub(crate) fn compared_by_text(
    (untainted, sourced): (bool, bool),
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(), Refusal> {
    if !untainted {
        context.typed.compare(construct);
    }
    match sourced {
        true => diverge(context, SOURCE_VARIANT_JSON, construct),
        false => Ok(()),
    }
}

/// A VARIANT `value` told apart from others by its text at `construct`.
pub(crate) fn compared_value(
    value: &Emitted,
    construct: &Construct,
    context: &Context<'_>,
) -> Result<(), Refusal> {
    match value.data_type == SnowflakeType::SemiStructured {
        true => compared_by_text((value.untainted, value.sourced), construct, context),
        false => Ok(()),
    }
}
