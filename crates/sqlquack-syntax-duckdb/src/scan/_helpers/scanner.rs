//! The scanner: DuckDB 2.0.0's PEG tokenizer (`base_tokenizer.cpp`, `parser_tokenizer.cpp`).

use sqlquack_syntax::tokens::models::Span;

use crate::scan::_helpers::strings::{decode_escaped, undouble};
use crate::scan::constants::{
    EMPTY_IDENTIFIER, FIRST_NON_ASCII, OPERATOR_CHARACTERS, OPERATOR_SPECIALS,
    SINGLE_BYTE_OPERATORS, STRING_PREFIXES, UNTERMINATED_COMMENT, UNTERMINATED_IDENTIFIER,
    UNTERMINATED_STRING,
};
use crate::scan::models::{StringPrefix, Token, TokenKind};

/// A lexical fault, always a syntax error: its message, where, and whether it precedes parsing.
pub(crate) struct Fault {
    pub(crate) message: String,
    pub(crate) position: usize,
    pub(crate) eager: bool,
}

/// `StringUtil::CharacterIsSpace`.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r')
}

/// `StringUtil::CharacterIsOperator`: the ASCII punctuation but `_`.
fn is_broad_operator(byte: u8) -> bool {
    byte != b'_'
        && (matches!(byte, b'!'..=b'/')
            || matches!(byte, b':'..=b'@')
            || matches!(byte, b'['..=b'`')
            || matches!(byte, b'{'..=b'~'))
}

/// `Tokenizer::CharacterIsOperator`: the characters that continue an operator.
fn is_operator(byte: u8) -> bool {
    OPERATOR_CHARACTERS.contains(&byte)
}

/// `Tokenizer::CharacterIsKeyword`: a byte that continues a word.
fn is_word_byte(byte: u8) -> bool {
    !(SINGLE_BYTE_OPERATORS.contains(&byte)
        || is_broad_operator(byte)
        || is_space(byte)
        || matches!(byte, b'\'' | b';' | b'"' | b'.'))
}

/// `IsValidDollarTagCharacter`.
fn is_tag_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= FIRST_NON_ASCII
}

pub(crate) struct Scanner<'sql> {
    sql: &'sql str,
    bytes: &'sql [u8],
    at: usize,
    /// The ranges no token covers but a blank or comment does.
    pub(crate) skipped: Vec<(usize, usize)>,
}

impl<'sql> Scanner<'sql> {
    pub(crate) fn new(sql: &'sql str) -> Self {
        Self {
            sql,
            bytes: sql.as_bytes(),
            at: 0,
            skipped: Vec::new(),
        }
    }

    fn byte(&self, position: usize) -> Option<u8> {
        self.bytes.get(position).copied()
    }

    fn token(&self, kind: TokenKind, value: String, start: usize, end: usize) -> Token {
        Token {
            kind,
            value,
            start,
            end,
            span: Span::default(),
        }
    }

    /// Every token up to the end or the first fault.
    pub(crate) fn scan(mut self) -> (Vec<Token>, Option<Fault>, Vec<(usize, usize)>) {
        let mut tokens: Vec<Token> = Vec::new();
        loop {
            if let Err(fault) = self.skip_blanks() {
                return (tokens, Some(fault), self.skipped);
            }
            if self.at >= self.bytes.len() {
                return (tokens, None, self.skipped);
            }
            match self.next_tokens() {
                Ok(next) => tokens.extend(next),
                Err(fault) => return (tokens, Some(fault), self.skipped),
            }
        }
    }

    /// Skips blanks, `--` comments to the end of the line and nested `/* */` comments.
    fn skip_blanks(&mut self) -> Result<(), Fault> {
        loop {
            let start: usize = self.at;
            match self.byte(start) {
                Some(byte) if is_space(byte) => self.at += 1,
                Some(b'-') if self.byte(start + 1) == Some(b'-') => {
                    self.at += 2;
                    while let Some(byte) = self.byte(self.at) {
                        self.at += 1;
                        if byte == b'\n' || byte == b'\r' {
                            break;
                        }
                    }
                }
                Some(b'/') if self.byte(start + 1) == Some(b'*') => self.comment(start)?,
                _ => return Ok(()),
            }
            self.skipped.push((start, self.at));
        }
    }

