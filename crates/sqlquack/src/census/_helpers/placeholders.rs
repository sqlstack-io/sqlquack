//! Replaces sqlbuild reference calls outside comments and quoted text with parseable SQL.

use sqlquack_core::classification::constants::PROJECT_FUNCTION_PLACEHOLDER;

use crate::census::constants::{
    CURSOR_CALLS, CURSOR_PLACEHOLDER, RELATION_CALLS, RELATION_PLACEHOLDER,
    TABLE_FUNCTION_PLACEHOLDER,
};

const PROJECT_FUNCTION_CALL: &str = "__udf";
const TABLE_FUNCTION_CALL: &str = "__table_fn";

pub(crate) fn rewrite_placeholders(sql: &str) -> String {
    if !sql.contains("__") {
        return sql.to_owned();
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
                rewritten.push_str(&sql[copied..position]);
                rewritten.push_str(replacement);
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
    rewritten
}

fn replacement(bytes: &[u8], name: &str, name_end: usize) -> Option<(&'static str, usize)> {
    let open = skip_whitespace(bytes, name_end);
    let call_end = if bytes.get(open) == Some(&b'(') {
        Some(matching_paren(bytes, open)?)
    } else {
        None
    };
    if RELATION_CALLS.contains(&name) {
        return Some((RELATION_PLACEHOLDER, call_end?));
    }
    if CURSOR_CALLS.contains(&name) {
        return Some((CURSOR_PLACEHOLDER, call_end.unwrap_or(name_end)));
    }
    match name {
        PROJECT_FUNCTION_CALL => Some((PROJECT_FUNCTION_PLACEHOLDER, call_end?)),
        TABLE_FUNCTION_CALL => Some((TABLE_FUNCTION_PLACEHOLDER, call_end?)),
        _ => None,
    }
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
