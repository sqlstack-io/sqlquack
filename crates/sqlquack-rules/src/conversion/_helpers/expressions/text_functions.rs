//! LEFT, RIGHT, SUBSTR, LENGTH, REPLACE, TRANSLATE, SPLIT_PART, LIKE and the REGEXP functions.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::strings::text_operand;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, flag, node_construct, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    LENGTH_TYPE, LIKE, POSIX_SPACE_CLASS, REGEXP, REGEXP_PARAMETERS, REGEXP_POSIX_SPACE,
    REGEXP_SHORTHANDS, TEXT_PAD, TEXT_POSITION, TEXT_REPLACE, TEXT_TRANSLATE,
};

/// LPAD or RPAD: padded or cut to the length; an empty pad only cuts, as Snowflake documents.
pub(crate) fn pad(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_PAD);
    let (text, length, filler) = match arguments {
        [text, length] => (text, length, None),
        [text, length, filler] => (text, length, Some(filler)),
        _ => {
            return Err(outside_conditions(
                construct.clone(),
                format!("{name} with these arguments"),
            ));
        }
    };
    let text = text_operand(text, construct, site)?;
    let length = format!(
        "CAST({} AS INTEGER)",
        integer_operand(length, construct, site)?
    );
    let filler = match filler {
        Some(filler) => text_operand(filler, construct, site)?,
        None => "' '".to_owned(),
    };
    let function = name.to_ascii_lowercase();
    Ok(Emitted::new(
        format!(
            "(CASE WHEN {length} <= 0 THEN (CASE WHEN {text} IS NULL OR {filler} IS NULL THEN NULL \
             ELSE '' END) WHEN {filler} = '' THEN left({text}, {length}) \
             ELSE {function}({text}, {length}, {filler}) END)"
        ),
        SnowflakeType::Varchar,
    ))
}

/// LEFT or RIGHT; a negative length gives the empty string, as in Snowflake.
pub(crate) fn left_right(
    name: &str,
    (text, length): (&Value, &Value),
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_POSITION);
    let text = text_operand(text, construct, site)?;
    let length = integer_operand(length, construct, site)?;
    Ok(Emitted::new(
        format!(
            "{}({text}, (CASE WHEN {length} < 0 THEN 0 ELSE {length} END))",
            name.to_ascii_lowercase()
        ),
        SnowflakeType::Varchar,
    ))
}

/// SUBSTR or SUBSTRING; start 0 counts as 1, and a length must be a non-negative literal.
pub(crate) fn substring(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("substring");
    let _rule = site.context.log.rule(TEXT_POSITION);
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &["this", "start", "length", "from_for_syntax"],
        &construct,
    )?;
    let text = text_operand(field(payload, "this", &construct)?, &construct, site)?;
    let start = integer_operand(field(payload, "start", &construct)?, &construct, site)?;
    let start = format!("(CASE WHEN {start} = 0 THEN 1 ELSE {start} END)");
    let length = match payload.get("length").filter(|length| !length.is_null()) {
        None => String::new(),
        Some(length) => {
            let emitted = convert_expression(length, site)?;
            match &emitted.literal {
                Some(Literal::Number(digits))
                    if digits.bytes().all(|byte| byte.is_ascii_digit()) =>
                {
                    format!(", {digits}")
                }
                _ => {
                    return Err(outside_conditions(
                        construct,
                        "a SUBSTR length other than a non-negative integer literal".to_owned(),
                    ));
                }
            }
        }
    };
    Ok(Emitted::new(
        format!("substring({text}, {start}{length})"),
        SnowflakeType::Varchar,
    ))
}

/// LENGTH in characters, typed NUMBER(18,0).
pub(crate) fn length(payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct("length");
    let _rule = site.context.log.rule(TEXT_POSITION);
    let payload = payload_object(payload, &construct)?;
    require_understood(payload, &["this", "original_name"], &construct)?;
    let text = text_operand(field(payload, "this", &construct)?, &construct, site)?;
    Ok(Emitted::new(
        format!("CAST(length({text}) AS DECIMAL(18,0))"),
        LENGTH_TYPE,
    ))
}

