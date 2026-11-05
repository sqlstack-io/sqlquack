//! Replaces sqlbuild reference calls outside comments and quoted text with parseable SQL.

use std::collections::BTreeSet;

use sqlquack_core::parsing::models::{SourceLines, SourceLocation};

use sqlquack_core::classification::constants::{CURSOR_PLACEHOLDER, PROJECT_FUNCTION_PLACEHOLDER};

use crate::census::constants::{
    CURSOR_CALLS, MODEL_CALLS, RELATION_CALLS, RELATION_PLACEHOLDER, TABLE_FUNCTION_PLACEHOLDER,
};
use crate::census::models::{Replacement, Rewritten};

const PROJECT_FUNCTION_CALL: &str = "__udf";
const TABLE_FUNCTION_CALL: &str = "__table_fn";
/// The keyword SQLBuild wraps a table function call in, and its opening.
const TABLE_KEYWORD: &str = "TABLE";
const TABLE_WRAPPER: &str = "TABLE(";

/// The rewritten SQL, its placeholder relations and replacements, cursors as `cursor_bound`.
pub(crate) fn rewrite_with_references(sql: &str, cursor_bound: Option<&str>) -> Rewritten {
    let mut references: BTreeSet<String> = BTreeSet::new();
    let mut model_references: BTreeSet<String> = BTreeSet::new();
    let mut replacements: Vec<Replacement> = Vec::new();
    if !sql.contains("__") {
        return Rewritten {
            sql: sql.to_owned(),
            references,
            model_references,
            replacements,
        };
    }
    let bytes = sql.as_bytes();
    let mut rewritten = String::with_capacity(sql.len());
    let mut position = 0;
    let mut copied = 0;
    let mut closing: Vec<usize> = Vec::new();
    while position <= bytes.len() {
        if closing.last() == Some(&position) {
            closing.pop();
            rewritten.push_str(&sql[copied..position]);
            replacements.push(Replacement {
                rewritten: rewritten.len()..rewritten.len() + 1,
                original: position..position,
            });
            rewritten.push(')');
            copied = position;
            continue;
        }
        if position == bytes.len() {
            break;
        }
        let skipped = skip_inert(bytes, position);
        if skipped > position {
            position = skipped;
            continue;
        }
        if bytes[position] == b'_'
            && bytes.get(position + 1) == Some(&b'_')
            && (position == 0 || !is_identifier_byte(bytes[position - 1]))
        {
            let name_end = identifier_end(bytes, position);
            let name = sql[position..name_end].to_ascii_lowercase();
            if let Some((mut replacement, end)) = replacement(bytes, &name, name_end, cursor_bound)
            {
                if let Some(arguments_end) = unwrapped_table_call(bytes, &name, position, end) {
                    replacement.insert_str(0, TABLE_WRAPPER);
                    closing.push(arguments_end);
                }
                if RELATION_CALLS.contains(&name.as_str()) {
                    references.insert(replacement.to_ascii_uppercase());
                }
                if MODEL_CALLS.contains(&name.as_str()) {
                    model_references.insert(replacement.to_ascii_uppercase());
                }
                rewritten.push_str(&sql[copied..position]);
                replacements.push(Replacement {
                    rewritten: rewritten.len()..rewritten.len() + replacement.len(),
                    original: position..end,
                });
                rewritten.push_str(&replacement);
                position = end;
                copied = end;
                continue;
            }
            position = name_end;
            continue;
        }
        position += 1;
    }
    rewritten.push_str(&sql[copied..]);
    Rewritten {
        sql: rewritten,
        references,
        model_references,
        replacements,
    }
}

/// The location in the original SQL of `location`, a location in the rewritten SQL.
pub(crate) fn original_location(
    rewritten: &Rewritten,
    original: &str,
    location: SourceLocation,
) -> SourceLocation {
    let original_char = |rewritten_char: usize| {
        let byte = byte_offset(&rewritten.sql, rewritten_char);
        let mut shifted = byte;
        for replacement in &rewritten.replacements {
            if byte < replacement.rewritten.start {
                break;
            }
            shifted = match byte < replacement.rewritten.end {
                true => replacement.original.start,
                false => byte - replacement.rewritten.end + replacement.original.end,
            };
        }
        original[..shifted.min(original.len())].chars().count()
    };
    SourceLines::new(original).location(original_char(location.start), original_char(location.end))
}

fn byte_offset(text: &str, characters: usize) -> usize {
    text.char_indices()
        .nth(characters)
        .map_or(text.len(), |(index, _)| index)
}

