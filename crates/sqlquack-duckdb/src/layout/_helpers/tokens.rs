//! The tokens of DuckDB SQL, for a layout that changes only the whitespace between them.

/// What a token is, as far as the layout needs to know.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// A bare word: a keyword, a function name or an unquoted name or number.
    Word,
    /// A string literal or a quoted name, kept whole.
    Quoted,
    /// A `--` comment, which ends its line.
    LineComment,
    /// A `/* */` comment.
    BlockComment,
    /// One character of punctuation or an operator.
    Symbol,
}

/// One token of the SQL and whether whitespace came before it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Token<'sql> {
    /// The token's text, exactly as written.
    pub(crate) text: &'sql str,
    /// What kind of token it is.
    pub(crate) kind: TokenKind,
    /// Whether whitespace separated it from the token before.
    pub(crate) spaced: bool,
}

impl Token<'_> {
    /// Whether the token is the bare word `word`, in any case.
    pub(crate) fn is(&self, word: &str) -> bool {
        self.kind == TokenKind::Word && self.text.eq_ignore_ascii_case(word)
    }

    /// Whether the token is the symbol `symbol`.
    pub(crate) fn is_symbol(&self, symbol: &str) -> bool {
        self.kind == TokenKind::Symbol && self.text == symbol
    }
}

/// The tokens of `sql`; strings, quoted names and comments stay single tokens.
pub(crate) fn tokens(sql: &str) -> Vec<Token<'_>> {
    let bytes = sql.as_bytes();
    let mut tokens: Vec<Token<'_>> = Vec::new();
    let (mut index, mut spaced) = (0, false);
    while let Some(&byte) = bytes.get(index) {
        if byte.is_ascii_whitespace() {
            spaced = true;
            index += 1;
            continue;
        }
        let (end, kind) = match (byte, bytes.get(index + 1).copied()) {
            (b'\'' | b'"', _) => (quoted_end(bytes, index), TokenKind::Quoted),
            (b'-', Some(b'-')) => (line_end(bytes, index), TokenKind::LineComment),
            (b'/', Some(b'*')) => (comment_end(bytes, index), TokenKind::BlockComment),
            (byte, _) if is_word_byte(byte) => (word_end(bytes, index), TokenKind::Word),
            _ => (symbol_end(sql, index), TokenKind::Symbol),
        };
        tokens.push(Token {
            text: &sql[index..end],
            kind,
            spaced,
        });
        (index, spaced) = (end, false);
    }
    tokens
}

/// Whether `byte` continues a bare word; bytes of non-ASCII characters count as word bytes.
fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || !byte.is_ascii()
}

/// The index past the quoted text opening at `start`, where a doubled quote is part of the text.
fn quoted_end(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut index = start + 1;
    while let Some(&byte) = bytes.get(index) {
        match (byte == quote, bytes.get(index + 1) == Some(&quote)) {
            (true, true) => index += 2,
            (true, false) => return index + 1,
            (false, _) => index += 1,
        }
    }
    index
}

/// The index of the line break ending the `--` comment at `start`, or the end of the SQL.
fn line_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |offset| start + offset)
}

/// The index past the `*/` closing the comment at `start`, or the end of the SQL.
fn comment_end(bytes: &[u8], start: usize) -> usize {
    bytes[start + 2..]
        .windows(2)
        .position(|pair| pair == b"*/")
        .map_or(bytes.len(), |offset| start + 2 + offset + 2)
}

/// The index past the bare word at `start`.
fn word_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| !is_word_byte(*byte))
        .map_or(bytes.len(), |offset| start + offset)
}

/// The index past the one character at `start`.
fn symbol_end(sql: &str, start: usize) -> usize {
    sql[start..]
        .chars()
        .next()
        .map_or(sql.len(), |character| start + character.len_utf8())
}
