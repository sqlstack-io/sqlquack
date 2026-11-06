//! The scanner: scan.l's rules, longest match first, over the input's bytes.

use sqlquack_syntax::tokens::models::Span;

use crate::keywords::main::keyword::keyword;
use crate::scan::_helpers::strings::{decode_escape_string, decode_unicode, quoted_end};
use crate::scan::constants::{
    FIRST_NON_ASCII, INT4_MAX, NAMEDATALEN, OP_CHARS, OP_KEEPS_SIGN, SINGLE_OPERATORS,
};
use crate::scan::main::digits::digits_value;
use crate::scan::models::{Token, TokenKind};

/// A lexical fault: its message, whether it is a syntax error, and where it is.
pub(crate) struct Fault {
    pub(crate) message: String,
    pub(crate) syntax: bool,
    pub(crate) position: usize,
}

impl Fault {
    pub(crate) fn syntax(message: &str, position: usize) -> Self {
        Self {
            message: message.to_owned(),
            syntax: true,
            position,
        }
    }
}

/// A token before the lookahead filter: U& strings and identifiers are not decoded yet.
pub(crate) struct RawToken {
    pub(crate) token: Token,
    /// The undecoded body of a `U&'...'` or `U&"..."` token.
    pub(crate) unicode: Option<String>,
}

pub(crate) fn ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte >= FIRST_NON_ASCII
}

fn ident_cont(byte: u8) -> bool {
    ident_start(byte) || byte.is_ascii_digit() || byte == b'$'
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn op_char(byte: u8) -> bool {
    OP_CHARS.as_bytes().contains(&byte)
}

/// ASCII-only lower-casing, as `downcase_identifier` does for UTF-8.
fn fold(name: &str) -> String {
    name.to_ascii_lowercase()
}

pub(crate) struct Scanner<'sql> {
    sql: &'sql str,
    bytes: &'sql [u8],
    at: usize,
}

impl<'sql> Scanner<'sql> {
    pub(crate) fn new(sql: &'sql str) -> Self {
        Self {
            sql,
            bytes: sql.as_bytes(),
            at: 0,
        }
    }

