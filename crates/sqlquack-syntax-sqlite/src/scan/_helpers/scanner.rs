//! `sqlite3GetToken`, and `sqlite3RunParser`'s checks of `WINDOW`, `OVER` and `FILTER`.

use sqlquack_syntax::tokens::models::Span;

use crate::keywords::main::keyword::keyword;
use crate::scan::constants::{
    BYTE_ORDER_MARK, DIGIT_SEPARATOR, FIRST_NON_ASCII, UNRECOGNIZED_TOKEN,
};
use crate::scan::models::{NumberKind, Quote, Token, TokenKind};

/// `aiClass`: the character class of each byte.
const CLASSES: [u8; 256] = [
    29, 28, 28, 28, 28, 28, 28, 28, 28, 7, 7, 28, 7, 7, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28,
    28, 28, 28, 28, 28, 28, 28, 7, 15, 8, 5, 4, 22, 24, 8, 17, 18, 21, 20, 23, 11, 26, 16, 3, 3, 3,
    3, 3, 3, 3, 3, 3, 3, 5, 19, 12, 14, 13, 6, 5, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 0, 2, 2, 9, 28, 28, 28, 2, 8, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 0, 2, 2, 28, 10, 28, 25, 28, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
    27, 27, 27, 27, 27, 30, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27, 27,
];

const CC_X: u8 = 0;
const CC_KYWD0: u8 = 1;
const CC_KYWD: u8 = 2;
const CC_DIGIT: u8 = 3;
const CC_DOLLAR: u8 = 4;
const CC_VARALPHA: u8 = 5;
const CC_VARNUM: u8 = 6;
const CC_SPACE: u8 = 7;
const CC_QUOTE: u8 = 8;
const CC_QUOTE2: u8 = 9;
const CC_PIPE: u8 = 10;
const CC_MINUS: u8 = 11;
const CC_LT: u8 = 12;
const CC_GT: u8 = 13;
const CC_EQ: u8 = 14;
const CC_BANG: u8 = 15;
const CC_SLASH: u8 = 16;
const CC_LP: u8 = 17;
const CC_RP: u8 = 18;
const CC_SEMI: u8 = 19;
const CC_PLUS: u8 = 20;
const CC_STAR: u8 = 21;
const CC_PERCENT: u8 = 22;
const CC_COMMA: u8 = 23;
const CC_AND: u8 = 24;
const CC_TILDA: u8 = 25;
const CC_DOT: u8 = 26;
const CC_ID: u8 = 27;
const CC_BOM: u8 = 30;

/// `sqlite3Isspace`: the space characters of `sqlite3CtypeMap`.
fn is_space(byte: u8) -> bool {
    matches!(byte, b'\t' | b'\n' | 0x0B | 0x0C | b'\r' | b' ')
}

/// `IdChar`: a letter, a digit, `_`, `$` or any byte from 0x80.
fn is_id_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$' || byte >= FIRST_NON_ASCII
}

/// One raw token of `sqlite3GetToken`, before the window keywords are settled.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Raw {
    Space,
    Comment,
    Word(Option<&'static str>),
    Quoted(Quote),
    String,
    Number(NumberKind),
    Blob,
    Variable,
    Op(&'static str),
    Illegal,
}

impl Raw {
    /// Whether `getToken` in the window-keyword checks reads it as `TK_ID`.
    fn is_name(self) -> bool {
        match self {
            Self::Word(None) | Self::Quoted(_) | Self::String => true,
            Self::Word(Some(token)) => {
                matches!(token, "JOIN_KW" | "WINDOW" | "OVER") || keyword_fallback(token)
            }
            _ => false,
        }
    }
}

/// Whether the keyword token falls back to `ID`.
fn keyword_fallback(token: &str) -> bool {
    crate::keywords::constants::FALLBACK_TOKENS.contains(&token)
}

pub(crate) struct Scanner<'sql> {
    sql: &'sql str,
    /// The input up to its first NUL byte.
    bytes: &'sql [u8],
}

impl<'sql> Scanner<'sql> {
    pub(crate) fn new(sql: &'sql str) -> Self {
        let bytes: &[u8] = sql.as_bytes();
        let end: usize = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        Self {
            sql,
            bytes: &bytes[..end],
        }
    }

    /// The byte at `index`, or NUL past the end, as the C tokenizer reads its string.
    fn at(&self, index: usize) -> u8 {
        self.bytes.get(index).copied().unwrap_or(0)
    }

