//! The text of quoted tokens, decoded as ClickHouse's `readQuotedStringWithSQLStyle` reads it.

use crate::scan::constants::{HEX_ESCAPE_DIGITS, HEX_RADIX};

/// The value a string or quoted-name token stands for, from its text as written.
pub fn unquote(text: &str) -> String {
    if let Some(inner) = text
        .strip_prefix('\u{2018}')
        .and_then(|rest| rest.strip_suffix('\u{2019}'))
        .or_else(|| {
            text.strip_prefix('\u{201C}')
                .and_then(|rest| rest.strip_suffix('\u{201D}'))
        })
    {
        return inner.to_owned();
    }
    let mut characters = text.chars();
    match (characters.next(), characters.next_back()) {
        (Some(open), Some(close)) if open == close && matches!(open, '\'' | '"' | '`') => {
            decode_body(characters.as_str(), open)
        }
        _ => text.to_owned(),
    }
}

/// The value of a quoted token's body: doubled quotes and backslash escapes decoded.
fn decode_body(body: &str, quote: char) -> String {
    let mut decoded: String = String::with_capacity(body.len());
    let mut characters = body.chars().peekable();
    while let Some(character) = characters.next() {
        if character == quote && characters.peek() == Some(&quote) {
            characters.next();
            decoded.push(quote);
            continue;
        }
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        let Some(escaped) = characters.next() else {
            decoded.push('\\');
            break;
        };
        match escaped {
            'b' => decoded.push('\u{0008}'),
            'f' => decoded.push('\u{000C}'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            '0' => decoded.push('\0'),
            'a' => decoded.push('\u{0007}'),
            'v' => decoded.push('\u{000B}'),
            'e' => decoded.push('\u{001B}'),
            'x' | 'X' => {
                let digits: String = (0..HEX_ESCAPE_DIGITS)
                    .filter_map(|_| characters.next_if(char::is_ascii_hexdigit))
                    .collect();
                let code: u32 = digits
                    .chars()
                    .filter_map(|digit| digit.to_digit(HEX_RADIX))
                    .fold(0, |code, digit| code * HEX_RADIX + digit);
                match char::from_u32(code) {
                    Some(value) if digits.len() == HEX_ESCAPE_DIGITS => decoded.push(value),
                    _ => {
                        decoded.push('\\');
                        decoded.push(escaped);
                        decoded.push_str(&digits);
                    }
                }
            }
            'N' => decoded.push_str("\\N"),
            other => decoded.push(other),
        }
    }
    decoded
}
