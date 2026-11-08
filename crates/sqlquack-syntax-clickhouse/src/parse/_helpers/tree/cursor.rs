//! The parser's state: the significant tokens, where it is, and how tokens read as names.

use sqlquack_syntax::Gap;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::parse::constants::{MAX_DEPTH, STACK_RED_ZONE, STACK_SEGMENT};
use crate::parse::errors::ParseError;
use crate::scan::main::unquote::unquote;
use crate::scan::models::{Token, TokenKind};

pub(crate) type Parsing<T> = Result<T, ParseError>;

pub(crate) struct Parser<'sql> {
    pub(crate) sql: &'sql str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) at: usize,
    depth: usize,
    pub(crate) gaps: Vec<Gap>,
    /// Whether the statement being parsed holds a gap, so it becomes a `Command`.
    pub(crate) opaque: bool,
    /// The depth limit's error, once reached; no backtracking undoes it.
    pub(crate) too_deep: Option<ParseError>,
}

/// Whether `token` is the unquoted word `name`, compared without case.
pub(crate) fn is_word(token: &Token, name: &str) -> bool {
    token.kind == TokenKind::BareWord && token.value.eq_ignore_ascii_case(name)
}

/// `ParserIdentifier`: an unquoted word, or a quoted name that is not empty.
pub(crate) fn is_identifier(token: &Token) -> bool {
    match token.kind {
        TokenKind::BareWord => true,
        TokenKind::QuotedIdentifier => !unquote(&token.value).is_empty(),
        _ => false,
    }
}

/// A name token as an identifier; a quoted name is marked so.
pub(crate) fn identifier(token: &Token) -> Identifier {
    match token.kind {
        TokenKind::QuotedIdentifier => Identifier::quoted(unquote(&token.value)),
        _ => Identifier::new(token.value.clone()),
    }
    .with_span(token.span)
}

/// A parser position to rewind to.
#[derive(Clone, Copy)]
pub(crate) struct Mark {
    at: usize,
    gaps: usize,
    opaque: bool,
    depth: usize,
}

impl<'sql> Parser<'sql> {
    pub(crate) fn new(sql: &'sql str, tokens: Vec<Token>) -> Self {
        Self {
            sql,
            tokens,
            at: 0,
            depth: 0,
            gaps: Vec::new(),
            opaque: false,
            too_deep: None,
        }
    }

