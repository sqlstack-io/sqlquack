//! LEFT, RIGHT, SUBSTR, LENGTH, REPLACE, TRANSLATE, SPLIT_PART, LIKE and the REGEXP functions.

use serde_json::Value;
use sqlquack_blocks::blocks::main::cast::cast;
use sqlquack_blocks::blocks::main::split_text_part::split_text_part;
use sqlquack_blocks::blocks::main::string_literal::string_literal;
use sqlquack_blocks::raises::main::raised::raised;
use sqlquack_blocks::text::constants::{
    CASE_FOLDING, LONG_S, REGEXP_PATTERN_SYNTAX, UNFOLDED_LETTERS,
};
use sqlquack_blocks::text::main::folded_text::folded_text;
use sqlquack_blocks::text::main::guarded_pattern::guarded_pattern;
use sqlquack_diagnostics::classification::models::Construct;
use sqlquack_diagnostics::refusals::main::outside_conditions::outside_conditions;
use sqlquack_diagnostics::refusals::models::Refusal;
use sqlquack_semantics::snowflake::typing::models::SnowflakeType;

use crate::conversion::_helpers::expressions::result_types::result_type;
use crate::conversion::_helpers::expressions::scalar::{call_name, convert_expression};
use crate::conversion::_helpers::expressions::strings::text_operand;
use crate::conversion::_helpers::support::divergence::diverge;
use crate::conversion::_helpers::support::scope::{Emitted, Literal, resolve};
use crate::conversion::_helpers::support::site::Site;
use crate::conversion::_helpers::support::tree::{
    field, flag, function_construct, identifier_name, list, node_construct, node_kind,
    payload_object, require_understood,
};
use crate::conversion::constants::{
    COMPUTED_SUBSTR_LENGTH, FUNCTION_KIND, ILIKE_FOLDING, INFIX_NEGATION, LENGTH_TYPE, LIKE,
    LIKE_WILDCARDS, NEGATED_FACET, NO_QUANTIFIER, NULL_KIND, POSIX_SPACE_CLASS, QUANTIFIER_FACET,
    REGEXP, REGEXP_COLUMN_PATTERN, REGEXP_CONTROL_ESCAPES, REGEXP_COUNT_TYPE, REGEXP_COUNTING,
    REGEXP_EXTRACTING_FUNCTIONS, REGEXP_PARAMETERS, REGEXP_POSIX_SPACE, REGEXP_SHORTHANDS,
    STORAGE_SUFFIX_PATTERN, SYSTEM_TYPE_NAME, SYSTEM_TYPEOF_NAME, TEXT_PAD, TEXT_POSITION,
    TEXT_POSITION_LIMIT, TEXT_REPLACE, TEXT_TRANSLATE, WIDENING_DATE_ADDS,
};

/// The quantifier LIKE ANY is written with.
const LIKE_ANY: &str = "ANY";
/// The node kind of a parenthesised list.
const TUPLE_KIND: &str = "tuple";
/// The node kind of ILIKE, the one LIKE recorded with ANY.
const ILIKE_KIND: &str = "i_like";
/// `\b`, an ASCII word boundary in both engines, as REGEXP_SUBSTR was recorded.
const WORD_BOUNDARY_ESCAPE: char = 'b';

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
            "list_transform([struct_pack(t := {text}, n := {length})], lambda p: {}(p['t'], \
             (CASE WHEN p['n'] < 0 THEN 0 WHEN p['n'] > {TEXT_POSITION_LIMIT} \
             THEN {TEXT_POSITION_LIMIT} ELSE p['n'] END)))[1]",
            name.to_ascii_lowercase()
        ),
        SnowflakeType::Varchar,
    ))
}

