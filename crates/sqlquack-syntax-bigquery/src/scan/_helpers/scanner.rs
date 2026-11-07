//! The scanner: googlesql.tm's lexer rules, longest match first, over the input's bytes.

use sqlquack_syntax::tokens::models::Span;

use crate::keywords::main::is_reserved::is_reserved;
use crate::scan::_helpers::strings::{byte_units, decode_quoted, quoted_end};
use crate::scan::constants::{
    ATTACHED_ALIAS, MULTI_CHAR_OPERATORS, UNCLOSED_COMMENT, UNICODE_BLANKS,
};
use crate::scan::models::{NumberKind, Token, TokenKind};

/// A lexical fault: its message and where it is. Every lexical fault is a syntax error.
pub(crate) struct Fault {
    pub(crate) message: String,
    pub(crate) position: usize,
}

/// `unquoted_identifier`'s characters: ASCII letters, digits and `_`.
pub(crate) fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_blank(character: char) -> bool {
    matches!(
        character,
        ' ' | '\t' | '\n' | '\r' | '\u{0008}' | '\u{000B}' | '\u{000C}'
    ) || UNICODE_BLANKS.contains(&character)
}

pub(crate) struct Scanner<'sql> {
    sql: &'sql str,
    bytes: &'sql [u8],
    at: usize,
    /// The ranges no token covers but a blank or comment does.
    pub(crate) skipped: Vec<(usize, usize)>,
}

