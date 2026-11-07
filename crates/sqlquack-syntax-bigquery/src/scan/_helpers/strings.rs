//! Quoted tokens: where they end and what their text decodes to.

/// The byte after the closing quote of the literal whose quote run (one or three) starts at `start`.
pub(crate) fn quoted_end(bytes: &[u8], start: usize, quote: u8, triple: bool) -> Option<usize> {
    let width: usize = if triple { 3 } else { 1 };
    let mut at: usize = start + width;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'\n' | b'\r' if !triple => return None,
            byte if byte == quote => {
                if !triple {
                    return Some(at + 1);
                }
                if bytes.get(at + 1) == Some(&quote) && bytes.get(at + 2) == Some(&quote) {
                    return Some(at + 3);
                }
                at += 1;
            }
            _ => at += 1,
        }
    }
    None
}

/// The value of a run of hexadecimal or octal digits, when every character is one (a sign is not).
fn digits_value(characters: &[char], radix: u32) -> Option<char> {
    if !characters.iter().all(|character| character.is_digit(radix)) {
        return None;
    }
    let text: String = characters.iter().collect();
    match u32::from_str_radix(&text, radix) {
        Ok(value) => char::from_u32(value),
        Err(_out_of_range) => None,
    }
}

/// A source character's units in a bytes literal's value: one character per UTF-8 byte.
fn character_byte_units(character: char) -> impl Iterator<Item = char> {
    let mut buffer: [u8; 4] = [0; 4];
    let length: usize = character.encode_utf8(&mut buffer).len();
    buffer.into_iter().take(length).map(char::from)
}

/// A raw bytes literal's body, one character per byte as `decode_quoted` gives bytes.
pub(crate) fn byte_units(body: &str) -> String {
    let mut decoded: String = String::with_capacity(body.len());
    for character in body.chars() {
        decoded.extend(character_byte_units(character));
    }
    decoded
}

/// A literal's body decoded as `UnescapeString` (or, with `bytes`, `UnescapeBytes`); `None` if refused.
pub(crate) fn decode_quoted(body: &str, bytes: bool) -> Option<String> {
    let characters: Vec<char> = body.chars().collect();
    let mut decoded: String = String::with_capacity(body.len());
    let mut at: usize = 0;
    while at < characters.len() {
        let character: char = characters[at];
        if character != '\\' {
            match bytes {
                true => decoded.extend(character_byte_units(character)),
                false => decoded.push(character),
            }
            at += 1;
            continue;
        }
        let next: char = *characters.get(at + 1)?;
        let (text, consumed): (char, usize) = match next {
            'a' => ('\u{0007}', 2),
            'b' => ('\u{0008}', 2),
            'f' => ('\u{000C}', 2),
            'n' => ('\n', 2),
            'r' => ('\r', 2),
            't' => ('\t', 2),
            'v' => ('\u{000B}', 2),
            '\\' | '?' | '"' | '\'' | '`' => (next, 2),
            'x' | 'X' => (digits_value(characters.get(at + 2..at + 4)?, 16)?, 4),
            'u' if !bytes => (digits_value(characters.get(at + 2..at + 6)?, 16)?, 6),
            'U' if !bytes => (digits_value(characters.get(at + 2..at + 10)?, 16)?, 10),
            '0'..='3' => (digits_value(characters.get(at + 1..at + 4)?, 8)?, 4),
            _ => return None,
        };
        decoded.push(text);
        at += consumed;
    }
    Some(decoded)
}