    fn byte(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.at + offset).copied()
    }

    fn token(&self, kind: TokenKind, value: String, start: usize) -> RawToken {
        RawToken {
            token: Token {
                kind,
                value,
                start,
                end: self.at,
                span: Span::default(),
            },
            unicode: None,
        }
    }

    /// The first byte before any fault that no token, space or comment covers.
    pub(crate) fn uncovered(self) -> Option<usize> {
        let (sql, bytes) = (self.sql, self.bytes);
        let (raws, fault) = self.scan();
        let mut covered: usize = 0;
        for raw in &raws {
            let mut blank: Scanner<'_> = Scanner {
                sql,
                bytes,
                at: covered,
            };
            if blank.skip_blank().is_err() || blank.at != raw.token.start {
                return Some(covered);
            }
            covered = raw.token.end;
        }
        if fault.is_some() {
            return None;
        }
        let mut blank: Scanner<'_> = Scanner {
            sql,
            bytes,
            at: covered,
        };
        match blank.skip_blank() {
            Ok(()) if blank.at == bytes.len() => None,
            _ => Some(covered),
        }
    }

    /// Every token up to the end of the input or the first fault.
    pub(crate) fn scan(mut self) -> (Vec<RawToken>, Option<Fault>) {
        let mut tokens: Vec<RawToken> = Vec::new();
        loop {
            match self.skip_blank() {
                Err(fault) => return (tokens, Some(fault)),
                Ok(()) if self.at >= self.bytes.len() => return (tokens, None),
                Ok(()) => {}
            }
            match self.next_token() {
                Ok(token) => tokens.push(token),
                Err(fault) => return (tokens, Some(fault)),
            }
        }
    }

    /// Skips whitespace, `--` comments and nested `/* */` comments.
    fn skip_blank(&mut self) -> Result<(), Fault> {
        while let Some(byte) = self.byte(0) {
            if is_space(byte) {
                self.at += 1;
            } else if byte == b'-' && self.byte(1) == Some(b'-') {
                while self
                    .byte(0)
                    .is_some_and(|byte| byte != b'\n' && byte != b'\r')
                {
                    self.at += 1;
                }
            } else if byte == b'/' && self.byte(1) == Some(b'*') {
                self.block_comment()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn block_comment(&mut self) -> Result<(), Fault> {
        let start: usize = self.at;
        let mut depth: usize = 0;
        while self.at < self.bytes.len() {
            match (self.byte(0), self.byte(1)) {
                (Some(b'/'), Some(b'*')) => {
                    depth += 1;
                    self.at += 2;
                }
                (Some(b'*'), Some(b'/')) => {
                    depth -= 1;
                    self.at += 2;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                _ => self.at += 1,
            }
        }
        Err(Fault::syntax("unterminated /* comment", start))
    }

    fn next_token(&mut self) -> Result<RawToken, Fault> {
        let start: usize = self.at;
        let first: u8 = self.byte(0).unwrap_or_default();
        let second: Option<u8> = self.byte(1);
        let prefixed = |kinds: &[u8]| kinds.contains(&first) && second == Some(b'\'');
        if prefixed(b"eE") {
            self.at += 1;
            return self.quoted_string(start, TokenKind::EscapeString);
        }
        if prefixed(b"bB") {
            self.at += 1;
            return self.quoted_string(start, TokenKind::BitString);
        }
        if prefixed(b"xX") {
            self.at += 1;
            return self.quoted_string(start, TokenKind::HexString);
        }
        if prefixed(b"nN") {
            self.at += 1;
            return self.quoted_string(start, TokenKind::String);
        }
        if (first == b'u' || first == b'U')
            && second == Some(b'&')
            && let Some(quote @ (b'\'' | b'"')) = self.byte(2)
        {
            self.at += 2;
            return self.unicode_token(start, quote);
        }
        match first {
            b'\'' => self.quoted_string(start, TokenKind::String),
            b'"' => self.quoted_identifier(start),
            b'$' => self.dollar(start),
            byte if byte.is_ascii_digit() => self.number(start),
            b'.' if second.is_some_and(|byte| byte.is_ascii_digit()) => self.number(start),
            byte if ident_start(byte) => Ok(self.identifier(start)),
            b':' if second == Some(b':') => Ok(self.fixed(start, 2, TokenKind::Typecast)),
            b':' if second == Some(b'=') => Ok(self.fixed(start, 2, TokenKind::ColonEquals)),
            b'.' if second == Some(b'.') => Ok(self.fixed(start, 2, TokenKind::DotDot)),
            byte if op_char(byte) => self.operator(start),
            _ => Ok(self.single(start)),
        }
    }

    fn fixed(&mut self, start: usize, length: usize, kind: TokenKind) -> RawToken {
        self.at += length;
        let value: String = self.sql[start..self.at].to_owned();
        self.token(kind, value, start)
    }

    /// One character as itself, a whole UTF-8 character when it is not ASCII.
    fn single(&mut self, start: usize) -> RawToken {
        let character: char = self.sql[start..].chars().next().unwrap_or_default();
        self.at += character.len_utf8();
        self.token(TokenKind::Char(character), character.to_string(), start)
    }

    fn identifier(&mut self, start: usize) -> RawToken {
        while self.byte(0).is_some_and(ident_cont) {
            self.at += 1;
        }
        let folded: String = fold(&self.sql[start..self.at]);
        let kind: TokenKind = keyword(&folded).map_or(TokenKind::Ident, TokenKind::Keyword);
        self.token(kind, folded, start)
    }

    /// A quoted string from the opening quote at `self.at`, with newline-separated continuations.
    fn quoted_string(&mut self, start: usize, kind: TokenKind) -> Result<RawToken, Fault> {
        let mut value: String = String::new();
        loop {
            let body_start: usize = self.at + 1;
            let escapes: bool = kind == TokenKind::EscapeString;
            let Some(end) = quoted_end(self.bytes, self.at, b'\'', escapes) else {
                let message: &str = match kind {
                    TokenKind::BitString => "unterminated bit string literal",
                    TokenKind::HexString => "unterminated hexadecimal string literal",
                    _ => "unterminated quoted string",
                };
                return Err(Fault::syntax(message, start));
            };
            let body: &str = &self.sql[body_start..end - 1];
            match escapes {
                true => value.push_str(&decode_escape_string(body, body_start)?),
                false => value.push_str(&body.replace("''", "'")),
            }
            self.at = end;
            if !self.continues() {
                break;
            }
        }
        Ok(self.token(kind, value, start))
    }

    /// Moves to the next quote when only whitespace holding a newline, and `--` comments, separate it.
    fn continues(&mut self) -> bool {
        let mut at: usize = self.at;
        let mut newline: bool = false;
        loop {
            match self.bytes.get(at) {
                Some(b'\n' | b'\r') => {
                    newline = true;
                    at += 1;
                }
                Some(&byte) if is_space(byte) => at += 1,
                Some(b'-') if newline && self.bytes.get(at + 1) == Some(&b'-') => {
                    while self
                        .bytes
                        .get(at)
                        .is_some_and(|&byte| byte != b'\n' && byte != b'\r')
                    {
                        at += 1;
                    }
                }
                Some(b'\'') if newline => {
                    self.at = at;
                    return true;
                }
                _ => return false,
            }
        }
    }

    fn quoted_identifier(&mut self, start: usize) -> Result<RawToken, Fault> {
        let Some(end) = quoted_end(self.bytes, self.at, b'"', false) else {
            return Err(Fault::syntax("unterminated quoted identifier", start));
        };
        let name: String = self.sql[start + 1..end - 1].replace("\"\"", "\"");
        if name.is_empty() {
            return Err(Fault::syntax("zero-length delimited identifier", start));
        }
        self.at = end;
        Ok(self.token(TokenKind::QuotedIdent, name, start))
    }

    /// `U&'...'` or `U&"..."`, decoded after the lookahead filter reads any `UESCAPE`.
    fn unicode_token(&mut self, start: usize, quote: u8) -> Result<RawToken, Fault> {
        let mut raw: RawToken = match quote {
            b'"' => {
                let Some(end) = quoted_end(self.bytes, self.at, b'"', false) else {
                    return Err(Fault::syntax("unterminated quoted identifier", start));
                };
                let body: String = self.sql[self.at + 1..end - 1].replace("\"\"", "\"");
                if body.is_empty() {
                    return Err(Fault::syntax("zero-length delimited identifier", start));
                }
                self.at = end;
                self.token(TokenKind::QuotedIdent, body, start)
            }
            _ => self.quoted_string(start, TokenKind::String)?,
        };
        raw.unicode = Some(raw.token.value.clone());
        Ok(raw)
    }

    /// A dollar-quoted string, a parameter, or a lone `$`.
    fn dollar(&mut self, start: usize) -> Result<RawToken, Fault> {
        if self.byte(1).is_some_and(|byte| byte.is_ascii_digit()) {
            return self.parameter(start);
        }
        let mut tag_end: usize = self.at + 1;
        if self
            .bytes
            .get(tag_end)
            .is_some_and(|&byte| ident_start(byte))
        {
            tag_end += 1;
            while self
                .bytes
                .get(tag_end)
                .is_some_and(|&byte| ident_start(byte) || byte.is_ascii_digit())
            {
                tag_end += 1;
            }
        }
        if self.bytes.get(tag_end) != Some(&b'$') {
            return Ok(self.single(start));
        }
        let tag: &str = &self.sql[start..=tag_end];
        let body_start: usize = tag_end + 1;
        let Some(close) = self.sql[body_start..].find(tag) else {
            return Err(Fault::syntax("unterminated dollar-quoted string", start));
        };
        let value: String = self.sql[body_start..body_start + close].to_owned();
        self.at = body_start + close + tag.len();
        Ok(self.token(TokenKind::DollarString, value, start))
    }

    fn parameter(&mut self, start: usize) -> Result<RawToken, Fault> {
        self.at += 1;
        while self.byte(0).is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
        if self.byte(0).is_some_and(ident_start) {
            return Err(Fault::syntax("trailing junk after parameter", start));
        }
        let digits: String = self.sql[start + 1..self.at].to_owned();
        if digits.parse::<i32>().is_err() {
            return Err(Fault::syntax("parameter number too large", start));
        }
        Ok(self.token(TokenKind::Param, digits, start))
    }

    /// Digits and single underscores between digits, from `self.at`; whether any digit was read.
    fn digits(&mut self, radix: u32) -> bool {
        let digit = |byte: Option<u8>| byte.is_some_and(|byte| char::from(byte).is_digit(radix));
        let mut any: bool = false;
        loop {
            if digit(self.byte(0)) {
                self.at += 1;
                any = true;
            } else if any && self.byte(0) == Some(b'_') && digit(self.byte(1)) {
                self.at += 2;
            } else {
                return any;
            }
        }
    }

    fn number(&mut self, start: usize) -> Result<RawToken, Fault> {
        let radix: Option<(u32, &str)> = match (self.byte(0), self.byte(1)) {
            (Some(b'0'), Some(b'x' | b'X')) => Some((16, "invalid hexadecimal integer")),
            (Some(b'0'), Some(b'o' | b'O')) => Some((8, "invalid octal integer")),
            (Some(b'0'), Some(b'b' | b'B')) => Some((2, "invalid binary integer")),
            _ => None,
        };
        if let Some((radix, message)) = radix {
            self.at += 2;
            if self.byte(0) == Some(b'_') {
                self.at += 1;
            }
            if !self.digits(radix) {
                return Err(Fault::syntax(message, start));
            }
            return self.number_end(start, true, radix);
        }
        let mut integer: bool = self.digits(10);
        if self.byte(0) == Some(b'.') && self.byte(1) == Some(b'.') {
            return self.number_end(start, true, 10);
        }
        if self.byte(0) == Some(b'.') {
            self.at += 1;
            integer = false;
            self.digits(10);
        }
        if matches!(self.byte(0), Some(b'e' | b'E')) {
            let sign: usize = usize::from(matches!(self.byte(1), Some(b'+' | b'-')));
            let after: Option<u8> = self.bytes.get(self.at + 1 + sign).copied();
            if after.is_some_and(|byte| byte.is_ascii_digit()) {
                self.at += 1 + sign;
                self.digits(10);
                integer = false;
            } else if sign == 1 {
                return Err(Fault::syntax("trailing junk after numeric literal", start));
            }
        }
        self.number_end(start, integer, 10)
    }

    /// Ends a number: trailing identifier characters are junk; an integer over 32 bits is numeric.
    fn number_end(&mut self, start: usize, integer: bool, radix: u32) -> Result<RawToken, Fault> {
        if self.byte(0).is_some_and(ident_start) {
            return Err(Fault::syntax("trailing junk after numeric literal", start));
        }
        let text: String = self.sql[start..self.at].replace('_', "");
        let digits: &str = match radix {
            10 => &text,
            _ => &text[2..],
        };
        let fits: bool =
            integer && digits_value(digits, radix).is_some_and(|value| value <= INT4_MAX);
        let kind: TokenKind = match fits {
            true => TokenKind::Integer,
            false => TokenKind::Numeric,
        };
        Ok(self.token(kind, text, start))
    }

    /// An operator: the longest run of operator characters, trimmed as scan.l's `{operator}` rule.
    fn operator(&mut self, start: usize) -> Result<RawToken, Fault> {
        let mut end: usize = start;
        while self.bytes.get(end).is_some_and(|&byte| op_char(byte)) {
            end += 1;
        }
        let run: &str = &self.sql[start..end];
        let mut length: usize = run.len();
        for start_of_comment in ["/*", "--"] {
            if let Some(found) = run[1..].find(start_of_comment) {
                length = length.min(found + 1);
            }
        }
        let keeps_sign: bool = run[..length]
            .bytes()
            .any(|byte| OP_KEEPS_SIGN.as_bytes().contains(&byte));
        while length > 1 && !keeps_sign && matches!(run.as_bytes()[length - 1], b'+' | b'-') {
            length -= 1;
        }
        let text: &str = &run[..length];
        self.at = start + length;
        let kind: TokenKind = match text {
            "=>" => TokenKind::EqualsGreater,
            "<=" => TokenKind::LessEquals,
            ">=" => TokenKind::GreaterEquals,
            "<>" | "!=" => TokenKind::NotEquals,
            single if single.len() == 1 && SINGLE_OPERATORS.contains(single) => {
                TokenKind::Char(single.chars().next().unwrap_or_default())
            }
            _ if length >= NAMEDATALEN => return Err(Fault::syntax("operator too long", start)),
            _ => TokenKind::Op,
        };
        Ok(self.token(kind, text.to_owned(), start))
    }
}

/// Decodes a U& token's body with its escape character, after the filter has read any UESCAPE.
pub(crate) fn decode_unicode_token(raw: RawToken, escape: char) -> Result<Token, Fault> {
    let body: String = raw.unicode.unwrap_or_default();
    let mut token: Token = raw.token;
    token.value = decode_unicode(&body, escape, token.start)?;
    Ok(token)
}
