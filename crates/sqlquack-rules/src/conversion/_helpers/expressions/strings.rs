//! UPPER, LOWER, INITCAP, TRIM, CHR, HEX_ENCODE and concatenation.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::failures::constants::HEX_DECODE_FAILED;
use sqlquack_core::failures::main::tagged_message::tagged_message;
use sqlquack_core::refusals::main::missing::missing;
use sqlquack_core::refusals::main::outside_conditions::outside_conditions;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb_blocks::blocks::main::full_case_lower::full_case_lower;
use sqlquack_duckdb_blocks::blocks::main::full_case_upper::full_case_upper;
use sqlquack_duckdb_blocks::blocks::main::null_propagating_concat::null_propagating_concat;
use sqlquack_duckdb_blocks::blocks::main::string_literal::string_literal;
use sqlquack_duckdb_blocks::floats::main::float_to_text::float_to_text;
use sqlquack_duckdb_blocks::raises::main::raised::raised;
use sqlquack_duckdb_blocks::semi_structured::constants::JSON_FIDELITY;
use sqlquack_duckdb_blocks::semi_structured::main::json_text::json_text;
use sqlquack_duckdb_blocks::text::constants::TEXT_SIMILARITY;
use sqlquack_duckdb_blocks::text::constants::{
    HEX_LOWER_CASE, HEX_UPPER_CASE, INITCAP_DEFAULT_DELIMITERS, INITCAP_TEXT,
};
use sqlquack_duckdb_blocks::text::main::base64_bytes::base64_bytes;
use sqlquack_duckdb_blocks::text::main::bytes_substring::bytes_substring;
use sqlquack_duckdb_blocks::text::main::bytes_text::bytes_text;
use sqlquack_duckdb_blocks::text::main::character_edit_distance::character_edit_distance;
use sqlquack_duckdb_blocks::text::main::character_similarity::character_similarity;
use sqlquack_duckdb_blocks::text::main::code_point_text::code_point_text;
use sqlquack_duckdb_blocks::text::main::hex_decoded_text::hex_decoded_text;
use sqlquack_duckdb_blocks::text::main::hex_text::hex_text;
use sqlquack_duckdb_blocks::text::main::title_case::title_case;
use sqlquack_snowflake_types::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::text_functions::integer_operand;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, function_construct, integer_literal, list, node_construct, node_kind, payload_object,
    require_understood,
};
use crate::conversion::constants::{
    BASE64_BINARY, BASE64_DECODER, CASE_MAPPING, CONCAT, CONCAT_WS, EDIT_DISTANCE,
    HEX_DECODE_MESSAGE, JARO_WINKLER, SHA2_DEFAULT_BITS, TEXT_AFFIX, TEXT_CHR, TEXT_CONTAINS,
    TEXT_HEX, TEXT_HEX_DECODE, TEXT_INITCAP, TEXT_MD5, TEXT_POSITION_LIMIT, TEXT_REPEAT, TEXT_SHA2,
    TRIM, TRYING_HEX_DECODE, VARIANT_TO_VARCHAR,
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
        SnowflakeType::Date => Ok(format!("CAST({} AS VARCHAR)", emitted.sql)),
        SnowflakeType::SemiStructured if !emitted.is_variant_held() => {
            variant_as_text(&emitted, site)
        }
        other => Err(outside_conditions(
            construct.clone(),
            format!("a {other} operand needs an implicit conversion to text"),
        )),
    }
}