    fn comment(&mut self, start: usize) -> Result<(), Fault> {
        let mut depth: usize = 1;
        self.at = start + 2;
        while self.at < self.bytes.len() {
            if self.byte(self.at) == Some(b'/') && self.byte(self.at + 1) == Some(b'*') {
                depth += 1;
                self.at += 2;
            } else if self.byte(self.at) == Some(b'*') && self.byte(self.at + 1) == Some(b'/') {
                depth -= 1;
                self.at += 2;
                if depth == 0 {
                    return Ok(());
                }
            } else {
                self.at += 1;
            }
        }
        Err(Fault {
            message: format!("{UNTERMINATED_COMMENT} \"{}\"", &self.sql[start..]),
            position: start,
            eager: true,
        })
    }

    /// The token or tokens at `self.at`: an operator may split into several.
    fn next_tokens(&mut self) -> Result<Vec<Token>, Fault> {
        let start: usize = self.at;
        let byte: u8 = self.bytes[start];
        let next: Option<u8> = self.byte(start + 1);
        if byte == b'\'' {
            return Ok(vec![self.string(start, start, StringPrefix::Plain)?]);
        }
        if byte == b'"' || byte == b'`' {
            return Ok(vec![self.quoted(start, byte)?]);
        }
        if byte == b';' {
            self.at += 1;
            return Ok(vec![self.token(
                TokenKind::Char(';'),
                ";".to_owned(),
                start,
                self.at,
            )]);
        }
        if byte == b'$' {
            return Ok(vec![self.dollar(start)?]);
        }
        if byte == b':' && matches!(next, Some(b':' | b'=')) {
            let length: usize = match self.byte(start + 2).is_some_and(is_operator) {
                true => 1,
                false => 2,
            };
            self.at += length;
            let text: String = self.sql[start..self.at].to_owned();
            let kind: TokenKind = match length {
                1 => TokenKind::Char(':'),
                _ => TokenKind::Op,
            };
            return Ok(vec![self.token(kind, text, start, self.at)]);
        }
        if SINGLE_BYTE_OPERATORS.contains(&byte) {
            self.at += 1;
            return Ok(vec![self.token(
                TokenKind::Char(byte as char),
                (byte as char).to_string(),
                start,
                self.at,
            )]);
        }
        if byte.is_ascii_digit() || byte == b'.' {
            return Ok(self.number(start));
        }
        if STRING_PREFIXES.contains(&byte) && next == Some(b'\'') {
            let prefix: StringPrefix = match byte.to_ascii_uppercase() {
                b'E' => StringPrefix::Escape,
                b'N' => StringPrefix::National,
                b'X' => StringPrefix::Hex,
                _ => StringPrefix::Bit,
            };
            return Ok(vec![self.string(start, start + 1, prefix)?]);
        }
        if is_broad_operator(byte) {
            return Ok(self.operator(start));
        }
        self.at = start + 1;
        while self
            .byte(self.at)
            .is_some_and(|byte| byte == b'$' || is_word_byte(byte))
        {
            self.at += 1;
        }
        let text: String = self.sql[start..self.at].to_owned();
        Ok(vec![self.token(TokenKind::Word, text, start, self.at)])
    }

    /// A string whose opening quote is at `quote`; only `E'...'` reads backslash escapes.
    fn string(&mut self, start: usize, quote: usize, prefix: StringPrefix) -> Result<Token, Fault> {
        let escapes: bool = prefix == StringPrefix::Escape;
        let mut at: usize = quote + 1;
        while at < self.bytes.len() {
            match self.bytes[at] {
                b'\\' if escapes && at + 1 < self.bytes.len() => at += 2,
                b'\'' if self.byte(at + 1) == Some(b'\'') => at += 2,
                b'\'' => {
                    self.at = at + 1;
                    let body: &str = &self.sql[quote + 1..at];
                    let value: String = match prefix {
                        StringPrefix::Escape => decode_escaped(body),
                        _ => undouble(body, '\''),
                    };
                    return Ok(self.token(TokenKind::String(prefix), value, start, self.at));
                }
                _ => at += 1,
            }
        }
        Err(Fault {
            message: UNTERMINATED_STRING.to_owned(),
            position: start,
            eager: true,
        })
    }

