//! UPPER, LOWER, INITCAP, TRIM, CHR, HEX_ENCODE and concatenation.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::full_case_lower::full_case_lower;
use sqlquack_duckdb::blocks::main::full_case_upper::full_case_upper;
use sqlquack_duckdb::blocks::main::null_propagating_concat::null_propagating_concat;
use sqlquack_duckdb::floats::main::float_to_text::float_to_text;
use sqlquack_duckdb::text::constants::{
    HEX_LOWER_CASE, HEX_UPPER_CASE, INITCAP_DEFAULT_DELIMITERS, INITCAP_TEXT,
};
use sqlquack_duckdb::text::main::character_edit_distance::character_edit_distance;
use sqlquack_duckdb::text::main::code_point_text::code_point_text;
use sqlquack_duckdb::text::main::hex_decoded_text::hex_decoded_text;
use sqlquack_duckdb::text::main::hex_text::hex_text;
use sqlquack_duckdb::text::main::title_case::title_case;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, missing, node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    CASE_MAPPING, CONCAT, CONCAT_WS, EDIT_DISTANCE, TEXT_AFFIX, TEXT_CHR, TEXT_CONTAINS, TEXT_HEX,
    TEXT_HEX_DECODE, TEXT_INITCAP, TEXT_MD5, TRIM,
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

/// MD5(text): the lower-case hex MD5 digest of the text's UTF-8 bytes.
pub(crate) fn text_md5(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_MD5);
    let [text] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "MD5 takes one argument".to_owned(),
        ));
    };
    let text = text_operand(text, construct, site)?;
    Ok(Emitted::new(format!("md5({text})"), SnowflakeType::Varchar))
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

/// Text from VARCHAR, NUMBER, FLOAT, BOOLEAN or DATE operands, formatted as Snowflake formats them.
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

/// Each operand as text: VARCHAR as it is, others as Snowflake's `::VARCHAR` writes them.
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
            SnowflakeType::Float => float_to_text(&emitted.sql, site.context.log),
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

/// INITCAP(s) and INITCAP(s, delimiters) with literal delimiters among space, `_` and `-`.
pub(crate) fn initcap(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_INITCAP);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let (text, delimiters) = match arguments {
        [text] => (text, INITCAP_DEFAULT_DELIMITERS.to_owned()),
        [text, delimiters] => match convert_expression(delimiters, site)?.literal {
            Some(Literal::Text(delimiters))
                if delimiters
                    .chars()
                    .all(|character| INITCAP_DEFAULT_DELIMITERS.contains(character)) =>
            {
                (text, delimiters)
            }
            _ => {
                return Err(refuse(
                    "INITCAP delimiters other than literal space, _ and -",
                ));
            }
        },
        _ => return Err(refuse("INITCAP takes one or two arguments")),
    };
    let text = text_operand(text, construct, site)?;
    diverge(site.context, INITCAP_TEXT, construct)?;
    Ok(Emitted::new(
        title_case(&text, &delimiters, site.context.log),
        SnowflakeType::Varchar,
    ))
}

/// CHR(n) of an integer NUMBER.
pub(crate) fn chr(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_CHR);
    let [code] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "CHR takes one argument".to_owned(),
        ));
    };
    let code = convert_expression(code, site)?;
    match code.data_type {
        SnowflakeType::Number { scale: 0, .. } | SnowflakeType::Null => Ok(Emitted::new(
            code_point_text(&code.sql, site.context.log),
            SnowflakeType::Varchar,
        )),
        other => Err(outside_conditions(
            construct.clone(),
            format!("CHR of {other}"),
        )),
    }
}

/// HEX_ENCODE(s) and HEX_ENCODE(s, case) with a literal case of 0 or 1.
pub(crate) fn hex_encode(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_HEX);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let (text, lower) = match arguments {
        [text] => (text, false),
        [text, case] => match convert_expression(case, site)?.literal {
            Some(Literal::Number(case)) if case == HEX_LOWER_CASE => (text, true),
            Some(Literal::Number(case)) if case == HEX_UPPER_CASE => (text, false),
            _ => return Err(refuse("HEX_ENCODE with a case other than literal 0 or 1")),
        },
        _ => return Err(refuse("HEX_ENCODE takes one or two arguments")),
    };
    let text = text_operand(text, construct, site)?;
    Ok(Emitted::new(
        hex_text(&text, lower, site.context.log),
        SnowflakeType::Varchar,
    ))
}

/// TRY_HEX_DECODE_STRING(s): the text its hexadecimal digits spell, or NULL.
pub(crate) fn try_hex_decode(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_HEX_DECODE);
    let [text] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "TRY_HEX_DECODE_STRING takes one argument".to_owned(),
        ));
    };
    let text = text_operand(text, construct, site)?;
    Ok(Emitted::new(
        hex_decoded_text(&text, site.context.log),
        SnowflakeType::Varchar,
    ))
}