/// A JSON-held VARIANT where text is expected, read as `::VARCHAR` reads it.
fn variant_as_text(emitted: &Emitted, site: Site<'_>) -> Result<String, Refusal> {
    let _rule = site.context.log.rule(VARIANT_TO_VARCHAR);
    diverge(site.context, JSON_FIDELITY, &node_construct("cast"))?;
    Ok(json_text(&emitted.sql, site.context.log))
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

/// JAROWINKLER_SIMILARITY(a, b): Jaro-Winkler similarity of the characters ignoring case, 0 to 100.
pub(crate) fn jaro_winkler(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(JARO_WINKLER);
    let [left, right] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "JAROWINKLER_SIMILARITY takes two arguments".to_owned(),
        ));
    };
    let left = text_operand(left, construct, site)?;
    let right = text_operand(right, construct, site)?;
    diverge(site.context, TEXT_SIMILARITY, construct)?;
    Ok(Emitted::new(
        character_similarity(&left, &right, site.context.log),
        SnowflakeType::Number {
            precision: 4,
            scale: 0,
        },
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
        field(payload, "left", &construct)?,
        field(payload, "right", &construct)?,
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
    parts: &[&Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(CONCAT);
    let emitted = parts
        .iter()
        .map(|part| convert_expression(part, site))
        .collect::<Result<Vec<Emitted>, Refusal>>()?;
    let mut folded = String::new();
    for part in &emitted {
        match &part.literal {
            Some(Literal::Text(text)) => folded.push_str(text),
            _ => {
                let mut texts: Vec<String> = Vec::with_capacity(emitted.len());
                for part in &emitted {
                    texts.push(emitted_text(part, construct, site)?);
                }
                return Ok(Emitted::new(
                    null_propagating_concat(&texts, site.context.log),
                    SnowflakeType::Varchar,
                ));
            }
        }
    }
    Ok(text_constant(folded, site))
}

/// A constant text, carried as a literal so arguments that must be literals accept it.
fn text_constant(text: String, site: Site<'_>) -> Emitted {
    Emitted {
        literal: Some(Literal::Text(text.clone())),
        ..Emitted::new(
            string_literal(&text, site.context.log),
            SnowflakeType::Varchar,
        )
    }
}

/// Each operand as text: VARCHAR as it is, others as Snowflake's `::VARCHAR` writes them.
fn operand_texts(
    parts: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Vec<String>, Refusal> {
    let mut texts: Vec<String> = Vec::with_capacity(parts.len());
    for part in parts {
        texts.push(emitted_text(
            &convert_expression(part, site)?,
            construct,
            site,
        )?);
    }
    Ok(texts)
}

/// An operand as text: VARCHAR as it is, others as Snowflake's `::VARCHAR` writes them.
fn emitted_text(
    emitted: &Emitted,
    construct: &Construct,
    site: Site<'_>,
) -> Result<String, Refusal> {
    match emitted.data_type {
        SnowflakeType::Varchar | SnowflakeType::Null => Ok(emitted.sql.clone()),
        SnowflakeType::Number { .. } | SnowflakeType::Boolean | SnowflakeType::Date => {
            Ok(format!("CAST({} AS VARCHAR)", emitted.sql))
        }
        SnowflakeType::Float => Ok(float_to_text(&emitted.sql, site.context.log)),
        SnowflakeType::SemiStructured if !emitted.is_variant_held() => {
            variant_as_text(emitted, site)
        }
        other => Err(outside_conditions(
            construct.clone(),
            format!("concatenating {other} needs Snowflake's text formatting for it"),
        )),
    }
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
    if let Some(Literal::Number(digits)) = &code.literal
        && let Ok(point) = digits.parse::<u32>()
        && point > 0
        && let Some(character) = char::from_u32(point)
    {
        return Ok(text_constant(character.to_string(), site));
    }
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

/// REPEAT of text by a whole count: empty for a count below one.
pub(crate) fn repeat(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_REPEAT);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let [text, count] = arguments else {
        return Err(refuse("REPEAT takes two arguments".to_owned()));
    };
    let text = convert_expression(text, site)?;
    let count = convert_expression(count, site)?;
    match (text.data_type, count.data_type) {
        (
            SnowflakeType::Varchar | SnowflakeType::Null,
            SnowflakeType::Number { scale: 0, .. } | SnowflakeType::Null,
        ) => Ok(Emitted::new(
            format!(
                "repeat(CAST({} AS VARCHAR), CAST({} AS BIGINT))",
                text.sql, count.sql
            ),
            SnowflakeType::Varchar,
        )),
        (text, count) => Err(refuse(format!("REPEAT of {text} by {count}"))),
    }
}

/// HEX_ENCODE(s) and HEX_ENCODE(s, case) with a literal case of 0 or 1.
pub(crate) fn hex_encode(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let rule = match arguments.first().is_some_and(decodes_bytes) {
        true => BASE64_BINARY,
        false => TEXT_HEX,
    };
    let _rule = site.context.log.rule(rule);
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
    if let Some(bytes) = binary_operand(text, site)? {
        let sql = match lower {
            true => format!("lower({bytes})"),
            false => bytes,
        };
        return Ok(Emitted::new(sql, SnowflakeType::Varchar));
    }
    let text = text_operand(text, construct, site)?;
    Ok(Emitted::new(
        hex_text(&text, lower, site.context.log),
        SnowflakeType::Varchar,
    ))
}

/// TO_VARCHAR or TO_CHAR of decoded bytes with a literal UTF-8 format, or None for other values.
pub(crate) fn bytes_as_text(
    (value, format): (&Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Option<Emitted>, Refusal> {
    if !decodes_bytes(value) {
        return Ok(None);
    }
    let _rule = site.context.log.rule(BASE64_BINARY);
    let Some(bytes) = binary_operand(value, site)? else {
        return Ok(None);
    };
    match convert_expression(format, site)?.literal {
        Some(Literal::Text(format))
            if format.eq_ignore_ascii_case("UTF-8") || format.eq_ignore_ascii_case("UTF8") =>
        {
            Ok(Some(Emitted::new(
                bytes_text(&bytes, site.context.log),
                SnowflakeType::Varchar,
            )))
        }
        _ => Err(outside_conditions(
            construct.clone(),
            "decoded bytes written as text in a format other than a literal UTF-8".to_owned(),
        )),
    }
}

/// Whether `node` is TRY_BASE64_DECODE_BINARY, or SUBSTR of it, whose bytes only some calls read.
pub(crate) fn decodes_bytes(node: &Value) -> bool {
    match node_kind(node) {
        Some(("function", payload)) => {
            let name = payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_uppercase();
            match (name.as_str(), payload.get("args").and_then(Value::as_array)) {
                (BASE64_DECODER, _) => true,
                ("SUBSTR" | "SUBSTRING", Some(arguments)) => {
                    arguments.first().is_some_and(decodes_bytes)
                }
                _ => false,
            }
        }
        Some(("substring", payload)) => payload.get("this").is_some_and(decodes_bytes),
        _ => false,
    }
}

/// Decoded bytes as upper-case hex digits, from TRY_BASE64_DECODE_BINARY or SUBSTR; else None.
fn binary_operand(node: &Value, site: Site<'_>) -> Result<Option<String>, Refusal> {
    match node_kind(node) {
        Some(("function", payload)) => {
            let name = payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_uppercase();
            let construct = function_construct(&name);
            let payload = payload_object(payload, &construct)?;
            match (name.as_str(), list(payload, "args")) {
                (BASE64_DECODER, arguments) => {
                    let _rule = site.context.log.rule(BASE64_BINARY);
                    require_understood(payload, &["name", "args"], &construct)?;
                    let [text] = arguments else {
                        return Err(outside_conditions(
                            construct,
                            "TRY_BASE64_DECODE_BINARY with other than one argument".to_owned(),
                        ));
                    };
                    let text = text_operand(text, &construct, site)?;
                    Ok(Some(base64_bytes(&text, site.context.log)))
                }
                ("SUBSTR" | "SUBSTRING", [bytes, start, length @ ..]) if length.len() <= 1 => {
                    bytes_substring_of((bytes, start, length.first()), site)
                }
                _ => Ok(None),
            }
        }
        Some(("substring", payload)) => {
            let construct = node_construct("substring");
            let payload = payload_object(payload, &construct)?;
            require_understood(
                payload,
                &["this", "start", "length", "from_for_syntax"],
                &construct,
            )?;
            let length = payload.get("length").filter(|length| !length.is_null());
            bytes_substring_of(
                (
                    field(payload, "this", &construct)?,
                    field(payload, "start", &construct)?,
                    length,
                ),
                site,
            )
        }
        _ => Ok(None),
    }
}

/// SUBSTR of decoded bytes, counted in bytes; None when `bytes` is not decoded bytes.
fn bytes_substring_of(
    (bytes, start, length): (&Value, &Value, Option<&Value>),
    site: Site<'_>,
) -> Result<Option<String>, Refusal> {
    let Some(bytes) = binary_operand(bytes, site)? else {
        return Ok(None);
    };
    let construct = node_construct("substring");
    let start = integer_operand(start, &construct, site)?;
    let start = format!(
        "(CASE WHEN {start} < -{TEXT_POSITION_LIMIT} THEN -{TEXT_POSITION_LIMIT} \
         WHEN {start} > {TEXT_POSITION_LIMIT} THEN {TEXT_POSITION_LIMIT} ELSE {start} END)"
    );
    let length = match length {
        None => None,
        Some(length) => match convert_expression(length, site)?.literal {
            Some(Literal::Number(digits)) if digits.bytes().all(|byte| byte.is_ascii_digit()) => {
                Some(digits)
            }
            _ => {
                return Err(outside_conditions(
                    construct,
                    "a SUBSTR length other than a non-negative integer literal".to_owned(),
                ));
            }
        },
    };
    Ok(Some(bytes_substring(
        &bytes,
        (&start, length.as_deref()),
        site.context.log,
    )))
}

/// TRY_HEX_DECODE_STRING(s): the text its hexadecimal digits spell, or NULL.
pub(crate) fn try_hex_decode(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_HEX_DECODE);
    let [text] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            format!("{name} takes one argument"),
        ));
    };
    let text = text_operand(text, construct, site)?;
    let decoded = hex_decoded_text(&text, site.context.log);
    let sql = match name {
        TRYING_HEX_DECODE => decoded,
        _ => strict_hex_decoded(&text, &decoded),
    };
    Ok(Emitted::new(sql, SnowflakeType::Varchar))
}

/// HEX_DECODE_STRING: the decoded text, failing with error 100115 where TRY_ gives NULL.
fn strict_hex_decoded(text: &str, decoded: &str) -> String {
    format!(
        "list_transform([struct_pack(t := {text}, d := {decoded})], lambda h: CASE \
         WHEN h['t'] IS NOT NULL AND h['d'] IS NULL THEN CAST({} AS VARCHAR) ELSE h['d'] END)[1]",
        raised(&tagged_message(HEX_DECODE_FAILED, HEX_DECODE_MESSAGE))
    )
}

/// SHA2 of text with 256 bits, the default: 64 lower-case hex digits of its UTF-8 bytes' digest.
pub(crate) fn text_sha2(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_SHA2);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let text = match arguments {
        [text] => text,
        [text, bits] if integer_literal(bits) == Some(SHA2_DEFAULT_BITS) => text,
        _ => return Err(refuse("SHA2 with a digest size other than 256 bits")),
    };
    let text = text_operand(text, construct, site)?;
    Ok(Emitted::new(
        format!("sha256({text})"),
        SnowflakeType::Varchar,
    ))
}