/// REPLACE; without a replacement the matches are removed, and an empty search changes nothing.
pub(crate) fn replace(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_REPLACE);
    let mut texts: Vec<String> = Vec::with_capacity(arguments.len());
    for argument in arguments {
        texts.push(text_operand(argument, construct, site)?);
    }
    let replacement = texts.get(2).cloned().unwrap_or_else(|| "''".to_owned());
    Ok(Emitted::new(
        format!("replace({}, {}, {replacement})", texts[0], texts[1]),
        SnowflakeType::Varchar,
    ))
}

/// TRANSLATE with literal characters: each `from` character distinct, `to` no longer than `from`.
pub(crate) fn translate(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_TRANSLATE);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let [text, from, to] = arguments else {
        return Err(refuse("TRANSLATE takes three arguments"));
    };
    let text = text_operand(text, construct, site)?;
    let from = text_literal(from, construct, site)?;
    let to = text_literal(to, construct, site)?;
    let characters: Vec<char> = from.chars().collect();
    if characters.is_empty() {
        return Err(refuse("TRANSLATE from an empty character list"));
    }
    if (1..characters.len()).any(|position| characters[..position].contains(&characters[position]))
    {
        return Err(refuse("TRANSLATE from a list repeating a character"));
    }
    if to.chars().count() > characters.len() {
        return Err(refuse("TRANSLATE to a list longer than its source"));
    }
    let log = site.context.log;
    Ok(Emitted::new(
        format!(
            "translate({text}, {}, {})",
            string_literal(&from, log),
            string_literal(&to, log)
        ),
        SnowflakeType::Varchar,
    ))
}

/// SPLIT_PART with a literal delimiter; part 0 counts as 1, and an empty delimiter keeps the text.
pub(crate) fn split_part(
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(TEXT_REPLACE);
    let [text, delimiter, part] = arguments else {
        return Err(outside_conditions(
            construct.clone(),
            "SPLIT_PART takes three arguments".to_owned(),
        ));
    };
    let text = text_operand(text, construct, site)?;
    let delimiter = text_literal(delimiter, construct, site)?;
    if delimiter.is_empty() {
        return match convert_expression(part, site)?.literal {
            Some(Literal::Number(number))
                if positive_integer(&number) && number.parse() == Ok(1_u8) =>
            {
                Ok(Emitted::new(text, SnowflakeType::Varchar))
            }
            _ => Err(outside_conditions(
                construct.clone(),
                "an empty SPLIT_PART delimiter with a part other than 1".to_owned(),
            )),
        };
    }
    let part = integer_operand(part, construct, site)?;
    Ok(Emitted::new(
        format!(
            "split_part({text}, {}, (CASE WHEN {part} = 0 THEN 1 ELSE {part} END))",
            string_literal(&delimiter, site.context.log)
        ),
        SnowflakeType::Varchar,
    ))
}

/// LIKE, NOT LIKE and ILIKE, with an optional ESCAPE; ILIKE needs an ASCII literal pattern.
pub(crate) fn like(kind: &str, payload: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let construct = node_construct(kind);
    let _rule = site.context.log.rule(LIKE);
    let payload = payload_object(payload, &construct)?;
    require_understood(
        payload,
        &[
            "negated",
            "left",
            "right",
            "escape",
            "quantifier",
            "inferred_type",
        ],
        &construct,
    )?;
    if payload
        .get("quantifier")
        .is_some_and(|value| !value.is_null())
    {
        return Err(outside_conditions(
            construct,
            "LIKE ANY and LIKE ALL".to_owned(),
        ));
    }
    let text = text_operand(field(payload, "left", &construct)?, &construct, site)?;
    let pattern_node = field(payload, "right", &construct)?;
    let pattern = text_operand(pattern_node, &construct, site)?;
    let operator = match kind {
        "i_like" => {
            let literal = text_literal(pattern_node, &construct, site)?;
            if !literal.is_ascii() {
                return Err(outside_conditions(
                    construct,
                    "ILIKE with a pattern outside ASCII, whose case folding is unproven".to_owned(),
                ));
            }
            "ILIKE"
        }
        _ => "LIKE",
    };
    let negation = match flag(payload, "negated") {
        true => "NOT ",
        false => "",
    };
    let escape = match payload.get("escape").filter(|escape| !escape.is_null()) {
        Some(escape) => format!(" ESCAPE {}", text_operand(escape, &construct, site)?),
        None => String::new(),
    };
    Ok(Emitted::new(
        format!("({text} {negation}{operator} {pattern}{escape})"),
        SnowflakeType::Boolean,
    ))
}