/// The end of the word bytes from `from`.
fn word_end_from(bytes: &[u8], from: usize) -> usize {
    from + bytes[from..]
        .iter()
        .take_while(|byte| is_word_byte(**byte))
        .count()
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

    fn byte_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.at + offset).copied()
    }

    fn char_at(&self, position: usize) -> Option<char> {
        self.sql
            .get(position..)
            .and_then(|rest| rest.chars().next())
    }

    fn token(&self, kind: TokenKind, value: String, start: usize) -> Token {
        Token {
            kind,
            value,
            start,
            end: self.at,
            span: Span::default(),
        }
    }

    fn fault(message: &str, position: usize) -> Fault {
        Fault {
            message: message.to_owned(),
            position,
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
            if let Some(path_tokens) = self.path_number(&tokens) {
                tokens.extend(path_tokens);
                continue;
            }
            match self.next_token() {
                Ok(token) => tokens.push(token),
                Err(fault) => return (tokens, Some(fault), self.skipped),
            }
        }
    }

    /// The tokens of digits a path continues with (`foo.bar.25x`), which GoogleSQL reads as names.
    fn path_number(&mut self, tokens: &[Token]) -> Option<Vec<Token>> {
        let start: usize = self.at;
        let names = |token: &Token| match token.kind {
            TokenKind::Quoted | TokenKind::Char(')' | ']') => true,
            TokenKind::Word => !is_reserved(&token.value),
            _ => false,
        };
        if self.byte_at(0).is_some_and(|byte| byte.is_ascii_digit())
            && matches!(
                tokens,
                [.., before, dot] if dot.kind == TokenKind::Char('.') && names(before)
            )
        {
            let end: usize = word_end_from(self.bytes, start);
            self.at = end;
            return Some(vec![self.token(
                TokenKind::Quoted,
                self.sql[start..end].to_owned(),
                start,
            )]);
        }
        if self.byte_at(0) == Some(b'.')
            && self.byte_at(1).is_some_and(|byte| byte.is_ascii_digit())
            && tokens.last().is_some_and(names)
        {
            self.at += 1;
            let dot: Token = self.token(TokenKind::Char('.'), ".".to_owned(), start);
            let end: usize = word_end_from(self.bytes, self.at);
            let name_start: usize = self.at;
            self.at = end;
            let name: Token = self.token(
                TokenKind::Quoted,
                self.sql[name_start..end].to_owned(),
                name_start,
            );
            return Some(vec![dot, name]);
        }
        None
    }

    /// Skips the rest of a line, as `dash_comment` and `pound_comment` do.
    fn skip_line(&mut self) {
        while let Some(byte) = self.byte_at(0) {
            if byte == b'\r' || byte == b'\n' {
                break;
            }
            self.at += 1;
        }
    }

    /// Skips blanks and comments (`whitespace_no_comments`, `comment`).
    fn skip_blanks(&mut self) -> Result<(), Fault> {
        loop {
            let start: usize = self.at;
            match self.char_at(self.at) {
                Some(character) if is_blank(character) => self.at += character.len_utf8(),
                Some('#') => self.skip_line(),
                Some('-') if self.byte_at(1) == Some(b'-') => self.skip_line(),
                Some('/') if self.byte_at(1) == Some(b'*') => {
                    match self.sql[start + 2..].find("*/") {
                        Some(length) => self.at = start + 2 + length + 2,
                        None => return Err(Self::fault(UNCLOSED_COMMENT, start)),
                    }
                }
                _ => return Ok(()),
            }
            self.skipped.push((start, self.at));
        }
    }

    fn next_token(&mut self) -> Result<Token, Fault> {
        let start: usize = self.at;
        let Some(character) = self.char_at(start) else {
            return Err(Self::fault("Syntax error: Illegal input character", start));
        };
        if let Some((prefix, raw, bytes)) = self.literal_prefix() {
            return self.string(start, prefix, raw, bytes);
        }
        if character == '\'' || character == '"' {
            return self.string(start, 0, false, false);
        }
        if character == '`' {
            return self.quoted_identifier(start);
        }
        if character.is_ascii_digit()
            || (character == '.' && self.byte_at(1).is_some_and(|byte| byte.is_ascii_digit()))
        {
            return self.number(start);
        }
        if character.is_ascii_alphabetic() || character == '_' {
            while self.byte_at(0).is_some_and(is_word_byte) {
                self.at += 1;
            }
            return Ok(self.token(TokenKind::Word, self.sql[start..self.at].to_owned(), start));
        }
        for operator in MULTI_CHAR_OPERATORS {
            if self.sql[start..].starts_with(operator) {
                self.at += operator.len();
                return Ok(self.token(TokenKind::Op, operator.to_owned(), start));
            }
        }
        if !character.is_ascii() {
            return Err(Self::fault("Syntax error: Illegal input character", start));
        }
        self.at += 1;
        Ok(self.token(TokenKind::Char(character), character.to_string(), start))
    }

    /// The `r`, `b`, `rb` or `br` literal prefix at the cursor: its length, and whether raw and bytes.
    fn literal_prefix(&self) -> Option<(usize, bool, bool)> {
        let first: u8 = self.byte_at(0)?.to_ascii_lowercase();
        let second: Option<u8> = self.byte_at(1).map(|byte| byte.to_ascii_lowercase());
        let quote_at = |offset: usize| matches!(self.byte_at(offset), Some(b'\'' | b'"'));
        match (first, second) {
            (b'r', Some(b'b')) | (b'b', Some(b'r')) if quote_at(2) => Some((2, true, true)),
            (b'r', _) if quote_at(1) => Some((1, true, false)),
            (b'b', _) if quote_at(1) => Some((1, false, true)),
            _ => None,
        }
    }

    /// A string or bytes literal, one- or three-quoted, after a prefix `prefix` bytes long.
    fn string(
        &mut self,
        start: usize,
        prefix: usize,
        raw: bool,
        bytes: bool,
    ) -> Result<Token, Fault> {
        let open: usize = start + prefix;
        let quote: u8 = self.bytes[open];
        let triple: bool =
            self.bytes.get(open + 1) == Some(&quote) && self.bytes.get(open + 2) == Some(&quote);
        let kind: &str = match (raw, bytes) {
            (false, false) => "string literal",
            (true, false) => "raw string literal",
            (false, true) => "bytes literal",
            (true, true) => "raw bytes literal",
        };
        let Some(end) = quoted_end(self.bytes, open, quote, triple) else {
            let message: String = match triple {
                true => format!("Syntax error: Unclosed triple-quoted {kind}"),
                false => format!("Syntax error: Unclosed {kind}"),
            };
            return Err(Fault {
                message,
                position: start,
            });
        };
        let width: usize = if triple { 3 } else { 1 };
        let body: &str = &self.sql[open + width..end - width];
        let value: String = match raw {
            true if bytes => byte_units(body),
            true => body.to_owned(),
            false => decode_quoted(body, bytes)
                .ok_or_else(|| Self::fault("Syntax error: Illegal escape sequence", start))?,
        };
        self.at = end;
        Ok(self.token(TokenKind::String { raw, bytes }, value, start))
    }

    /// `` `name` ``: escapes as in strings; it may not hold a newline or be empty.
    fn quoted_identifier(&mut self, start: usize) -> Result<Token, Fault> {
        let Some(end) = quoted_end(self.bytes, start, b'`', false) else {
            return Err(Self::fault(
                "Syntax error: Unclosed identifier literal",
                start,
            ));
        };
        let value: String = decode_quoted(&self.sql[start + 1..end - 1], false)
            .ok_or_else(|| Self::fault("Syntax error: Illegal escape sequence", start))?;
        if value.is_empty() {
            return Err(Self::fault("Syntax error: Invalid empty identifier", start));
        }
        self.at = end;
        Ok(self.token(TokenKind::Quoted, value, start))
    }

    /// A decimal, hexadecimal or floating-point literal; a name straight after it is a fault.
    fn number(&mut self, start: usize) -> Result<Token, Fault> {
        let bytes: &[u8] = self.bytes;
        let digits_from = |position: usize, test: fn(&u8) -> bool| -> usize {
            bytes[position.min(bytes.len())..]
                .iter()
                .take_while(|byte| test(byte))
                .count()
        };
        let hex: bool = bytes[start] == b'0'
            && matches!(bytes.get(start + 1), Some(b'x' | b'X'))
            && bytes.get(start + 2).is_some_and(u8::is_ascii_hexdigit);
        let mut named: bool = false;
        let (end, kind): (usize, NumberKind) = if hex {
            (
                start + 2 + digits_from(start + 2, u8::is_ascii_hexdigit),
                NumberKind::Hex,
            )
        } else {
            let mut end: usize = start + digits_from(start, u8::is_ascii_digit);
            let mut kind: NumberKind = NumberKind::Integer;
            let dot: usize = end;
            named = bytes.get(dot) == Some(&b'.')
                && bytes
                    .get(dot + 1)
                    .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_');
            if bytes.get(end) == Some(&b'.') {
                end += 1 + digits_from(end + 1, u8::is_ascii_digit);
                kind = NumberKind::Float;
            }
            if matches!(bytes.get(end), Some(b'e' | b'E')) {
                let signed: usize =
                    end + 1 + usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
                let count: usize = digits_from(signed, u8::is_ascii_digit);
                if count > 0 {
                    end = signed + count;
                    kind = NumberKind::Float;
                }
            }
            named &= bytes.get(end).is_some_and(|byte| is_word_byte(*byte));
            if named {
                end = dot + 1;
            }
            (end, kind)
        };
        if !named && bytes.get(end).is_some_and(|byte| is_word_byte(*byte)) {
            return Err(Self::fault(ATTACHED_ALIAS, end));
        }
        self.at = end;
        Ok(self.token(
            TokenKind::Number(kind),
            self.sql[start..end].to_owned(),
            start,
        ))
    }
}