    pub(crate) fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.at + offset)
    }

    pub(crate) fn peek(&self) -> Option<&Token> {
        self.peek_at(0)
    }

    /// Whether the token `offset` ahead is the word `name`.
    pub(crate) fn kw_at(&self, offset: usize, name: &str) -> bool {
        self.peek_at(offset)
            .is_some_and(|token| is_word(token, name))
    }

    pub(crate) fn kw(&self, name: &str) -> bool {
        self.kw_at(0, name)
    }

    /// Whether the token `offset` ahead is one of the words `names`.
    pub(crate) fn kw_in(&self, offset: usize, names: &[&str]) -> bool {
        names.iter().any(|name| self.kw_at(offset, name))
    }

    /// Whether the words `names` come next, in order.
    pub(crate) fn kws(&self, names: &[&str]) -> bool {
        names
            .iter()
            .enumerate()
            .all(|(offset, name)| self.kw_at(offset, name))
    }

    /// The upper-cased word `offset` tokens ahead, if that token is an unquoted word.
    pub(crate) fn word_at(&self, offset: usize) -> Option<String> {
        self.peek_at(offset)
            .filter(|token| token.kind == TokenKind::BareWord)
            .map(|token| token.value.to_ascii_uppercase())
    }

    /// Whether the token `offset` ahead is of `kind`.
    pub(crate) fn kind_at(&self, offset: usize, kind: TokenKind) -> bool {
        self.peek_at(offset).is_some_and(|token| token.kind == kind)
    }

    pub(crate) fn is(&self, kind: TokenKind) -> bool {
        self.kind_at(0, kind)
    }

    /// Whether the token `offset` ahead satisfies `test`.
    pub(crate) fn is_at(&self, offset: usize, test: fn(&Token) -> bool) -> bool {
        self.peek_at(offset).is_some_and(test)
    }

    /// Whether the input or the statement ends here.
    pub(crate) fn at_end(&self) -> bool {
        self.peek()
            .is_none_or(|token| token.kind == TokenKind::Semicolon)
    }

    pub(crate) fn advance(&mut self) -> Parsing<Token> {
        match self.peek() {
            Some(token) if token.kind.is_error() => Err(self.error()),
            Some(token) => {
                let token: Token = token.clone();
                self.at += 1;
                Ok(token)
            }
            None => Err(self.error()),
        }
    }

    pub(crate) fn eat_kw(&mut self, name: &str) -> bool {
        let found: bool = self.kw(name);
        self.at += usize::from(found);
        found
    }

    /// Consumes the words `names` when they all come next, in order.
    pub(crate) fn eat_kws(&mut self, names: &[&str]) -> bool {
        let found: bool = self.kws(names);
        if found {
            self.at += names.len();
        }
        found
    }

    pub(crate) fn eat(&mut self, kind: TokenKind) -> bool {
        let found: bool = self.is(kind);
        self.at += usize::from(found);
        found
    }

    pub(crate) fn expect_kw(&mut self, name: &str) -> Parsing<()> {
        match self.eat_kw(name) {
            true => Ok(()),
            false => Err(self.error()),
        }
    }

    pub(crate) fn expect_kws(&mut self, names: &[&str]) -> Parsing<()> {
        match self.eat_kws(names) {
            true => Ok(()),
            false => Err(self.error()),
        }
    }

    pub(crate) fn expect_kind(&mut self, kind: TokenKind) -> Parsing<()> {
        match self.eat(kind) {
            true => Ok(()),
            false => Err(self.error()),
        }
    }

    /// The syntax error at the current token, or the lexical error it is.
    pub(crate) fn error(&self) -> ParseError {
        match self.peek() {
            None => ParseError {
                message: "Syntax error: failed at end of query".to_owned(),
                position: self.sql.len(),
                syntax: true,
            },
            Some(token) if token.kind.is_error() => ParseError {
                message: format!("{}: Syntax error", token.kind.error_description()),
                position: token.start,
                syntax: true,
            },
            Some(token) => ParseError {
                message: format!(
                    "Syntax error: failed at position {} ({})",
                    token.start + 1,
                    token.value
                ),
                position: token.start,
                syntax: true,
            },
        }
    }

    /// An error the parser's own checks raise, with their own message.
    pub(crate) fn error_with(&self, message: &str, syntax: bool) -> ParseError {
        ParseError {
            message: message.to_owned(),
            position: self.offset(),
            syntax,
        }
    }

    /// Byte offset of the current token, or the input's end.
    pub(crate) fn offset(&self) -> usize {
        self.peek().map_or(self.sql.len(), |token| token.start)
    }

    /// Records a construct the shared tree cannot hold, from byte `start` to here.
    pub(crate) fn gap(&mut self, construct: &'static str, start: usize) {
        self.opaque = true;
        let end: usize = self
            .at
            .checked_sub(1)
            .and_then(|last| self.tokens.get(last))
            .map_or(start, |token| token.end);
        self.gaps.push(Gap {
            construct,
            start,
            end: end.max(start),
        });
    }

    /// Runs `parse` one level deeper, failing past the depth limit.
    pub(crate) fn deeper<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Parsing<T> {
        self.within_depth(0)?;
        self.depth += 1;
        let result: Parsing<T> = stacker::maybe_grow(STACK_RED_ZONE, STACK_SEGMENT, || parse(self));
        self.depth -= 1;
        result
    }

    /// Fails past the depth limit with `links` chained levels (postfixes, set operations) added.
    pub(crate) fn within_depth(&mut self, links: usize) -> Parsing<()> {
        if self.depth + links < MAX_DEPTH {
            return Ok(());
        }
        let error: ParseError = self.error_with("Maximum parse depth exceeded", false);
        self.too_deep.get_or_insert_with(|| error.clone());
        Err(error)
    }

    /// Tries `parse`; on failure rewinds, as ClickHouse's parsers do, and returns the error.
    pub(crate) fn attempt<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Parsing<T>,
    ) -> Result<T, ParseError> {
        let mark: Mark = self.mark();
        parse(self).inspect_err(|_| self.rewind(mark))
    }

    /// `parse`'s result, or `None` rewound; a depth refusal is kept in `too_deep` either way.
    pub(crate) fn optional<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Option<T> {
        let mark: Mark = self.mark();
        match parse(self) {
            Ok(value) => Some(value),
            Err(_) => {
                self.rewind(mark);
                None
            }
        }
    }

    /// Where the parser is now.
    fn mark(&self) -> Mark {
        Mark {
            at: self.at,
            gaps: self.gaps.len(),
            opaque: self.opaque,
            depth: self.depth,
        }
    }

    /// Returns the parser to `mark`, dropping gaps recorded since.
    fn rewind(&mut self, mark: Mark) {
        self.at = mark.at;
        self.gaps.truncate(mark.gaps);
        self.opaque = mark.opaque;
        self.depth = mark.depth;
    }

    /// Whether `parse` reads here, rewinding when it does not.
    pub(crate) fn parses<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> bool {
        self.optional(parse).is_some()
    }

    /// Skips to the `;` ending the statement at this depth; a lexical error stops it.
    pub(crate) fn skip_statement_rest(&mut self) -> Parsing<()> {
        let mut depth: usize = 0;
        while let Some(token) = self.peek() {
            match token.kind {
                kind if kind.is_error() => return Err(self.error()),
                TokenKind::Semicolon if depth == 0 => return Ok(()),
                TokenKind::OpeningRoundBracket
                | TokenKind::OpeningSquareBracket
                | TokenKind::OpeningCurlyBrace => depth += 1,
                TokenKind::ClosingRoundBracket
                | TokenKind::ClosingSquareBracket
                | TokenKind::ClosingCurlyBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.at += 1;
        }
        Ok(())
    }

    /// `item, ...` with at least one item.
    pub(crate) fn comma_list<T>(
        &mut self,
        mut item: impl FnMut(&mut Self) -> Parsing<T>,
    ) -> Parsing<Vec<T>> {
        let mut items: Vec<T> = vec![item(self)?];
        while self.eat(TokenKind::Comma) {
            items.push(item(self)?);
        }
        Ok(items)
    }

    /// `ParserIdentifier`: a name.
    pub(crate) fn ident(&mut self) -> Parsing<Identifier> {
        match self.is_at(0, is_identifier) {
            true => self.advance().map(|token| identifier(&token)),
            false => Err(self.error()),
        }
    }

    /// `ParserCompoundIdentifier`: `name ( . name )*`.
    pub(crate) fn compound_identifier(&mut self) -> Parsing<Vec<Identifier>> {
        let mut parts: Vec<Identifier> = vec![self.ident()?];
        while self.is(TokenKind::Dot) && self.is_at(1, is_identifier) {
            self.at += 1;
            parts.push(self.ident()?);
        }
        Ok(parts)
    }

    /// Whether a string literal is next.
    pub(crate) fn string_next(&self) -> bool {
        self.is(TokenKind::StringLiteral)
    }

    /// A string literal's value, consumed.
    pub(crate) fn string_literal(&mut self) -> Parsing<String> {
        match self.string_next() {
            true => self.advance().map(|token| unquote(&token.value)),
            false => Err(self.error()),
        }
    }
}