/// SUBSTR or SUBSTRING; start 0 counts as 1, and a negative length as 0.
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
                    let _rule = site.context.log.rule(COMPUTED_SUBSTR_LENGTH);
                    let length = integer_operand(length, &construct, site)?;
                    format!(", (CASE WHEN {length} < 0 THEN 0 ELSE {length} END)")
                }
            }
        }
    };
    Ok(Emitted::new(
        format!(
            "list_transform([struct_pack(t := {text}, s := {start})], lambda p: \
             list_transform([struct_pack(t := p['t'], s := (CASE WHEN p['s'] < -{TEXT_POSITION_LIMIT} \
             THEN -{TEXT_POSITION_LIMIT} WHEN p['s'] > {TEXT_POSITION_LIMIT} THEN {TEXT_POSITION_LIMIT} \
             ELSE p['s'] END))], lambda q: CASE WHEN q['s'] < 0 AND -q['s'] > length(q['t']) THEN '' \
             ELSE substring(q['t'], (CASE WHEN q['s'] = 0 THEN 1 ELSE q['s'] END){length}) END)[1])[1]"
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
    let delimiter = string_literal(&delimiter, site.context.log);
    Ok(Emitted::new(
        split_text_part(&text, (&delimiter, &part), site.context.log),
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
    let text = match kind {
        ILIKE_KIND => {
            diverge(site.context, CASE_FOLDING, &construct)?;
            folded_text(&text, site.context.log)
        }
        _ => text,
    };
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
        let pattern = match kind {
            ILIKE_KIND => {
                let literal = text_literal(pattern_node, &construct, site)?;
                escape_kept(&literal, escape_character, &construct)?;
                folded_pattern(&literal, &construct).map(|folded| string_literal(&folded, log))?
            }
            _ => text_operand(pattern_node, &construct, site)?,
        };
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
        (_, "REGEXP_REPLACE" | "REGEXP_SUBSTR") if pattern.data_type == SnowflakeType::Varchar => {
            return regexp_column(name, (&text, &pattern.sql), rest, (construct, site));
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
                ["1", occurrence, parameters, group] => (*occurrence, *parameters, Some(*group)),
                _ => return Err(refuse("REGEXP_SUBSTR with these parameters")),
            };
            let extract = parameters.contains('e');
            let group = group.unwrap_or(if extract { "1" } else { "0" });
            let flags = extracting_flags(parameters)
                .ok_or_else(|| refuse("REGEXP_SUBSTR parameters other than c, e, i, m and s"))?;
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
                ["1", "1", parameters, group] => (*parameters, Some(*group)),
                _ => return Err(refuse("REGEXP_SUBSTR_ALL with these parameters")),
            };
            let group = group.unwrap_or(if parameters.contains('e') { "1" } else { "0" });
            let flags = extracting_flags(parameters).ok_or_else(|| {
                refuse("REGEXP_SUBSTR_ALL parameters other than c, e, i, m and s")
            })?;
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
        ("REGEXP_COUNT", options) => {
            let parameters = match options {
                [] | ["1"] => "",
                ["1", parameters] => *parameters,
                _ => return Err(refuse("REGEXP_COUNT from a position other than 1")),
            };
            let flags = extracting_flags(parameters)
                .ok_or_else(|| refuse("REGEXP_COUNT parameters other than c, i, m and s"))?;
            let _counted = log.rule(REGEXP_COUNTING);
            let pattern = string_literal(&format!("{flags}{}", greedy(&pattern_text)), log);
            (
                format!(
                    "(CASE WHEN {} THEN CAST(NULL AS BIGINT) \
                     ELSE len(regexp_extract_all({text}, {pattern}, 0)) END)",
                    empty_match_guard(name, &pattern, (construct, site))?
                ),
                REGEXP_COUNT_TYPE,
            )
        }
        ("REGEXP_INSTR", []) => {
            let _counted = log.rule(REGEXP_COUNTING);
            let read = greedy(&pattern_text);
            let pattern = string_literal(&read, log);
            let prefixed = string_literal(&format!("^((?s:.*?))(?:{read})"), log);
            (
                format!(
                    "list_transform([{text}], lambda t: CASE WHEN {} THEN CAST(NULL AS BIGINT) \
                     WHEN t IS NULL THEN NULL WHEN regexp_matches(t, {pattern}) \
                     THEN length(regexp_extract(t, {prefixed}, 1)) + 1 ELSE 0 END)[1]",
                    empty_match_guard(name, &pattern, (construct, site))?
                ),
                REGEXP_COUNT_TYPE,
            )
        }
        _ => return Err(refuse("a REGEXP function with these arguments")),
    };
    let sql = match data_type {
        REGEXP_COUNT_TYPE => cast(&sql, data_type, log).map_err(|reason| refuse(&reason))?,
        _ => sql,
    };
    Ok(Emitted::new(sql, data_type))
}

/// A condition raising regexp_pattern_syntax where the pattern matches empty text; FALSE otherwise.
fn empty_match_guard(
    name: &str,
    pattern: &str,
    (construct, site): (&Construct, Site<'_>),
) -> Result<String, Refusal> {
    diverge(site.context, REGEXP_PATTERN_SYNTAX, construct)?;
    let raise = raised(&format!(
        "{REGEXP_PATTERN_SYNTAX}: a {name} pattern matching empty text, whose matches no \
         recording shows"
    ));
    Ok(format!(
        "(CASE WHEN regexp_matches('', {pattern}) THEN CAST({raise} AS BOOLEAN) ELSE FALSE END)"
    ))
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

/// RE2 inline flags for REGEXP_SUBSTR's parameters: those of [`inline_flags`], e, and m for multi-line.
fn extracting_flags(parameters: &str) -> Option<String> {
    let flags = inline_flags(&parameters.replace(['e', 'm'], ""))?;
    Some(match (parameters.contains('m'), flags.strip_suffix(')')) {
        (false, _) => flags,
        (true, Some(open)) => format!("{open}m)"),
        (true, None) => "(?m)".to_owned(),
    })
}

fn positive_integer(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| byte.is_ascii_digit())
        && !text.trim_start_matches('0').is_empty()
}

/// A NUMBER operand of scale 0, as a BIGINT expression.
pub(crate) fn integer_operand(
    node: &Value,
    construct: &Construct,
    site: Site<'_>,
) -> Result<String, Refusal> {
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

/// REGEXP_REPLACE and two-argument REGEXP_SUBSTR with a column pattern, checked as REGEXP_LIKE's is.
fn regexp_column(
    name: &str,
    (text, pattern): (&str, &str),
    rest: &[Value],
    (construct, site): (&Construct, Site<'_>),
) -> Result<Emitted, Refusal> {
    let refuse = |reason: &str| outside_conditions(construct.clone(), reason.to_owned());
    let log = site.context.log;
    let _rule = log.rule(REGEXP_COLUMN_PATTERN);
    let mut literals: Vec<String> = Vec::with_capacity(rest.len());
    for argument in rest {
        match convert_expression(argument, site)?.literal {
            Some(Literal::Text(text) | Literal::Number(text)) => literals.push(text),
            None => return Err(refuse("a REGEXP argument other than a literal")),
        }
    }
    let literals: Vec<&str> = literals.iter().map(String::as_str).collect();
    let (call, flags) = match (name, literals.as_slice()) {
        ("REGEXP_REPLACE", []) => (
            "regexp_replace(r['t'], r['q'], '', 'g')".to_owned(),
            String::new(),
        ),
        ("REGEXP_REPLACE", [replacement]) => {
            shared_replacement(replacement).map_err(|reason| refuse(&reason))?;
            (
                format!(
                    "regexp_replace(r['t'], r['q'], {}, 'g')",
                    string_literal(replacement, log)
                ),
                String::new(),
            )
        }
        ("REGEXP_SUBSTR", options) => {
            let (occurrence, parameters, group) = match options {
                [] | ["1"] => ("1", "", None),
                ["1", occurrence] => (*occurrence, "", None),
                ["1", occurrence, parameters] => (*occurrence, *parameters, None),
                ["1", occurrence, parameters, group] => (*occurrence, *parameters, Some(*group)),
                _ => return Err(refuse("REGEXP_SUBSTR with these parameters")),
            };
            let group = group.unwrap_or(if parameters.contains('e') { "1" } else { "0" });
            let flags = extracting_flags(parameters)
                .ok_or_else(|| refuse("REGEXP_SUBSTR parameters other than c, e, i, m and s"))?;
            if !positive_integer(occurrence) || !group.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(refuse(
                    "REGEXP_SUBSTR with a non-literal occurrence or group",
                ));
            }
            let _parameters = (!flags.is_empty()).then(|| log.rule(REGEXP_PARAMETERS));
            (
                format!("regexp_extract_all(r['t'], r['q'], {group})[{occurrence}]"),
                flags,
            )
        }
        _ => {
            return Err(refuse(
                "a REGEXP function of a column pattern with these arguments",
            ));
        }
    };
    diverge(site.context, REGEXP_PATTERN_SYNTAX, construct)?;
    let empty = raised(&format!(
        "{REGEXP_PATTERN_SYNTAX}: a {name} pattern from a column matching empty text, whose \
         matches no recording shows"
    ));
    Ok(Emitted::new(
        format!(
            "list_transform([struct_pack(t := {text}, q := {})], lambda r: CASE WHEN r['q'] IS NULL \
             THEN NULL WHEN regexp_matches('', r['q']) THEN CAST({empty} AS VARCHAR) ELSE {call} END)[1]",
            guarded_pattern(pattern, &flags, log)
        ),
        SnowflakeType::Varchar,
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
    let mut lazy = false;
    while let Some(character) = characters.next() {
        let quantifier = matches!(character, '*' | '+' | '?' | '}');
        let follows_lazy = std::mem::take(&mut lazy);
        match character {
            '\\' => match characters.next() {
                Some(escaped) if REGEXP_SHORTHANDS.contains(escaped) => {}
                Some(escaped) if extracting && REGEXP_CONTROL_ESCAPES.contains(escaped) => {}
                Some(escaped) if escaped.is_ascii_punctuation() => {}
                Some(WORD_BOUNDARY_ESCAPE) if extracting && !in_bracket => {}
                Some(escaped) => return Err(format!("the regular expression escape \\{escaped}")),
                None => return Err("a pattern ending in a backslash".to_owned()),
            },
            '*' | '+' | '?' | '{' if follows_lazy && !in_bracket => {
                return Err("a quantifier directly after a lazy quantifier".to_owned());
            }
            '?' if previous_quantifier && extracting && previous != '}' => lazy = !in_bracket,
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
    let mut characters = pattern.chars().peekable();
    let (mut previous_quantifier, mut in_bracket) = (false, false);
    while let Some(character) = characters.next() {
        if character == '[' && !in_bracket {
            written.push(character);
            written.extend(characters.next_if_eq(&'^'));
            written.extend(characters.next_if_eq(&']'));
            (in_bracket, previous_quantifier) = (true, false);
            continue;
        }
        if character == '[' && in_bracket && characters.peek() == Some(&':') {
            written.push(character);
            while let Some(class) = characters.next() {
                written.push(class);
                if class == ':' && characters.peek() == Some(&']') {
                    written.extend(characters.next());
                    break;
                }
            }
            continue;
        }
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

/// An ILIKE pattern with the long s as s; a letter whose folding no recording shows is refused.
fn folded_pattern(pattern: &str, construct: &Construct) -> Result<String, Refusal> {
    match pattern.chars().any(unfolded) {
        true => Err(outside_conditions(
            construct.clone(),
            "an ILIKE pattern holding a letter whose case folding no recording shows".to_owned(),
        )),
        false => Ok(pattern.replace(LONG_S, "s")),
    }
}

/// Whether `character` is a letter whose case folding no recording shows.
fn unfolded(character: char) -> bool {
    let point = u32::from(character);
    UNFOLDED_LETTERS
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&point))
}

/// Refuses an ILIKE escape that folding or lower-casing the pattern could create or remove.
fn escape_kept(pattern: &str, escape: Option<char>, construct: &Construct) -> Result<(), Refusal> {
    let Some(escape) = escape else {
        return Ok(());
    };
    let folded = pattern.contains(LONG_S);
    let lowered = !pattern.is_ascii() && !escape.to_lowercase().eq([escape]);
    match folded || lowered {
        true => Err(outside_conditions(
            construct.clone(),
            "an ILIKE pattern with an ESCAPE that folding or lower-casing the pattern changes"
                .to_owned(),
        )),
        false => Ok(()),
    }
}

/// The SYSTEM$TYPEOF call `node` reads, if it is one.
pub(crate) fn system_typeof_call(node: &Value) -> Option<&Value> {
    let (kind, payload) = node_kind(node)?;
    (kind == FUNCTION_KIND && call_name(payload) == SYSTEM_TYPEOF_NAME).then_some(payload)
}

/// REGEXP_REPLACE(t, '\\[.*\\]$', '') of SYSTEM$TYPEOF text: the type name without its suffix.
pub(crate) fn stripped_type_name(
    (typed, pattern, replacement): (&Value, &Value, &Value),
    site: Site<'_>,
) -> Result<Emitted, Refusal> {
    let _rule = site.context.log.rule(SYSTEM_TYPE_NAME);
    let construct = function_construct(SYSTEM_TYPEOF_NAME);
    let literal = |node: &Value| -> Result<Option<String>, Refusal> {
        Ok(match convert_expression(node, site)?.literal {
            Some(Literal::Text(text)) => Some(text),
            _ => None,
        })
    };
    if literal(pattern)?.as_deref() != Some(STORAGE_SUFFIX_PATTERN)
        || literal(replacement)?.as_deref() != Some("")
    {
        return Err(outside_conditions(
            construct,
            format!(
                "SYSTEM$TYPEOF other than with its storage suffix removed by REGEXP_REPLACE(..., \
                 '{STORAGE_SUFFIX_PATTERN}', '')"
            ),
        ));
    }
    let named = match system_typeof_call(typed) {
        Some(call) => {
            let payload = payload_object(call, &construct)?;
            require_understood(payload, &["name", "args"], &construct)?;
            let [operand] = list(payload, "args") else {
                return Err(outside_conditions(
                    construct,
                    "SYSTEM$TYPEOF with these arguments".to_owned(),
                ));
            };
            type_name(operand, site)?
        }
        None => convert_expression(typed, site.suffix_free(true))?,
    };
    Ok(Emitted {
        suffixed: false,
        ..named
    })
}

/// Whether `node` reads a column of SYSTEM$TYPEOF text whose storage suffix is still on.
pub(crate) fn suffixed_column(node: &Value, site: Site<'_>) -> bool {
    let Some(("column", column)) = node_kind(node) else {
        return false;
    };
    let qualifier = column.get("table").and_then(identifier_name);
    let log = site.context.log;
    let _muted = log.muted();
    column
        .get("name")
        .and_then(identifier_name)
        .is_some_and(|name| {
            resolve(site.scope, qualifier.as_deref(), &name, log).is_ok_and(|read| read.suffixed)
        })
}

/// Whether `node` is `x ILIKE NULL`, the predicate with a NULL operand Snowflake was recorded typing.
fn null_pattern(node: &Value) -> bool {
    node_kind(node).is_some_and(|(kind, payload)| {
        kind == ILIKE_KIND
            && payload.get("quantifier").is_none_or(Value::is_null)
            && payload
                .get("right")
                .and_then(node_kind)
                .is_some_and(|(right, _)| right == NULL_KIND)
    })
}

/// SYSTEM$TYPEOF(operand): the type name without its plan-decided suffix, marked as suffixed.
pub(crate) fn type_name(operand: &Value, site: Site<'_>) -> Result<Emitted, Refusal> {
    let log = site.context.log;
    let _rule = log.rule(SYSTEM_TYPE_NAME);
    let construct = function_construct(SYSTEM_TYPEOF_NAME);
    let refuse = |reason: String| outside_conditions(construct.clone(), reason);
    let data_type = match convert_expression(operand, site) {
        Ok(converted) => converted.data_type,
        Err(refusal) => result_type(operand).ok_or(refusal)?,
    };
    let widened = node_kind(operand).is_some_and(|(kind, call)| {
        kind == FUNCTION_KIND && WIDENING_DATE_ADDS.contains(&call_name(call).as_str())
    }) && matches!(data_type, SnowflakeType::TimestampNtz { precision } | SnowflakeType::TimestampLtz { precision } | SnowflakeType::TimestampTz { precision } if precision < 9);
    let null_operand = node_kind(operand)
        .and_then(|(_, payload)| payload.as_object())
        .is_some_and(|payload| {
            payload
                .values()
                .any(|child| node_kind(child).is_some_and(|(kind, _)| kind == NULL_KIND))
        });
    let name = match data_type {
        SnowflakeType::Varchar | SnowflakeType::SemiStructured => {
            return Err(refuse(format!(
                "SYSTEM$TYPEOF of {data_type}, whose length or kind is not tracked"
            )));
        }
        _ if widened => {
            return Err(refuse(format!(
                "SYSTEM$TYPEOF of DATEADD of a {data_type}, which Snowflake was recorded widening \
                 to nanoseconds"
            )));
        }
        SnowflakeType::Boolean if null_pattern(operand) => SnowflakeType::Null.to_string(),
        SnowflakeType::Boolean if null_operand => {
            return Err(refuse(
                "SYSTEM$TYPEOF of a predicate other than ILIKE with a NULL operand, which \
                 Snowflake was recorded typing NULL for ILIKE"
                    .to_owned(),
            ));
        }
        _ => data_type.to_string(),
    };
    Ok(Emitted {
        suffixed: true,
        ..Emitted::new(string_literal(&name, log), SnowflakeType::Varchar)
    })
}