    /// Every token, and the ranges of the blanks and comments between them.
    pub(crate) fn scan(&self) -> (Vec<Token>, Vec<(usize, usize)>) {
        let mut tokens: Vec<Token> = Vec::new();
        let mut skipped: Vec<(usize, usize)> = Vec::new();
        let mut position: usize = 0;
        let mut last: Option<Raw> = None;
        while position < self.bytes.len() {
            let (length, raw) = self.raw_token(position);
            let end: usize = position + length;
            if matches!(raw, Raw::Space | Raw::Comment) {
                skipped.push((position, end));
                position = end;
                continue;
            }
            let kind: TokenKind = match raw {
                Raw::Word(Some("WINDOW")) => self.window_keyword(end),
                Raw::Word(Some("OVER")) => self.over_keyword(end, last),
                Raw::Word(Some("FILTER")) => self.filter_keyword(end, last),
                Raw::Word(Some(token)) => TokenKind::Keyword(token),
                Raw::Word(None) => TokenKind::Ident,
                Raw::Quoted(quote) => TokenKind::Quoted(quote),
                Raw::String => TokenKind::String,
                Raw::Number(kind) => TokenKind::Number(kind),
                Raw::Blob => TokenKind::Blob,
                Raw::Variable => TokenKind::Variable,
                Raw::Op(token) => TokenKind::Op(token),
                Raw::Illegal | Raw::Space | Raw::Comment => TokenKind::Fault,
            };
            let text: &str = &self.sql[position..end];
            let kind: TokenKind = match kind {
                TokenKind::Number(NumberKind::Separated) if !separated_ok(text) => {
                    TokenKind::Number(NumberKind::Misseparated)
                }
                _ => kind,
            };
            let value: String = match kind {
                TokenKind::Fault => unrecognized(text),
                _ => token_value(kind, text),
            };
            tokens.push(Token {
                kind,
                value,
                start: position,
                end,
                span: Span::default(),
            });
            last = Some(raw);
            position = end;
        }
        (tokens, skipped)
    }

    /// The raw token after `position`, past blanks and comments, as `getToken` reads it.
    fn next_raw(&self, mut position: usize) -> Option<Raw> {
        while position < self.bytes.len() {
            let (length, raw) = self.raw_token(position);
            if !matches!(raw, Raw::Space | Raw::Comment) {
                return Some(raw);
            }
            position += length;
        }
        None
    }

    /// The raw token after the next one, past blanks and comments.
    fn raw_after_next(&self, mut position: usize) -> Option<Raw> {
        let mut seen: bool = false;
        while position < self.bytes.len() {
            let (length, raw) = self.raw_token(position);
            position += length;
            if matches!(raw, Raw::Space | Raw::Comment) {
                continue;
            }
            if seen {
                return Some(raw);
            }
            seen = true;
        }
        None
    }

    /// `analyzeWindowKeyword`: a keyword when a name and then `AS` follow.
    fn window_keyword(&self, end: usize) -> TokenKind {
        let name: bool = self.next_raw(end).is_some_and(Raw::is_name);
        let then_as: bool = self.raw_after_next(end) == Some(Raw::Word(Some("AS")));
        match name && then_as {
            true => TokenKind::Keyword("WINDOW"),
            false => TokenKind::Ident,
        }
    }

    /// `analyzeOverKeyword`: a keyword after `)` when `(` or a name follows.
    fn over_keyword(&self, end: usize, last: Option<Raw>) -> TokenKind {
        let next: Option<Raw> = self.next_raw(end);
        let follows: bool = next == Some(Raw::Op("LP")) || next.is_some_and(Raw::is_name);
        match last == Some(Raw::Op("RP")) && follows {
            true => TokenKind::Keyword("OVER"),
            false => TokenKind::Ident,
        }
    }

    /// `analyzeFilterKeyword`: a keyword after `)` when `(` follows.
    fn filter_keyword(&self, end: usize, last: Option<Raw>) -> TokenKind {
        match last == Some(Raw::Op("RP")) && self.next_raw(end) == Some(Raw::Op("LP")) {
            true => TokenKind::Keyword("FILTER"),
            false => TokenKind::Ident,
        }
    }

