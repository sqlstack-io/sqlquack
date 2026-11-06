//! Splitting SQLBuild's `FUNCTION ( ... );` header from its body, and into its properties.

use crate::analysis::constants::OVERRIDE_HEADER;

/// The text inside `FUNCTION ( ... );` and the body after it, or why `text` does not start with one.
pub(crate) fn header_and_body(text: &str) -> Result<(&str, String), String> {
    let rest = text.trim_start();
    let rest = rest
        .get(..OVERRIDE_HEADER.len())
        .filter(|head| head.eq_ignore_ascii_case(OVERRIDE_HEADER))
        .map(|_| rest[OVERRIDE_HEADER.len()..].trim_start())
        .and_then(|rest| rest.strip_prefix('('))
        .ok_or_else(|| "does not start with FUNCTION (".to_owned())?;
    let close = closing(rest).ok_or_else(|| "its FUNCTION ( header is not closed".to_owned())?;
    let body = rest[close + 1..]
        .trim_start()
        .strip_prefix(';')
        .ok_or_else(|| "its header must end with );".to_owned())?
        .trim()
        .trim_end_matches(';')
        .trim_end();
    if body.is_empty() {
        return Err("has no body".to_owned());
    }
    Ok((&rest[..close], body.to_owned()))
}

/// The header's `key value` properties, keys lower-cased.
pub(crate) fn properties(header: &str) -> Result<Vec<(String, String)>, String> {
    let mut properties: Vec<(String, String)> = Vec::new();
    for property in top_level(header)
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
    {
        let (key, value) = property
            .split_once(|character: char| character.is_whitespace() || character == '(')
            .map(|(key, _)| (key, property[key.len()..].trim()))
            .ok_or_else(|| format!("the property `{property}` has no value"))?;
        properties.push((key.to_ascii_lowercase(), value.to_owned()));
    }
    Ok(properties)
}

/// The byte offset of the parenthesis closing `text`'s open one, outside quotes, or None.
fn closing(text: &str) -> Option<usize> {
    let mut depth: usize = 1;
    let mut quote: Option<char> = None;
    for (offset, character) in text.char_indices() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(character),
            (None, '(') => depth += 1,
            (None, ')') if depth == 1 => return Some(offset),
            (None, ')') => depth -= 1,
            (None, _) => {}
        }
    }
    None
}

/// `text` split at commas outside parentheses and quotes.
pub(crate) fn top_level(text: &str) -> Vec<&str> {
    let mut parts: Vec<&str> = Vec::new();
    let (mut depth, mut start) = (0_usize, 0_usize);
    let mut quote: Option<char> = None;
    for (offset, character) in text.char_indices() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(character),
            (None, '(') => depth += 1,
            (None, ')') => depth = depth.saturating_sub(1),
            (None, ',') if depth == 0 => {
                parts.push(&text[start..offset]);
                start = offset + 1;
            }
            (None, _) => {}
        }
    }
    parts.push(&text[start..]);
    parts
}
