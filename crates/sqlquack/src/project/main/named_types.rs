//! `named_types`: a header's `(name TYPE, ...)` list as name and type text pairs.

use crate::project::_helpers::header_parts::top_level;

/// `(name TYPE, ...)` as name and type text pairs, the name a plain identifier.
pub fn named_types(value: &str) -> Result<Vec<(String, String)>, String> {
    let inner = value
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .ok_or_else(|| "arguments must be written `arguments (name TYPE, ...)`".to_owned())?;
    let mut named: Vec<(String, String)> = Vec::new();
    for part in top_level(inner)
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
    {
        let (name, type_name) = part
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("the argument `{part}` has no type"))?;
        let identifier = name.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_alphabetic()
                || (index > 0 && character.is_ascii_digit())
        });
        if !identifier {
            return Err(format!(
                "the argument name `{name}` is not a plain identifier"
            ));
        }
        named.push((name.to_owned(), type_name.trim().to_owned()));
    }
    Ok(named)
}
