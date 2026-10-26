//! UPPER, LOWER, TRIM and concatenation.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::full_case_lower::full_case_lower;
use sqlquack_duckdb::blocks::main::full_case_upper::full_case_upper;
use sqlquack_duckdb::blocks::main::null_propagating_concat::null_propagating_concat;
use sqlquack_duckdb::text::main::character_edit_distance::character_edit_distance;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::scope::Emitted;
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, missing, node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    CASE_MAPPING, CONCAT, CONCAT_WS, EDIT_DISTANCE, TEXT_AFFIX, TEXT_CONTAINS, TRIM,
};

/// What TRIM removes without a character list: spaces only, as in Snowflake.
const SPACE_ONLY: &str = "' '";

/// A VARCHAR (or NULL) operand's SQL.
pub(crate) fn text_operand(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<String, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::Varchar | SnowflakeType::Null => Ok(emitted.sql),
        other => Err(outside_conditions(
            construct.clone(),
            format!("a {other} operand needs an implicit conversion to text"),
        )),
    }
}

/// STARTSWITH or ENDSWITH on VARCHAR operands.
pub(crate) fn text_affix(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let _rule = site.context.log.rule(TEXT_AFFIX);
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &["this", "expression", "original_name"],
        &construct,
    )?;
    let text = text_operand(field(payload, "this", &construct)?, &construct, site)?;
    let affix = text_operand(field(payload, "expression", &construct)?, &construct, site)?;
    let function = match kind {
        "starts_with" => "starts_with",
        _ => "suffix",
    };
    Ok(Emitted::new(
        format!("{function}({text}, {affix})"),
        SnowflakeType::Boolean,
    ))
}

/// CONTAINS of VARCHAR operands, compared bytewise.
pub(crate) fn text_contains(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_CONTAINS);
    let [text, part] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "CONTAINS takes two arguments".to_owned(),
        ));
    };
    let text = text_operand(text, construct, site)?;
    let part = text_operand(part, construct, site)?;
    Ok(Emitted::new(
        format!("contains({text}, {part})"),
        SnowflakeType::Boolean,
    ))
}

/// EDITDISTANCE(a, b): Levenshtein distance in characters, as Snowflake's NUMBER(9,0).
pub(crate) fn edit_distance(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(EDIT_DISTANCE);
    let [left, right] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "EDITDISTANCE with a maximum distance, which no recording shows".to_owned(),
        ));
    };
    let left = text_operand(left, construct, site)?;
    let right = text_operand(right, construct, site)?;
    Ok(Emitted::new(
        format!(
            "CAST({} AS DECIMAL(9,0))",
            character_edit_distance(&left, &right, site.context.log)
        ),
        SnowflakeType::Number {
            precision: 9,
            scale: 0,
        },
    ))
}

/// `trim`, `ltrim` or `rtrim`; without characters only spaces are removed.
pub(crate) fn trim_call(function: &str, text: &str, characters: Option<&str>) -> String {
    format!("{function}({text}, {})", characters.unwrap_or(SPACE_ONLY))
}

/// UPPER and LOWER with full Unicode case mapping.
pub(crate) fn case_mapping(
    kind: &str,
    payload: &Value,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let log = site.context.log;
    let _rule = log.rule(CASE_MAPPING);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this"], &construct)?;
    let inner = text_operand(field(payload, "this", &construct)?, &construct, site)?;
    let sql = match kind {
        "upper" => full_case_upper(&inner, log),
        _ => full_case_lower(&inner, log),
    };
    Ok(Emitted::new(sql, SnowflakeType::Varchar))
}

/// TRIM, with an optional side and character list.
pub(crate) fn trim(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("trim");
    let _rule = site.context.log.rule(TRIM);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "characters", "position"], &construct)?;
    let function = match payload.get("position").and_then(Value::as_str) {
        Some("Both") | None => "trim",
        Some("Leading") => "ltrim",
        Some("Trailing") => "rtrim",
        Some(other) => {
            return Err(missing(construct, format!("TRIM position {other}")));
        }
    };
    let text = text_operand(field(payload, "this", &construct)?, &construct, site)?;
    let characters = match payload.get("characters").filter(|value| !value.is_null()) {
        Some(characters) => Some(text_operand(characters, &construct, site)?),
        None => None,
    };
    Ok(Emitted::new(
        trim_call(function, &text, characters.as_deref()),
        SnowflakeType::Varchar,
    ))
}

/// `||`.
pub(crate) fn concat_operator(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("concat");
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["left", "right"], &construct)?;
    let parts = [
        field(payload, "left", &construct)?.clone(),
        field(payload, "right", &construct)?.clone(),
    ];
    concatenate(&parts, &construct, site)
}

/// CONCAT_WS: the parts joined by the separator, NULL when the separator or any part is NULL.
pub(crate) fn concatenate_with_separator(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(CONCAT_WS);
    let [separator, first, rest @ ..] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "CONCAT_WS without a part to join".to_owned(),
        ));
    };
    let mut interleaved: Vec<Value> = vec![first.clone()];
    for part in rest {
        interleaved.push(separator.clone());
        interleaved.push(part.clone());
    }
    if rest.is_empty() {
        interleaved.push(separator.clone());
        let texts = operand_texts(&interleaved, construct, site)?;
        return Ok(Emitted::new(
            format!("CAST({} AS VARCHAR)", texts[0]),
            SnowflakeType::Varchar,
        ));
    }
    let texts = operand_texts(&interleaved, construct, site)?;
    Ok(Emitted::new(
        format!(
            "CAST({} AS VARCHAR)",
            null_propagating_concat(&texts, site.context.log)
        ),
        SnowflakeType::Varchar,
    ))
}

/// Text from VARCHAR, NUMBER, BOOLEAN or DATE operands, formatted as Snowflake formats them.
pub(crate) fn concatenate(
    parts: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(CONCAT);
    let texts = operand_texts(parts, construct, site)?;
    Ok(Emitted::new(
        null_propagating_concat(&texts, site.context.log),
        SnowflakeType::Varchar,
    ))
}

/// Each operand as text: VARCHAR as it is, NUMBER, BOOLEAN and DATE cast as Snowflake formats them.
fn operand_texts(
    parts: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Vec<String>, Refusal> {
    let mut texts: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts {
        let emitted = convert_expression(part, site)?;
        let text = match emitted.data_type {
            SnowflakeType::Varchar | SnowflakeType::Null => emitted.sql,
            SnowflakeType::Number { .. } | SnowflakeType::Boolean | SnowflakeType::Date => {
                format!("CAST({} AS VARCHAR)", emitted.sql)
            }
            other => {
                return Err(outside_conditions(
                    construct.clone(),
                    format!("concatenating {other} needs Snowflake's text formatting for it"),
                ));
            }
        };
        texts.push(text);
    }
    Ok(texts)
}