    /// `sqlite3GetToken`: the length and kind of the token starting at `start`.
    fn raw_token(&self, start: usize) -> (usize, Raw) {
        let z = |offset: usize| self.at(start + offset);
        let class: u8 = CLASSES[usize::from(z(0))];
        match class {
            CC_SPACE => {
                let mut i: usize = 1;
                while is_space(z(i)) {
                    i += 1;
                }
                (i, Raw::Space)
            }
            CC_MINUS => match z(1) {
                b'-' => {
                    let mut i: usize = 2;
                    while z(i) != 0 && z(i) != b'\n' {
                        i += 1;
                    }
                    (i, Raw::Comment)
                }
                b'>' => (2 + usize::from(z(2) == b'>'), Raw::Op("PTR")),
                _ => (1, Raw::Op("MINUS")),
            },
            CC_LP => (1, Raw::Op("LP")),
            CC_RP => (1, Raw::Op("RP")),
            CC_SEMI => (1, Raw::Op("SEMI")),
            CC_PLUS => (1, Raw::Op("PLUS")),
            CC_STAR => (1, Raw::Op("STAR")),
            CC_SLASH => {
                if z(1) != b'*' || z(2) == 0 {
                    return (1, Raw::Op("SLASH"));
                }
                let mut i: usize = 3;
                let mut c: u8 = z(2);
                while (c != b'*' || z(i) != b'/') && {
                    c = z(i);
                    c != 0
                } {
                    i += 1;
                }
                if c != 0 {
                    i += 1;
                }
                (i, Raw::Comment)
            }
            CC_PERCENT => (1, Raw::Op("REM")),
            CC_EQ => (1 + usize::from(z(1) == b'='), Raw::Op("EQ")),
            CC_LT => match z(1) {
                b'=' => (2, Raw::Op("LE")),
                b'>' => (2, Raw::Op("NE")),
                b'<' => (2, Raw::Op("LSHIFT")),
                _ => (1, Raw::Op("LT")),
            },
            CC_GT => match z(1) {
                b'=' => (2, Raw::Op("GE")),
                b'>' => (2, Raw::Op("RSHIFT")),
                _ => (1, Raw::Op("GT")),
            },
            CC_BANG => match z(1) {
                b'=' => (2, Raw::Op("NE")),
                _ => (1, Raw::Illegal),
            },
            CC_PIPE => match z(1) {
                b'|' => (2, Raw::Op("CONCAT")),
                _ => (1, Raw::Op("BITOR")),
            },
            CC_COMMA => (1, Raw::Op("COMMA")),
            CC_AND => (1, Raw::Op("BITAND")),
            CC_TILDA => (1, Raw::Op("BITNOT")),
            CC_QUOTE => self.quoted(start),
            CC_DOT if !z(1).is_ascii_digit() => (1, Raw::Op("DOT")),
            CC_DOT | CC_DIGIT => self.number(start),
            CC_QUOTE2 => {
                let mut i: usize = 1;
                let mut c: u8 = z(0);
                while c != b']' && {
                    c = z(i);
                    c != 0
                } {
                    i += 1;
                }
                match c == b']' {
                    true => (i, Raw::Quoted(Quote::Bracket)),
                    false => (i, Raw::Illegal),
                }
            }
            CC_VARNUM => {
                let mut i: usize = 1;
                while z(i).is_ascii_digit() {
                    i += 1;
                }
                (i, Raw::Variable)
            }
            CC_DOLLAR | CC_VARALPHA => self.variable(start),
            CC_KYWD0 => {
                if CLASSES[usize::from(z(1))] > CC_KYWD {
                    return self.identifier(start, 1);
                }
                let mut i: usize = 2;
                while CLASSES[usize::from(z(i))] <= CC_KYWD {
                    i += 1;
                }
                if is_id_char(z(i)) {
                    return self.identifier(start, i + 1);
                }
                let word: &str = &self.sql[start..start + i];
                (i, Raw::Word(keyword(word).map(|found| found.token)))
            }
            CC_X if z(1) == b'\'' => {
                let mut i: usize = 2;
                while z(i).is_ascii_hexdigit() {
                    i += 1;
                }
                let mut raw: Raw = Raw::Blob;
                if z(i) != b'\'' || i % 2 == 1 {
                    raw = Raw::Illegal;
                    while z(i) != 0 && z(i) != b'\'' {
                        i += 1;
                    }
                }
                if z(i) != 0 {
                    i += 1;
                }
                (i, raw)
            }
            CC_X | CC_KYWD | CC_ID => self.identifier(start, 1),
            CC_BOM if z(1) == BYTE_ORDER_MARK[1] && z(2) == BYTE_ORDER_MARK[2] => (3, Raw::Space),
            CC_BOM => self.identifier(start, 1),
            _ => (1, Raw::Illegal),
        }
    }

    /// An identifier from `start`, its first `length` bytes read, continued over `IdChar`s.
    fn identifier(&self, start: usize, mut length: usize) -> (usize, Raw) {
        while is_id_char(self.at(start + length)) {
            length += 1;
        }
        (length, Raw::Word(None))
    }

    /// `'string'`, `"name"` or `` `name` ``; unterminated, it is illegal to the end.
    fn quoted(&self, start: usize) -> (usize, Raw) {
        let z = |offset: usize| self.at(start + offset);
        let delimiter: u8 = z(0);
        let mut i: usize = 1;
        let c: u8 = loop {
            let c: u8 = z(i);
            if c == 0 {
                break c;
            }
            if c == delimiter {
                if z(i + 1) != delimiter {
                    break c;
                }
                i += 1;
            }
            i += 1;
        };
        match (c, delimiter) {
            (b'\'', _) => (i + 1, Raw::String),
            (0, _) => (i, Raw::Illegal),
            (_, b'`') => (i + 1, Raw::Quoted(Quote::Backtick)),
            _ => (i + 1, Raw::Quoted(Quote::Double)),
        }
    }

