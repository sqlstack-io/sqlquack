//! Whether a JSON literal is one `json_checked_v1`'s fast check reads as it is, checked in Rust.

use std::collections::HashSet;

/// The deepest nesting checked; deeper text keeps the macro call.
const DEEPEST: usize = 64;
/// The most digits of a whole number the fast check clears, so no number leaves 64 bits.
const WHOLE_DIGITS: usize = 18;
/// The most digits before a decimal point the fast check clears.
const INTEGER_DIGITS: usize = 9;
/// The most digits after a decimal point the fast check clears.
const FRACTION_DIGITS: usize = 5;

/// Whether `text` is JSON `json_checked_v1`'s fast check reads as it is, read more strictly: no key escaped or repeated.
pub(crate) fn plain_json(text: &str) -> bool {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        at: 0,
    };
    reader.blank();
    let read = reader.value(0);
    reader.blank();
    read && reader.at == reader.bytes.len()
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn taken(&mut self, byte: u8) -> bool {
        let found = self.peek() == Some(byte);
        self.at += usize::from(found);
        found
    }

    fn blank(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn value(&mut self, depth: usize) -> bool {
        match self.peek() {
            _ if depth > DEEPEST => false,
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().is_some(),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => ["true", "false", "null"].iter().any(|word| self.word(word)),
            None => false,
        }
    }

    fn word(&mut self, word: &str) -> bool {
        let found = self.bytes[self.at..].starts_with(word.as_bytes());
        self.at += if found { word.len() } else { 0 };
        found
    }

    fn object(&mut self, depth: usize) -> bool {
        self.at += 1;
        self.blank();
        if self.taken(b'}') {
            return true;
        }
        let mut keys: HashSet<&'a [u8]> = HashSet::new();
        loop {
            self.blank();
            let Some(key) = self.string() else {
                return false;
            };
            if key.contains(&b'\\') || !keys.insert(key) {
                return false;
            }
            self.blank();
            if !self.taken(b':') {
                return false;
            }
            self.blank();
            if !self.value(depth + 1) {
                return false;
            }
            self.blank();
            if self.taken(b'}') {
                return true;
            }
            if !self.taken(b',') {
                return false;
            }
        }
    }

    fn array(&mut self, depth: usize) -> bool {
        self.at += 1;
        self.blank();
        if self.taken(b']') {
            return true;
        }
        loop {
            self.blank();
            if !self.value(depth + 1) {
                return false;
            }
            self.blank();
            if self.taken(b']') {
                return true;
            }
            if !self.taken(b',') {
                return false;
            }
        }
    }

    /// A string without a raw control character, an escaped backslash or a UTF-16 surrogate escape; its bytes inside the quotes.
    fn string(&mut self) -> Option<&'a [u8]> {
        if !self.taken(b'"') {
            return None;
        }
        let start = self.at;
        loop {
            let byte = self.peek()?;
            self.at += 1;
            match byte {
                b'"' => return Some(&self.bytes[start..self.at - 1]),
                0x00..=0x1f => return None,
                b'\\' => {
                    let escaped = self.peek()?;
                    self.at += 1;
                    match escaped {
                        b'"' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                        b'u' => {
                            let digits = self.bytes.get(self.at..self.at + 4)?;
                            let surrogate = matches!(digits[0], b'd' | b'D')
                                && !matches!(digits[1], b'0'..=b'7');
                            if surrogate || !digits.iter().all(u8::is_ascii_hexdigit) {
                                return None;
                            }
                            self.at += 4;
                        }
                        _ => return None,
                    }
                }
                _ => {}
            }
        }
    }

    fn number(&mut self) -> bool {
        let start = self.at;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.at += 1;
        }
        plain_number(&self.bytes[start..self.at])
    }
}

/// A whole number of up to 18 digits but -0, or a decimal of up to 9 + 5 digits not all zero after the point.
fn plain_number(token: &[u8]) -> bool {
    let unsigned = token.strip_prefix(b"-").unwrap_or(token);
    let digits = |part: &[u8]| !part.is_empty() && part.iter().all(u8::is_ascii_digit);
    let leading = |part: &[u8], most: usize| {
        digits(part) && part.len() <= most && (part == b"0" || part[0] != b'0')
    };
    match unsigned.iter().position(|byte| *byte == b'.') {
        None if unsigned == b"0" => unsigned.len() == token.len(),
        None => leading(unsigned, WHOLE_DIGITS) && unsigned[0] != b'0',
        Some(point) => {
            let (whole, fraction) = (&unsigned[..point], &unsigned[point + 1..]);
            leading(whole, INTEGER_DIGITS)
                && digits(fraction)
                && fraction.len() <= FRACTION_DIGITS
                && fraction.iter().any(|byte| *byte != b'0')
        }
    }
}
