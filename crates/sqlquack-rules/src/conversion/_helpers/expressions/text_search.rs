//! CHARINDEX and POSITION: where one text first occurs in another.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;

use crate::conversion::_helpers::expressions::bytes::byte_position;
use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::strings::emitted_text_operand;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, is_inert, node_construct, payload_object, require_understood,
};
use crate::conversion::constants::{
    POSITION_PAIR, TEXT_SEARCH_POSITION, TEXT_SEARCH_POSITION_TYPE,
};
use sqlquack_snowflake::typing::models::SnowflakeType;

/// The name POSITION is called by, which converts as its CHARINDEX spelling.
const POSITION_NAME: &str = "POSITION";

/// CHARINDEX(s, t) or POSITION(s, t), without a start position.
pub(crate) fn charindex_call(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let [needle, haystack] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} with a start position, which Snowflake was recorded rejecting"),
        ));
    };
    if name == POSITION_NAME {
        site.context.log.equivalence(POSITION_PAIR);
    }
    position_of((needle, haystack), construct, site)
}

/// POSITION(s IN t), the parser's `str_position` node.
pub(crate) fn position_in(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("str_position");
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &["this", "substr", "position", "occurrence"],
        &construct,
    )?;
    let given = |name: &str| payload.get(name).is_some_and(|value| !is_inert(value));
    if given("position") || given("occurrence") {
        return Err(outside_conditions(
            construct,
            "POSITION with a start position or occurrence".to_owned(),
        ));
    }
    site.context.log.equivalence(POSITION_PAIR);
    let needle = field(payload, "substr", &construct)?;
    let haystack = field(payload, "this", &construct)?;
    position_of((needle, haystack), &construct, site)
}

/// The first character position of `needle` in `haystack`, 0 when absent, 1 for empty `needle`.
fn position_of(
    (needle, haystack): (&Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let needle = convert_expression(needle, site)?;
    let haystack = convert_expression(haystack, site)?;
    if [&needle, &haystack]
        .iter()
        .any(|operand| operand.data_type == SnowflakeType::Binary)
    {
        return byte_position((&needle, &haystack), construct, site);
    }
    let _rule = site.context.log.rule(TEXT_SEARCH_POSITION);
    let needle = emitted_text_operand(needle, construct, site)?;
    let haystack = emitted_text_operand(haystack, construct, site)?;
    Ok(Emitted::new(
        format!("CAST(strpos({haystack}, {needle}) AS DECIMAL(9,0))"),
        TEXT_SEARCH_POSITION_TYPE,
    ))
}
