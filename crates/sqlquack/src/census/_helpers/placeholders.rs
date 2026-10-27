//! Replaces sqlbuild reference calls outside comments and quoted text with parseable SQL.

use std::collections::BTreeSet;

use sqlquack_core::classification::constants::PROJECT_FUNCTION_PLACEHOLDER;

use crate::census::constants::{
    CURSOR_CALLS, CURSOR_PLACEHOLDER, RELATION_CALLS, RELATION_PLACEHOLDER,
    TABLE_FUNCTION_PLACEHOLDER,
};

const PROJECT_FUNCTION_CALL: &str = "__udf";
const TABLE_FUNCTION_CALL: &str = "__table_fn";

/// The rewritten SQL, and the placeholder relation each reference call became, in upper case.
pub(crate) fn rewrite_with_references(sql: &str) -> (String, BTreeSet<String>) {
    let mut references: BTreeSet<String> = BTreeSet::new();
    if !sql.contains("__") {
        return (sql.to_owned(), references);
    }
    let bytes = sql.as_bytes();
    let mut rewritten = String::with_capacity(sql.len());
    let mut position = 0;
    let mut copied = 0;
    while position < bytes.len() {
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
            if let Some((replacement, end)) = replacement(bytes, &name, name_end) {
                if RELATION_CALLS.contains(&name.as_str()) {
                    references.insert(replacement.to_ascii_uppercase());
                }
                rewritten.push_str(&sql[copied..position]);
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
    (rewritten, references)
}

fn replacement(bytes: &[u8], name: &str, name_end: usize) -> Option<(String, usize)> {
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
        return Some((CURSOR_PLACEHOLDER.to_owned(), call_end.unwrap_or(name_end)));
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
