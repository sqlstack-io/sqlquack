//! CHARINDEX and POSITION: where one text first occurs in another.

use serde_json::Value;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;

use crate::snowflake::conversion::_helpers::expressions::strings::text_operand;
use crate::snowflake::conversion::_helpers::expressions::text_functions::integer_operand;
use crate::snowflake::conversion::_helpers::support::scope::Emitted;
use crate::snowflake::conversion::_helpers::support::site::Site;
use crate::snowflake::conversion::_helpers::support::tree::{
    field, is_inert, node_construct, payload_object, require_understood,
};
use crate::snowflake::conversion::constants::{
    POSITION_PAIR, TEXT_SEARCH_POSITION, TEXT_SEARCH_POSITION_TYPE,
};

/// The name POSITION is called by, which converts as its CHARINDEX spelling.
const POSITION_NAME: &str = "POSITION";

/// CHARINDEX(s, t [, start]) or POSITION(s, t [, start]).
pub(crate) fn charindex_call(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let (needle, haystack, start) = match arguments {
        [needle, haystack] => (needle, haystack, None),
        [needle, haystack, start] => (needle, haystack, Some(start)),
        _ => {
            return Err(outside_conditions(
                construct.clone(),
                format!("{name} with other than two or three arguments"),
            ));
        }
    };
    if name == POSITION_NAME {
        site.context.log.equivalence(POSITION_PAIR);
    }
    match start {
        None => position_of((needle, haystack), construct, site),
        Some(start) => position_from((needle, haystack, start), construct, site),
    }
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
    let _rule = site.context.log.rule(TEXT_SEARCH_POSITION);
    let needle = text_operand(needle, construct, site)?;
    let haystack = text_operand(haystack, construct, site)?;
    Ok(Emitted::new(
        format!("CAST(strpos({haystack}, {needle}) AS DECIMAL(9,0))"),
        TEXT_SEARCH_POSITION_TYPE,
    ))
}

/// The first position of `needle` in `haystack` at or after `start` (below 1 reads as 1), as recorded.
fn position_from(
    (needle, haystack, start): (&Value, &Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_SEARCH_POSITION);
    let needle = text_operand(needle, construct, site)?;
    let haystack = text_operand(haystack, construct, site)?;
    let start = integer_operand(start, construct, site)?;
    Ok(Emitted::new(
        format!(
            "CAST(list_transform([struct_pack(n := {needle}, h := {haystack}, p := {start})], \
             lambda s: CASE WHEN s['p'] IS NOT NULL AND s['h'] IS NOT NULL AND s['n'] IS NOT NULL \
             THEN list_transform([greatest(s['p'], 1)], \
             lambda f: list_transform([strpos(substr(s['h'], f), s['n'])], \
             lambda i: CASE WHEN i = 0 OR f > greatest(length(s['h']), 1) THEN 0 ELSE i + f - 1 END)[1])[1] \
             END)[1] AS DECIMAL(9,0))"
        ),
        TEXT_SEARCH_POSITION_TYPE,
    ))
}
