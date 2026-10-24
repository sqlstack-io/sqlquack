//! Parsing type patterns such as `NUMBER(p,s)`.

use crate::policy::models::{PatternArgument, TypePattern};

pub(crate) fn parse_type_pattern(text: &str) -> Result<TypePattern, String> {
    let text = text.trim();
    let (base, rest) = match text.split_once('(') {
        Some((base, rest)) => (base.trim(), Some(rest)),
        None => (text, None),
    };
    if base.is_empty()
        || !base.chars().all(|character| {
            character.is_ascii_uppercase() || character == '_' || character.is_ascii_digit()
        })
    {
        return Err(format!(
            "type pattern `{text}` needs an upper-case type name"
        ));
    }
    let arguments = match rest {
        None => Vec::new(),
        Some(rest) => {
            let inner = rest
                .strip_suffix(')')
                .ok_or_else(|| format!("type pattern `{text}` has an unclosed parenthesis"))?;
            inner
                .split(',')
                .map(|argument| parse_argument(argument.trim(), text))
                .collect::<Result<Vec<_>, String>>()?
        }
    };
    Ok(TypePattern {
        base: base.to_owned(),
        arguments,
    })
}

fn parse_argument(argument: &str, text: &str) -> Result<PatternArgument, String> {
    if let Ok(number) = argument.parse::<i64>() {
        return Ok(PatternArgument::Literal(number));
    }
    if !argument.is_empty()
        && argument
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_')
    {
        return Ok(PatternArgument::Variable(argument.to_owned()));
    }
    Err(format!(
        "type pattern `{text}` has an invalid parameter `{argument}`"
    ))
}
