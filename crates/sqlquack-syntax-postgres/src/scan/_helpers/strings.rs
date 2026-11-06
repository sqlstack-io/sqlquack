//! Quoted bodies: where they end and how their escapes decode (scan.l and parser.c).

use crate::scan::_helpers::scanner::Fault;
use crate::scan::constants::HEX_RADIX;
use crate::scan::main::digits::digits_value;

const FIRST_SURROGATE: std::ops::RangeInclusive<u32> = 0xD800..=0xDBFF;
const SECOND_SURROGATE: std::ops::RangeInclusive<u32> = 0xDC00..=0xDFFF;

/// The index after the quote closing the body opened at `open`; a doubled quote stays inside.
pub(crate) fn quoted_end(bytes: &[u8], open: usize, quote: u8, escapes: bool) -> Option<usize> {
    let mut at: usize = open + 1;
    while let Some(&byte) = bytes.get(at) {
        if (escapes && byte == b'\\') || (byte == quote && bytes.get(at + 1) == Some(&quote)) {
            at += 2;
        } else if byte == quote {
            return Some(at + 1);
        } else {
            at += 1;
        }
    }
    None
}

fn hex_value(text: &str) -> Option<u32> {
    digits_value(text, HEX_RADIX)
}

fn surrogate_pair(first: u32, second: u32) -> u32 {
    ((first & 0x3FF) << 10) + 0x10000 + (second & 0x3FF)
}

/// The character for a code point, or the syntax error PostgreSQL gives for an invalid one.
fn code_point(value: u32, position: usize) -> Result<char, Fault> {
    char::from_u32(value)
        .filter(|_| value != 0)
        .ok_or_else(|| Fault::syntax("invalid Unicode escape value", position))
}

/// A `\u` or `\U` escape at the start of `rest`: its code point and length, or why it fails.
fn escape_unicode(rest: &str, position: usize) -> Result<(u32, usize), Fault> {
    let width: usize = match rest.as_bytes().get(1) {
        Some(b'u') => 4,
        _ => 8,
    };
    match rest.get(2..2 + width).and_then(hex_value) {
        Some(value) => Ok((value, 2 + width)),
        None => Err(Fault {
            message: "invalid Unicode escape".to_owned(),
            syntax: false,
            position,
        }),
    }
}

/// An `E'...'` body with its backslash escapes applied; bytes that are not UTF-8 become U+FFFD.
pub(crate) fn decode_escape_string(body: &str, offset: usize) -> Result<String, Fault> {
    let mut bytes: Vec<u8> = Vec::with_capacity(body.len());
    let mut at: usize = 0;
    let text: &[u8] = body.as_bytes();
    while at < text.len() {
        let byte: u8 = text[at];
        if byte == b'\'' {
            bytes.push(b'\'');
            at += 2;
            continue;
        }
        if byte != b'\\' || at + 1 >= text.len() {
            bytes.push(byte);
            at += 1;
            continue;
        }
        let next: u8 = text[at + 1];
        match next {
            b'u' | b'U' => {
                let (value, length) = escape_unicode(&body[at..], offset + at)?;
                at += length;
                let value: u32 = match value {
                    value if SECOND_SURROGATE.contains(&value) => {
                        return Err(Fault::syntax("invalid Unicode surrogate pair", offset + at));
                    }
                    value if FIRST_SURROGATE.contains(&value) => {
                        let follows: bool = matches!(text.get(at + 1), Some(b'u' | b'U'))
                            && text.get(at) == Some(&b'\\');
                        let (second, length) = match follows {
                            true => escape_unicode(&body[at..], offset + at)?,
                            false => {
                                return Err(Fault::syntax(
                                    "invalid Unicode surrogate pair",
                                    offset + at,
                                ));
                            }
                        };
                        if !SECOND_SURROGATE.contains(&second) {
                            return Err(Fault::syntax(
                                "invalid Unicode surrogate pair",
                                offset + at,
                            ));
                        }
                        at += length;
                        surrogate_pair(value, second)
                    }
                    value => value,
                };
                let character: char = code_point(value, offset + at)?;
                bytes.extend(character.to_string().as_bytes());
            }
            b'0'..=b'7' => {
                let digits: usize = text[at + 1..]
                    .iter()
                    .take(3)
                    .take_while(|byte| (b'0'..=b'7').contains(byte))
                    .count();
                let value: u32 =
                    u32::from_str_radix(&body[at + 1..at + 1 + digits], 8).unwrap_or(0);
                bytes.push((value & 0xFF) as u8);
                at += 1 + digits;
            }
            b'x' if text.get(at + 2).is_some_and(u8::is_ascii_hexdigit) => {
                let digits: usize = text[at + 2..]
                    .iter()
                    .take(2)
                    .take_while(|byte| byte.is_ascii_hexdigit())
                    .count();
                let value: u32 = hex_value(&body[at + 2..at + 2 + digits]).unwrap_or(0);
                bytes.push((value & 0xFF) as u8);
                at += 2 + digits;
            }
            _ => {
                bytes.push(match next {
                    b'b' => 0x08,
                    b'f' => 0x0c,
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    b'v' => 0x0b,
                    other => other,
                });
                at += 2;
            }
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// A `U&` body with its escapes applied, `escape` being `\` or the `UESCAPE` character.
pub(crate) fn decode_unicode(body: &str, escape: char, position: usize) -> Result<String, Fault> {
    let mut decoded: String = String::with_capacity(body.len());
    let mut rest: &str = body;
    let mut first: Option<u32> = None;
    let invalid = || Fault::syntax("invalid Unicode escape", position);
    let pair = || Fault::syntax("invalid Unicode surrogate pair", position);
    while let Some(character) = rest.chars().next() {
        if character != escape {
            if first.is_some() {
                return Err(pair());
            }
            decoded.push(character);
            rest = &rest[character.len_utf8()..];
            continue;
        }
        let after: &str = &rest[escape.len_utf8()..];
        let (value, length) = if after.starts_with(escape) {
            if first.is_some() {
                return Err(pair());
            }
            decoded.push(escape);
            rest = &after[escape.len_utf8()..];
            continue;
        } else if let Some(value) = after.get(..4).and_then(hex_value) {
            (value, 4)
        } else if let Some(value) = after
            .strip_prefix('+')
            .and_then(|six| six.get(..6))
            .and_then(hex_value)
        {
            (value, 7)
        } else {
            return Err(invalid());
        };
        rest = &after[length..];
        let value: u32 = match (first.take(), value) {
            (Some(high), low) if SECOND_SURROGATE.contains(&low) => surrogate_pair(high, low),
            (Some(_), _) => return Err(pair()),
            (None, high) if FIRST_SURROGATE.contains(&high) => {
                first = Some(high);
                continue;
            }
            (None, low) if SECOND_SURROGATE.contains(&low) => return Err(pair()),
            (None, value) => value,
        };
        decoded.push(code_point(value, position)?);
    }
    match first {
        Some(_) => Err(pair()),
        None => Ok(decoded),
    }
}
