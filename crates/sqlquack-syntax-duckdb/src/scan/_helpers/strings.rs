//! Quoted text: what a string or quoted identifier's body decodes to.

/// A body with each doubled `quote` read as one.
pub(crate) fn undouble(body: &str, quote: char) -> String {
    let doubled: String = [quote, quote].iter().collect();
    body.replace(&doubled, &quote.to_string())
}

/// The character a run of digits of `radix` names, when every digit is one and it names one.
fn digits_char(digits: &[char], radix: u32) -> Option<char> {
    digits
        .iter()
        .try_fold(0_u32, |value, digit| {
            Some(value.checked_mul(radix)? + digit.to_digit(radix)?)
        })
        .and_then(char::from_u32)
}

/// An `E'...'` body with doubled quotes and backslash escapes decoded.
pub(crate) fn decode_escaped(body: &str) -> String {
    let characters: Vec<char> = body.chars().collect();
    let mut decoded: String = String::with_capacity(body.len());
    let mut at: usize = 0;
    while at < characters.len() {
        let character: char = characters[at];
        if character == '\'' && characters.get(at + 1) == Some(&'\'') {
            decoded.push('\'');
            at += 2;
            continue;
        }
        if character != '\\' || at + 1 >= characters.len() {
            decoded.push(character);
            at += 1;
            continue;
        }
        let hex = |length: usize| -> Option<char> {
            characters
                .get(at + 2..at + 2 + length)
                .and_then(|digits| digits_char(digits, 16))
        };
        let octal: Option<char> = characters
            .get(at + 1..at + 4)
            .and_then(|digits| digits_char(digits, 8));
        let (value, consumed): (Option<char>, usize) = match characters[at + 1] {
            'x' if hex(2).is_some() => (hex(2), 4),
            'u' if hex(4).is_some() => (hex(4), 6),
            'U' if hex(8).is_some() => (hex(8), 10),
            _ if octal.is_some() => (octal, 4),
            'b' => (Some('\u{0008}'), 2),
            'f' => (Some('\u{000C}'), 2),
            'n' => (Some('\n'), 2),
            'r' => (Some('\r'), 2),
            't' => (Some('\t'), 2),
            other => (Some(other), 2),
        };
        decoded.extend(value);
        at += consumed;
    }
    decoded
}
