//! Quoted tokens: where they end and what their text decodes to.

/// The byte after the quote closing the token opened at `start`, if it is closed.
pub(crate) fn quoted_end(bytes: &[u8], start: usize, quote: u8, escapes: bool) -> Option<usize> {
    let mut at: usize = start + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if escapes => at += 2,
            byte if byte == quote => {
                if bytes.get(at + 1) == Some(&quote) {
                    at += 2;
                } else {
                    return Some(at + 1);
                }
            }
            _ => at += 1,
        }
    }
    None
}

/// The value of a hexadecimal digit run, when every character is one.
fn hex_value(text: &str) -> Option<char> {
    match u32::from_str_radix(text, 16) {
        Ok(value) => char::from_u32(value),
        Err(_not_hex) => None,
    }
}

/// A string's body with its escapes decoded, as Spark's `unescapeSQLString` reads it.
pub(crate) fn decode_quoted(body: &str, quote: char) -> String {
    let characters: Vec<char> = body.chars().collect();
    let mut decoded: String = String::with_capacity(body.len());
    let mut at: usize = 0;
    while at < characters.len() {
        let character: char = characters[at];
        if character == quote && characters.get(at + 1) == Some(&quote) {
            decoded.push(quote);
            at += 2;
            continue;
        }
        if character != '\\' || at + 1 >= characters.len() {
            decoded.push(character);
            at += 1;
            continue;
        }
        let next: char = characters[at + 1];
        let unicode = |length: usize| -> Option<char> {
            let digits: String = characters.get(at + 2..at + 2 + length)?.iter().collect();
            hex_value(&digits)
        };
        let octal: Option<char> = characters.get(at + 1..at + 4).and_then(|digits| {
            let text: String = digits.iter().collect();
            match digits.iter().all(|digit| ('0'..='7').contains(digit)) {
                true => match u32::from_str_radix(&text, 8) {
                    Ok(value) => char::from_u32(value),
                    Err(_out_of_range) => None,
                },
                false => None,
            }
        });
        let (text, consumed): (String, usize) = match next {
            'u' if unicode(4).is_some() => (unicode(4).map(String::from).unwrap_or_default(), 6),
            'U' if unicode(8).is_some() => (unicode(8).map(String::from).unwrap_or_default(), 10),
            _ if octal.is_some() => (octal.map(String::from).unwrap_or_default(), 4),
            '0' => ("\u{0000}".to_owned(), 2),
            'b' => ("\u{0008}".to_owned(), 2),
            'n' => ("\n".to_owned(), 2),
            'r' => ("\r".to_owned(), 2),
            't' => ("\t".to_owned(), 2),
            'Z' => ("\u{001A}".to_owned(), 2),
            '%' => ("\\%".to_owned(), 2),
            '_' => ("\\_".to_owned(), 2),
            other => (other.to_string(), 2),
        };
        decoded.push_str(&text);
        at += consumed;
    }
    decoded
}