/// REGEXP_LIKE, REGEXP_REPLACE and REGEXP_SUBSTR with a literal pattern in the shared syntax.
pub(crate) fn regexp(
    name: &str,
    arguments: &[Value],
    construct: &Construct,
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(REGEXP);
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let [subject, pattern, rest @ ..] = arguments else {
        return Err(refuse("a REGEXP function without a pattern"));
    };
    let text = text_operand(subject, construct, site)?;
    let pattern_text = text_literal(pattern, construct, site)?;
    let log = site.context.log;
    let _posix = shared_pattern(&pattern_text)
        .map_err(|reason| refuse(&reason))?
        .then(|| log.rule(REGEXP_POSIX_SPACE));
    let pattern = string_literal(&pattern_text, log);
    let mut literals: Vec<String> = Vec::with_capacity(rest.len());
    for argument in rest {
        let emitted = convert_expression(argument, site)?;
        match emitted.literal {
            Some(Literal::Text(text) | Literal::Number(text)) => literals.push(text),
            None => return Err(refuse("a REGEXP argument other than a literal")),
        }
    }
    let literals: Vec<&str> = literals.iter().map(String::as_str).collect();
    let (sql, data_type) = match (name, literals.as_slice()) {
        ("REGEXP_LIKE", []) => (
            format!("regexp_full_match({text}, {pattern})"),
            SnowflakeType::Boolean,
        ),
        ("REGEXP_LIKE", [parameters]) => {
            let _parameters = log.rule(REGEXP_PARAMETERS);
            let flags = inline_flags(parameters)
                .ok_or_else(|| refuse("REGEXP_LIKE parameters other than c, i and s"))?;
            (
                format!(
                    "regexp_full_match({text}, {})",
                    string_literal(&format!("{flags}{pattern_text}"), log)
                ),
                SnowflakeType::Boolean,
            )
        }
        ("REGEXP_REPLACE", [replacement, options @ ..]) => {
            shared_replacement(replacement).map_err(|reason| refuse(&reason))?;
            let flags = match options {
                [] | ["1"] | ["1", "0"] | ["1", "0", "c"] => "g",
                ["1", "0", "i"] => "gi",
                _ => {
                    return Err(refuse(
                        "REGEXP_REPLACE from a position or of one occurrence",
                    ));
                }
            };
            (
                format!(
                    "regexp_replace({text}, {pattern}, {}, '{flags}')",
                    string_literal(replacement, log)
                ),
                SnowflakeType::Varchar,
            )
        }
        ("REGEXP_REPLACE", []) => (
            format!("regexp_replace({text}, {pattern}, '', 'g')"),
            SnowflakeType::Varchar,
        ),
        ("REGEXP_SUBSTR", options) => {
            let (occurrence, group) = match options {
                [] | ["1"] => ("1", "0"),
                ["1", occurrence] | ["1", occurrence, "c"] => (*occurrence, "0"),
                ["1", occurrence, "e"] | ["1", occurrence, "ce"] => (*occurrence, "1"),
                ["1", occurrence, "e" | "ce", group] => (*occurrence, *group),
                _ => return Err(refuse("REGEXP_SUBSTR with these parameters")),
            };
            if !positive_integer(occurrence) || !group.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(refuse(
                    "REGEXP_SUBSTR with a non-literal occurrence or group",
                ));
            }
            (
                format!("regexp_extract_all({text}, {pattern}, {group})[{occurrence}]"),
                SnowflakeType::Varchar,
            )
        }
        _ => return Err(refuse("a REGEXP function with these arguments")),
    };
    Ok(Emitted::new(sql, data_type))
}