    /// A decimal or `0x` number; running into an identifier character makes it illegal.
    fn number(&self, start: usize) -> (usize, Raw) {
        let z = |offset: usize| self.at(start + offset);
        let mut kind: NumberKind = NumberKind::Integer;
        let mut i: usize;
        let separated = |kind: &mut NumberKind, byte: u8| {
            let separator: bool = byte == DIGIT_SEPARATOR;
            if separator {
                *kind = NumberKind::Separated;
            }
            separator
        };
        if z(0) == b'0' && matches!(z(1), b'x' | b'X') && z(2).is_ascii_hexdigit() {
            i = 3;
            while z(i).is_ascii_hexdigit() || separated(&mut kind, z(i)) {
                i += 1;
            }
        } else {
            i = 0;
            while z(i).is_ascii_digit() || separated(&mut kind, z(i)) {
                i += 1;
            }
            if z(i) == b'.' {
                if kind == NumberKind::Integer {
                    kind = NumberKind::Float;
                }
                i += 1;
                while z(i).is_ascii_digit() || separated(&mut kind, z(i)) {
                    i += 1;
                }
            }
            let exponent: bool = matches!(z(i), b'e' | b'E')
                && (z(i + 1).is_ascii_digit()
                    || (matches!(z(i + 1), b'+' | b'-') && z(i + 2).is_ascii_digit()));
            if exponent {
                if kind == NumberKind::Integer {
                    kind = NumberKind::Float;
                }
                i += 2;
                while z(i).is_ascii_digit() || separated(&mut kind, z(i)) {
                    i += 1;
                }
            }
        }
        let mut raw: Raw = Raw::Number(kind);
        while is_id_char(z(i)) {
            raw = Raw::Illegal;
            i += 1;
        }
        (i, raw)
    }

    /// `$name`, `@name`, `:name` or `#name`, with Tcl's `(...)` suffix and `::`.
    fn variable(&self, start: usize) -> (usize, Raw) {
        let z = |offset: usize| self.at(start + offset);
        let mut raw: Raw = Raw::Variable;
        let mut named: usize = 0;
        let mut i: usize = 1;
        loop {
            let c: u8 = z(i);
            if c == 0 {
                break;
            }
            if is_id_char(c) {
                named += 1;
            } else if c == b'(' && named > 0 {
                loop {
                    i += 1;
                    let c: u8 = z(i);
                    if c == 0 || is_space(c) || c == b')' {
                        break;
                    }
                }
                if z(i) == b')' {
                    i += 1;
                } else {
                    raw = Raw::Illegal;
                }
                break;
            } else if c == b':' && z(i + 1) == b':' {
                i += 1;
            } else {
                break;
            }
            i += 1;
        }
        if named == 0 {
            raw = Raw::Illegal;
        }
        (i, raw)
    }
}

/// The parser's message for text the tokenizer cannot read.
fn unrecognized(text: &str) -> String {
    format!("{UNRECOGNIZED_TOKEN} \"{text}\"")
}

/// `sqlite3DequoteNumber`'s check: each separator stands between two digits of the number's base.
fn separated_ok(text: &str) -> bool {
    let bytes: &[u8] = text.as_bytes();
    let hex: bool = bytes.len() > 1 && bytes[0] == b'0' && matches!(bytes[1], b'x' | b'X');
    let digit = |byte: Option<&u8>| {
        byte.is_some_and(|byte| match hex {
            true => byte.is_ascii_hexdigit(),
            false => byte.is_ascii_digit(),
        })
    };
    bytes.iter().enumerate().all(|(index, byte)| {
        *byte != DIGIT_SEPARATOR
            || (index > 0 && digit(bytes.get(index - 1)) && digit(bytes.get(index + 1)))
    })
}

/// A token's value: strings and quoted names decoded, separators dropped, blobs their digits.
fn token_value(kind: TokenKind, text: &str) -> String {
    let inner = |open: usize| text.get(open..text.len().saturating_sub(1)).unwrap_or("");
    match kind {
        TokenKind::String => inner(1).replace("''", "'"),
        TokenKind::Quoted(Quote::Double) => inner(1).replace("\"\"", "\""),
        TokenKind::Quoted(Quote::Backtick) => inner(1).replace("``", "`"),
        TokenKind::Quoted(Quote::Bracket) => inner(1).to_owned(),
        TokenKind::Blob => inner(2).to_owned(),
        TokenKind::Number(NumberKind::Separated) => text.replace('_', ""),
        _ => text.to_owned(),
    }
}