    fn quoted(&mut self, start: usize, delimiter: u8) -> Result<Token, Fault> {
        let mut at: usize = start + 1;
        while at < self.bytes.len() {
            if self.bytes[at] == delimiter {
                if self.byte(at + 1) == Some(delimiter) {
                    at += 2;
                    continue;
                }
                if at == start + 1 {
                    return Err(Fault {
                        message: EMPTY_IDENTIFIER.to_owned(),
                        position: start,
                        eager: true,
                    });
                }
                self.at = at + 1;
                let value: String = undouble(&self.sql[start + 1..at], delimiter as char);
                return Ok(self.token(TokenKind::Quoted, value, start, self.at));
            }
            at += 1;
        }
        Err(Fault {
            message: UNTERMINATED_IDENTIFIER.to_owned(),
            position: start,
            eager: true,
        })
    }

    /// `$tag$...$tag$`, or `$` alone before a digit or a name (a parameter).
    fn dollar(&mut self, start: usize) -> Result<Token, Fault> {
        let lone = |scanner: &mut Self| {
            scanner.at = start + 1;
            scanner.token(TokenKind::Char('$'), "$".to_owned(), start, start + 1)
        };
        if self
            .byte(start + 1)
            .is_none_or(|byte| byte.is_ascii_digit())
        {
            return Ok(lone(self));
        }
        let mut at: usize = start + 1;
        while self.byte(at).is_some_and(is_tag_byte) {
            at += 1;
        }
        if self.byte(at) != Some(b'$') {
            return Ok(lone(self));
        }
        let marker: &str = &self.sql[start..=at];
        let body: usize = at + 1;
        match self.sql[body..].find(marker) {
            Some(length) => {
                self.at = body + length + marker.len();
                let value: String = self.sql[body..body + length].to_owned();
                Ok(self.token(TokenKind::DollarString, value, start, self.at))
            }
            None => Err(Fault {
                message: format!("Syntax error at or near '{}'", &self.sql[start..]),
                position: start,
                eager: false,
            }),
        }
    }

    /// A number, giving back a trailing `e` or sign; points alone are one `.` token each.
    fn number(&mut self, start: usize) -> Vec<Token> {
        let bytes: &[u8] = self.bytes;
        let mut at: usize = start + 1;
        while at < bytes.len() {
            let byte: u8 = bytes[at];
            let previous: u8 = bytes[at - 1];
            let continues: bool = byte.is_ascii_digit()
                || byte == b'.'
                || (byte == b'_'
                    && previous.is_ascii_digit()
                    && bytes.get(at + 1).is_some_and(u8::is_ascii_digit))
                || (matches!(byte, b'e' | b'E')
                    && !matches!(previous, b'e' | b'E')
                    && (bytes[start].is_ascii_digit() || previous.is_ascii_digit()))
                || (matches!(byte, b'+' | b'-') && matches!(previous, b'e' | b'E'));
            if !continues {
                break;
            }
            at += 1;
        }
        while !(bytes[at - 1].is_ascii_digit() || bytes[at - 1] == b'.') {
            at -= 1;
        }
        self.at = at;
        if bytes[start..at].iter().all(|byte| *byte == b'.') {
            return (start..at)
                .map(|position| {
                    self.token(TokenKind::Char('.'), ".".to_owned(), position, position + 1)
                })
                .collect();
        }
        let text: String = self.sql[start..at].to_owned();
        vec![self.token(TokenKind::Number, text, start, at)]
    }

    /// An operator run before any `--` or `/*`, giving back trailing signs unless it has a special.
    fn operator(&mut self, start: usize) -> Vec<Token> {
        let mut at: usize = start + 1;
        while let Some(byte) = self.byte(at) {
            let comment: bool = (byte == b'-' && self.byte(at + 1) == Some(b'-'))
                || (byte == b'/' && self.byte(at + 1) == Some(b'*'));
            if comment || !is_operator(byte) {
                break;
            }
            at += 1;
        }
        let mut end: usize = at;
        let special: bool = self.bytes[start..at]
            .iter()
            .any(|byte| OPERATOR_SPECIALS.contains(byte));
        if !special {
            while end > start + 1 && matches!(self.bytes[end - 1], b'+' | b'-') {
                end -= 1;
            }
        }
        let text: String = self.sql[start..end].to_owned();
        let mut tokens: Vec<Token> = vec![self.token(TokenKind::Op, text, start, end)];
        tokens.extend((end..at).map(|position| {
            let sign: String = self.sql[position..=position].to_owned();
            self.token(TokenKind::Op, sign, position, position + 1)
        }));
        self.at = at;
        tokens
    }
}
