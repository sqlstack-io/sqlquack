//! LEFT, RIGHT, SUBSTR, LENGTH, REPLACE, TRANSLATE, SPLIT_PART, LIKE and the REGEXP functions.

use serde_json::Value;
use sqlquack_core::classification::models::Construct;
use sqlquack_core::refusals::models::Refusal;
use sqlquack_duckdb::blocks::main::string_literal::string_literal;
use sqlquack_duckdb::text::constants::REGEXP_PATTERN_SYNTAX;
use sqlquack_duckdb::text::main::guarded_pattern::guarded_pattern;
use sqlquack_snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::scalar::convert_expression;
use crate::conversion::_helpers::expressions::strings::text_operand;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Emitted, Literal};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, flag, node_construct, node_kind, outside_conditions, payload_object, require_understood,
};
use crate::conversion::constants::{
    ILIKE_FOLDING, INFIX_NEGATION, LENGTH_TYPE, LIKE, LIKE_WILDCARDS, NEGATED_FACET, NO_QUANTIFIER,
    POSIX_SPACE_CLASS, QUANTIFIER_FACET, REGEXP, REGEXP_EXTRACTING_FUNCTIONS,
    REGEXP_LINE_FEED_ESCAPE, REGEXP_PARAMETERS, REGEXP_POSIX_SPACE, REGEXP_SHORTHANDS, TEXT_PAD,
    TEXT_POSITION, TEXT_POSITION_LIMIT, TEXT_REPLACE, TEXT_TRANSLATE,
};

/// The quantifier LIKE ANY is written with.
const LIKE_ANY: &str = "ANY";
/// The node kind of a parenthesised list.
const TUPLE_KIND: &str = "tuple";
/// The node kind of ILIKE, the one LIKE recorded with ANY.
const ILIKE_KIND: &str = "i_like";

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
            "{}({text}, (CASE WHEN {length} < 0 THEN 0 WHEN {length} > {TEXT_POSITION_LIMIT} \
             THEN {TEXT_POSITION_LIMIT} ELSE {length} END))",
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
        format!(
            "list_transform([struct_pack(t := {text}, s := (CASE WHEN {start} < \
             -{TEXT_POSITION_LIMIT} THEN -{TEXT_POSITION_LIMIT} WHEN {start} > {TEXT_POSITION_LIMIT} \
             THEN {TEXT_POSITION_LIMIT} ELSE {start} END))], lambda p: CASE WHEN p['s'] < 0 \
             AND -p['s'] > length(p['t']) THEN '' ELSE substring(p['t'], (CASE WHEN p['s'] = 0 THEN 1 ELSE p['s'] END){length}) END)[1]"
        ),
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

/// LIKE, NOT LIKE and ILIKE, with an optional one-character literal ESCAPE over literal patterns.
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
    let right = field(payload, "right", &construct)?;
    let patterns: Vec<&Value> = match payload.get("quantifier").and_then(Value::as_str) {
        None => vec![right],
        Some(LIKE_ANY) if kind != ILIKE_KIND || flag(payload, "negated") => {
            return Err(outside_conditions(
                construct,
                "LIKE ANY, and NOT ILIKE ANY written infix, which no recording shows".to_owned(),
            ));
        }
        Some(LIKE_ANY) if payload.get("escape").is_none_or(Value::is_null) => {
            match node_kind(right) {
                Some((TUPLE_KIND, tuple)) => tuple
                    .get("expressions")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().collect())
                    .unwrap_or_default(),
                _ => vec![right],
            }
        }
        Some(_) => {
            return Err(outside_conditions(
                construct,
                "LIKE ALL, or LIKE ANY with an escape".to_owned(),
            ));
        }
    };
    let log = site.context.log;
    log.facet(
        QUANTIFIER_FACET,
        &payload
            .get("quantifier")
            .and_then(Value::as_str)
            .unwrap_or(NO_QUANTIFIER)
            .to_ascii_lowercase(),
    );
    if flag(payload, "negated") {
        log.facet(NEGATED_FACET, INFIX_NEGATION);
    }
    if patterns.is_empty() {
        return Err(outside_conditions(
            construct,
            "LIKE ANY of no patterns".to_owned(),
        ));
    }
    let text = text_operand(field(payload, "left", &construct)?, &construct, site)?;
    let escape_character = match payload.get("escape").filter(|escape| !escape.is_null()) {
        Some(escape) => Some(like_escape(escape, &construct, site)?),
        None => None,
    };
    let escape = match escape_character {
        Some(character) => format!(" ESCAPE {}", string_literal(&character.to_string(), log)),
        None => String::new(),
    };
    let mut matches: Vec<String> = Vec::with_capacity(patterns.len());
    for pattern_node in patterns {
        if let Some(character) = escape_character {
            escaped_pattern(
                &text_literal(pattern_node, &construct, site)?,
                character,
                &construct,
            )?;
        }
        let pattern = text_operand(pattern_node, &construct, site)?;
        let matched = match kind {
            "i_like" if !text_literal(pattern_node, &construct, site)?.is_ascii() => {
                let _folding = site.context.log.rule(ILIKE_FOLDING);
                format!("(lower({text}) LIKE lower({pattern}){escape})")
            }
            "i_like" => format!("({text} ILIKE {pattern}{escape})"),
            _ => format!("({text} LIKE {pattern}{escape})"),
        };
        matches.push(matched);
    }
    let negation = match flag(payload, "negated") {
        true => "NOT ",
        false => "",
    };
    Ok(Emitted::new(
        format!("({negation}({}))", matches.join(" OR ")),
        SnowflakeType::Boolean,
    ))
}