fn replacement(
    bytes: &[u8],
    name: &str,
    name_end: usize,
    cursor_bound: Option<&str>,
) -> Option<(String, usize)> {
    let open = skip_whitespace(bytes, name_end);
    let call_end = if bytes.get(open) == Some(&b'(') {
        Some(matching_paren(bytes, open)?)
    } else {
        None
    };
    if RELATION_CALLS.contains(&name) {
        let end = call_end?;
        return Some((relation_name(&bytes[open + 1..end - 1]), end));
    }
    if CURSOR_CALLS.contains(&name) {
        return Some((
            cursor_bound.unwrap_or(CURSOR_PLACEHOLDER).to_owned(),
            call_end.unwrap_or(name_end),
        ));
    }
    match name {
        PROJECT_FUNCTION_CALL => Some((
            placeholder_name(
                PROJECT_FUNCTION_PLACEHOLDER,
                &bytes[open + 1..call_end? - 1],
            ),
            call_end?,
        )),
        TABLE_FUNCTION_CALL => Some((
            placeholder_name(TABLE_FUNCTION_PLACEHOLDER, &bytes[open + 1..call_end? - 1]),
            call_end?,
        )),
        _ => None,
    }
}

/// Where the arguments of a table function call SQLBuild wraps in TABLE(...) end, unless written in one.
fn unwrapped_table_call(bytes: &[u8], name: &str, start: usize, end: usize) -> Option<usize> {
    let open = skip_whitespace(bytes, end);
    if name != TABLE_FUNCTION_CALL || bytes.get(open) != Some(&b'(') {
        return None;
    }
    let before = String::from_utf8_lossy(&bytes[..start]);
    let wrapped = before
        .trim_end()
        .strip_suffix('(')
        .map(str::trim_end)
        .and_then(|before| {
            let keyword = before.len().checked_sub(TABLE_KEYWORD.len())?;
            let preceding = before.as_bytes()[..keyword].last().copied();
            Some(
                before[keyword..].eq_ignore_ascii_case(TABLE_KEYWORD)
                    && !preceding.is_some_and(is_identifier_byte),
            )
        })
        .unwrap_or(false);
    match wrapped {
        true => None,
        false => matching_paren(bytes, open),
    }
}

/// The placeholder relation a reference becomes: its arguments joined by `__`, so schemas name it.
pub(crate) fn relation_name(arguments: &[u8]) -> String {
    placeholder_name(RELATION_PLACEHOLDER, arguments)
}

/// `prefix` and each argument, cleaned to identifier characters, joined by `__`.
fn placeholder_name(prefix: &str, arguments: &[u8]) -> String {
    let text = String::from_utf8_lossy(arguments);
    let mut name = prefix.to_owned();
    for argument in text.split(',') {
        let cleaned: String = argument
            .trim()
            .trim_matches(['"', '\''])
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        name.push_str("__");
        name.push_str(&cleaned);
    }
    name
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

fn identifier_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && is_identifier_byte(bytes[end]) {
        end += 1;
    }
    end
}

fn skip_whitespace(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_whitespace() {
        end += 1;
    }
    end
}

fn matching_paren(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut position = open;
    while position < bytes.len() {
        let skipped = skip_inert(bytes, position);
        if skipped > position {
            position = skipped;
            continue;
        }
        match bytes[position] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(position + 1);
                }
            }
            _ => {}
        }
        position += 1;
    }
    None
}

fn skip_inert(bytes: &[u8], start: usize) -> usize {
    match (bytes[start], bytes.get(start + 1)) {
        (b'-', Some(b'-')) => find(bytes, start + 2, b"\n").unwrap_or(bytes.len()),
        (b'/', Some(b'*')) => find(bytes, start + 2, b"*/").map_or(bytes.len(), |end| end + 2),
        (b'$', Some(b'$')) => find(bytes, start + 2, b"$$").map_or(bytes.len(), |end| end + 2),
        (b'\'', _) => quoted_end(bytes, start, b'\'', true),
        (b'"', _) => quoted_end(bytes, start, b'"', false),
        _ => start,
    }
}

fn quoted_end(bytes: &[u8], start: usize, quote: u8, backslash_escapes: bool) -> usize {
    let mut position = start + 1;
    while position < bytes.len() {
        let byte = bytes[position];
        if backslash_escapes && byte == b'\\' {
            position += 2;
            continue;
        }
        if byte == quote {
            if bytes.get(position + 1) == Some(&quote) {
                position += 2;
                continue;
            }
            return position + 1;
        }
        position += 1;
    }
    bytes.len()
}

fn find(bytes: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(start..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| start + offset)
}