/// RE2 inline flags for Snowflake's c, i and s parameters, the last of c and i winning.
fn inline_flags(parameters: &str) -> Option<String> {
    let mut insensitive = false;
    let mut dot_all = false;
    for parameter in parameters.chars() {
        match parameter {
            'c' => insensitive = false,
            'i' => insensitive = true,
            's' => dot_all = true,
            _ => return None,
        }
    }
    let flags: String = [(insensitive, 'i'), (dot_all, 's')]
        .iter()
        .filter(|(set, _)| *set)
        .map(|(_, flag)| *flag)
        .collect();
    Some(match flags.is_empty() {
        true => String::new(),
        false => format!("(?{flags})"),
    })
}

fn positive_integer(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && !text.trim_start_matches('0').is_empty()
}

/// A NUMBER operand of scale 0, as a BIGINT expression.
fn integer_operand(node: &Value, construct: &Construct, site: Site<'_>) -> Result<String, Refusal> {
    let emitted = convert_expression(node, site)?;
    match emitted.data_type {
        SnowflakeType::Number { scale: 0, .. } | SnowflakeType::Null => {
            Ok(format!("CAST({} AS BIGINT)", emitted.sql))
        }
        other => Err(outside_conditions(
            construct.clone(),
            format!("a {other} position or length needs an implicit conversion"),
        )),
    }
}

/// The value of a string literal argument.
fn text_literal(node: &Value, construct: &Construct, site: Site<'_>) -> Result<String, Refusal> {
    match convert_expression(node, site)?.literal {
        Some(Literal::Text(text)) => Ok(text),
        _ => Err(outside_conditions(
            construct.clone(),
            "an argument that must be a string literal".to_owned(),
        )),
    }
}

/// Checks a pattern uses syntax Snowflake and RE2 read alike; true when it uses `[:space:]`.
fn shared_pattern(pattern: &str) -> Result<bool, String> {
    let mut characters = pattern.chars().peekable();
    let mut previous_quantifier = false;
    let mut in_bracket = false;
    let mut posix = false;
    while let Some(character) = characters.next() {
        let quantifier = matches!(character, '*' | '+' | '?' | '}');
        match character {
            '\\' => match characters.next() {
                Some(escaped) if REGEXP_SHORTHANDS.contains(escaped) => {}
                Some(escaped) if escaped.is_ascii_punctuation() => {}
                Some(escaped) => return Err(format!("the regular expression escape \\{escaped}")),
                None => return Err("a pattern ending in a backslash".to_owned()),
            },
            '?' if previous_quantifier => {
                return Err("a lazy quantifier".to_owned());
            }
            '(' if characters.peek() == Some(&'?') => {
                return Err("a group modifier such as a lookaround".to_owned());
            }
            '[' if characters.peek() == Some(&':') => {
                let class: String = characters.by_ref().take(POSIX_SPACE_CLASS.len()).collect();
                if !in_bracket || class != POSIX_SPACE_CLASS {
                    return Err("a POSIX character class other than [:space:]".to_owned());
                }
                posix = true;
            }
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ => {}
        }
        previous_quantifier = quantifier;
    }
    Ok(posix)
}

/// Checks that a replacement uses backslashes only for group references.
fn shared_replacement(replacement: &str) -> Result<(), String> {
    let mut characters = replacement.chars();
    while let Some(character) = characters.next() {
        if character == '\\'
            && !characters
                .next()
                .is_some_and(|digit| digit.is_ascii_digit())
        {
            return Err("a backslash in the replacement other than a group reference".to_owned());
        }
    }
    Ok(())
}