/// A LIKE escape: one literal character other than a wildcard, as Snowflake accepts.
fn like_escape(node: &Value, construct: &Construct, site: Site<'_>) -> Result<char, Refusal> {
    let text = text_literal(node, construct, site)?;
    let mut characters = text.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) if !LIKE_WILDCARDS.contains(&character) => Ok(character),
        _ => Err(outside_conditions(
            construct.clone(),
            format!("the LIKE escape '{text}', which is not one character other than a wildcard"),
        )),
    }
}

/// Refuses a pattern whose escape character precedes anything but a wildcard or itself.
fn escaped_pattern(pattern: &str, escape: char, construct: &Construct) -> Result<(), Refusal> {
    let mut characters = pattern.chars();
    while let Some(character) = characters.next() {
        if character != escape {
            continue;
        }
        match characters.next() {
            Some(next) if next == escape || LIKE_WILDCARDS.contains(&next) => {}
            _ => {
                return Err(outside_conditions(
                    construct.clone(),
                    "a LIKE pattern whose escape character precedes neither a wildcard nor \
                     itself"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
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
    let pattern = convert_expression(pattern, site)?;
    let pattern_text = match (&pattern.literal, name) {
        (Some(Literal::Text(text)), _) => text.clone(),
        (_, "REGEXP_LIKE") if pattern.data_type == SnowflakeType::Varchar => {
            return regexp_like_column(&text, &pattern.sql, rest, (construct, site));
        }
        _ => return Err(refuse("an argument that must be a string literal")),
    };
    let log = site.context.log;
    let extracting = REGEXP_EXTRACTING_FUNCTIONS.contains(&name);
    let _posix = shared_pattern(&pattern_text, extracting)
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
                ["1", "1"] | ["1", "1", "c"] => "",
                ["1", "1", "i"] => "i",
                _ => {
                    return Err(refuse(
                        "REGEXP_REPLACE from a position other than 1 or of an occurrence other than 1",
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
            let (occurrence, parameters, group) = match options {
                [] | ["1"] => ("1", "", None),
                ["1", occurrence] => (*occurrence, "", None),
                ["1", occurrence, parameters] => (*occurrence, *parameters, None),
                ["1", occurrence, parameters, group] if parameters.contains('e') => {
                    (*occurrence, *parameters, Some(*group))
                }
                _ => return Err(refuse("REGEXP_SUBSTR with these parameters")),
            };
            let extract = parameters.contains('e');
            let group = group.unwrap_or(if extract { "1" } else { "0" });
            let flags = inline_flags(&parameters.replace('e', ""))
                .ok_or_else(|| refuse("REGEXP_SUBSTR parameters other than c, e, i and s"))?;
            if !positive_integer(occurrence) || !group.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(refuse(
                    "REGEXP_SUBSTR with a non-literal occurrence or group",
                ));
            }
            let _parameters = (!flags.is_empty()).then(|| log.rule(REGEXP_PARAMETERS));
            (
                format!(
                    "regexp_extract_all({text}, {}, {group})[{occurrence}]",
                    string_literal(&format!("{flags}{}", greedy(&pattern_text)), log)
                ),
                SnowflakeType::Varchar,
            )
        }
        ("REGEXP_SUBSTR_ALL", options) => {
            let (parameters, group) = match options {
                [] | ["1"] | ["1", "1"] => ("", None),
                ["1", "1", parameters] => (*parameters, None),
                ["1", "1", parameters, group] if parameters.contains('e') => {
                    (*parameters, Some(*group))
                }
                _ => return Err(refuse("REGEXP_SUBSTR_ALL with these parameters")),
            };
            let group = group.unwrap_or(if parameters.contains('e') { "1" } else { "0" });
            let flags = inline_flags(&parameters.replace('e', ""))
                .ok_or_else(|| refuse("REGEXP_SUBSTR_ALL parameters other than c, e, i and s"))?;
            if !group.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(refuse("REGEXP_SUBSTR_ALL with a non-literal group"));
            }
            let _parameters = (!flags.is_empty()).then(|| log.rule(REGEXP_PARAMETERS));
            (
                format!(
                    "CAST(to_json(regexp_extract_all({text}, {}, {group})) AS JSON)",
                    string_literal(&format!("{flags}{}", greedy(&pattern_text)), log)
                ),
                SnowflakeType::SemiStructured,
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

/// REGEXP_LIKE with a pattern from a column, raising for syntax no recording shows read alike.
fn regexp_like_column(
    text: &str,
    pattern: &str,
    rest: &[Value],
    (construct, site): (&Construct, Site<'_>),
) -> Result<Emitted, Refusal> {
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let log = site.context.log;
    let flags = match rest {
        [] => String::new(),
        [parameters] => {
            let _parameters = log.rule(REGEXP_PARAMETERS);
            match convert_expression(parameters, site)?.literal {
                Some(Literal::Text(parameters)) => inline_flags(&parameters)
                    .ok_or_else(|| refuse("REGEXP_LIKE parameters other than c, i and s"))?,
                _ => return Err(refuse("a REGEXP argument other than a literal")),
            }
        }
        _ => return Err(refuse("REGEXP_LIKE takes two or three arguments")),
    };
    diverge(site.context, REGEXP_PATTERN_SYNTAX, construct)?;
    Ok(Emitted::new(
        format!(
            "regexp_full_match({text}, {})",
            guarded_pattern(pattern, &flags, log)
        ),
        SnowflakeType::Boolean,
    ))
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

/// Checks a pattern Snowflake and RE2 read alike (`extracting`: as REGEXP_SUBSTR); true for `[:space:]`.
fn shared_pattern(pattern: &str, extracting: bool) -> Result<bool, String> {
    let mut characters = pattern.chars().peekable();
    let mut previous_quantifier = false;
    let mut in_bracket = false;
    let mut posix = false;
    let mut previous = ' ';
    while let Some(character) = characters.next() {
        let quantifier = matches!(character, '*' | '+' | '?' | '}');
        match character {
            '\\' => match characters.next() {
                Some(escaped) if REGEXP_SHORTHANDS.contains(escaped) => {}
                Some(REGEXP_LINE_FEED_ESCAPE) if extracting => {}
                Some(escaped) if escaped.is_ascii_punctuation() => {}
                Some(escaped) => return Err(format!("the regular expression escape \\{escaped}")),
                None => return Err("a pattern ending in a backslash".to_owned()),
            },
            '?' if previous_quantifier && extracting && previous != '}' => {}
            '?' if previous_quantifier => {
                return Err("a lazy quantifier".to_owned());
            }
            '(' if characters.peek() == Some(&'?') => {
                return Err("a group modifier such as a lookaround".to_owned());
            }
            '[' if in_bracket && characters.peek() == Some(&':') => {
                let class: String = characters.by_ref().take(POSIX_SPACE_CLASS.len()).collect();
                if !in_bracket || class != POSIX_SPACE_CLASS {
                    return Err("a POSIX character class other than [:space:]".to_owned());
                }
                posix = true;
            }
            '[' if !in_bracket => {
                in_bracket = true;
                if characters.peek() == Some(&'^') {
                    characters.next();
                }
                if characters.peek() == Some(&']') {
                    characters.next();
                }
            }
            ']' => in_bracket = false,
            _ => {}
        }
        previous_quantifier = quantifier && !(character == '?' && previous_quantifier);
        previous = character;
    }
    Ok(posix)
}

/// The pattern with `+?`, `*?` and `??` as the optional repetitions REGEXP_SUBSTR was recorded reading.
fn greedy(pattern: &str) -> String {
    let mut written = String::with_capacity(pattern.len());
    let mut characters = pattern.chars();
    let (mut previous_quantifier, mut in_bracket) = (false, false);
    while let Some(character) = characters.next() {
        if character == '?' && previous_quantifier && !in_bracket {
            if written.ends_with('+') {
                written.pop();
                written.push('*');
            }
            previous_quantifier = false;
            continue;
        }
        written.push(character);
        match character {
            '\\' => {
                written.extend(characters.next());
                previous_quantifier = false;
                continue;
            }
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ => {}
        }
        previous_quantifier = !in_bracket && matches!(character, '*' | '+' | '?' | '}');
    }
    written
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
