//! The parser's state: the tokens, where it is, and how tokens read as names.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::keywords::main::is_reserved::is_reserved;
use crate::parse::constants::MAX_DEPTH;
use crate::parse::errors::ParseError;
use crate::parse::models::{Gap, StatementRange};
use crate::scan::models::{Token, TokenKind};

pub(crate) type Parsing<T> = Result<T, ParseError>;

pub(crate) struct Parser<'sql> {
    pub(crate) sql: &'sql str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) at: usize,
    depth: usize,
    pub(crate) gaps: Vec<Gap>,
    /// Where each statement read so far was written, in source order.
    pub(crate) ranges: Vec<StatementRange>,
    /// How many statements enclose the one being read.
    nesting: usize,
    /// Token positions of a `(` already refused as a subquery, so backtracking tries each reading once.
    pub(crate) not_subquery: std::collections::HashSet<usize>,
}

/// Whether `token` is a name: a delimited identifier, or a word that is not reserved.
pub(crate) fn is_name(token: &Token) -> bool {
    match token.kind {
        TokenKind::Quoted(_) => true,
        TokenKind::Word => !is_reserved(&token.value) && !token.value.starts_with('$'),
        _ => false,
    }
}

/// Whether `token` is any word or delimited identifier, reserved or not.
pub(crate) fn is_any_name(token: &Token) -> bool {
    matches!(token.kind, TokenKind::Quoted(_) | TokenKind::Word)
}

/// A name token as an identifier, with its source range; a delimited name is marked quoted.
pub(crate) fn identifier(token: &Token) -> Identifier {
    match token.kind {
        TokenKind::Quoted(_) => Identifier::quoted(token.value.clone()),
        _ => Identifier::new(token.value.clone()),
    }
    .with_span(token.span)
}

/// Whether `token` is the unquoted word `name`, compared without case.
pub(crate) fn is_word(token: &Token, name: &str) -> bool {
    token.kind == TokenKind::Word && token.value.eq_ignore_ascii_case(name)
}

impl<'sql> Parser<'sql> {
    pub(crate) fn new(sql: &'sql str, tokens: Vec<Token>) -> Self {
        Self {
            sql,
            tokens,
            at: 0,
            depth: 0,
            gaps: Vec::new(),
            ranges: Vec::new(),
            nesting: 0,
            not_subquery: std::collections::HashSet::new(),
        }
    }

    pub(crate) fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.at + offset)
    }

    pub(crate) fn peek(&self) -> Option<&Token> {
        self.peek_at(0)
    }

    pub(crate) fn kw_at(&self, offset: usize, name: &str) -> bool {
        self.peek_at(offset)
            .is_some_and(|token| is_word(token, name))
    }

    pub(crate) fn kw(&self, name: &str) -> bool {
        self.kw_at(0, name)
    }

    pub(crate) fn kw_in(&self, offset: usize, names: &[&str]) -> bool {
        names.iter().any(|name| self.kw_at(offset, name))
    }

    /// The upper-cased word `offset` tokens ahead, if that token is an unquoted word.
    pub(crate) fn word_at(&self, offset: usize) -> Option<String> {
        self.peek_at(offset)
            .filter(|token| token.kind == TokenKind::Word)
            .map(|token| token.value.to_ascii_uppercase())
    }

    pub(crate) fn char_at(&self, offset: usize, character: char) -> bool {
        self.peek_at(offset)
            .is_some_and(|token| token.kind == TokenKind::Char(character))
    }

    pub(crate) fn is_char(&self, character: char) -> bool {
        self.char_at(0, character)
    }

    pub(crate) fn op_at(&self, offset: usize, text: &str) -> bool {
        self.peek_at(offset)
            .is_some_and(|token| token.kind == TokenKind::Op && token.value == text)
    }

    pub(crate) fn is_op(&self, text: &str) -> bool {
        self.op_at(0, text)
    }

    pub(crate) fn is(&self, test: fn(&Token) -> bool) -> bool {
        self.peek().is_some_and(test)
    }

    pub(crate) fn is_at(&self, offset: usize, test: fn(&Token) -> bool) -> bool {
        self.peek_at(offset).is_some_and(test)
    }

    /// Whether the input ends here.
    pub(crate) fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    pub(crate) fn advance(&mut self) -> Parsing<Token> {
        match self.peek() {
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
        let found: bool = names
            .iter()
            .enumerate()
            .all(|(offset, name)| self.kw_at(offset, name));
        if found {
            self.at += names.len();
        }
        found
    }

    pub(crate) fn eat_char(&mut self, character: char) -> bool {
        let found: bool = self.is_char(character);
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
        for name in names {
            self.expect_kw(name)?;
        }
        Ok(())
    }

    pub(crate) fn expect_char(&mut self, character: char) -> Parsing<()> {
        match self.eat_char(character) {
            true => Ok(()),
            false => Err(self.error()),
        }
    }

    /// The current token if it satisfies `test`, consumed; a syntax error otherwise.
    pub(crate) fn expect_token(&mut self, test: fn(&Token) -> bool) -> Parsing<Token> {
        match self.is(test) {
            true => self.advance(),
            false => Err(self.error()),
        }
    }

    /// The syntax error at the current token, worded as the engine's error 102.
    pub(crate) fn error(&self) -> ParseError {
        match self.peek() {
            None => ParseError {
                message: "Incorrect syntax near the end of the input.".to_owned(),
                position: self.sql.len(),
                syntax: true,
            },
            Some(token) => ParseError {
                message: format!(
                    "Incorrect syntax near '{}'.",
                    &self.sql[token.start..token.end]
                ),
                position: token.start,
                syntax: true,
            },
        }
    }

    /// An error with its own message, at the current token.
    pub(crate) fn error_with(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_owned(),
            position: self.offset(),
            syntax: true,
        }
    }

    /// Byte offset of the current token, or the input's end.
    pub(crate) fn offset(&self) -> usize {
        self.peek().map_or(self.sql.len(), |token| token.start)
    }

    /// Byte offset just past the last consumed token.
    pub(crate) fn last_end(&self) -> usize {
        self.at
            .checked_sub(1)
            .and_then(|last| self.tokens.get(last))
            .map_or(0, |token| token.end)
    }

    /// Records a gap from `start` and stops: the rest of the input is one opaque statement.
    pub(crate) fn gap(&mut self, construct: &'static str, start: usize) -> ParseError {
        self.gaps.push(Gap {
            construct,
            start,
            end: self.sql.len(),
        });
        self.at = self.tokens.len();
        ParseError {
            message: String::new(),
            position: start,
            syntax: false,
        }
    }

    /// Runs `parse` one level deeper, failing past the depth limit.
    pub(crate) fn deeper<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Parsing<T> {
        if self.depth >= MAX_DEPTH {
            let start: usize = self.offset();
            return Err(self.gap("nesting_depth", start));
        }
        self.depth += 1;
        let red_zone: usize = match cfg!(debug_assertions) {
            true => 4 * 1024 * 1024,
            false => 1024 * 1024,
        };
        let result: Parsing<T> = stacker::maybe_grow(red_zone, 8 * 1024 * 1024, || parse(self));
        self.depth -= 1;
        result
    }

    /// Counts one more link of a left-nested chain toward the depth limit.
    pub(crate) fn link(&mut self) -> Parsing<()> {
        if self.depth >= MAX_DEPTH {
            let start: usize = self.offset();
            return Err(self.gap("nesting_depth", start));
        }
        self.depth += 1;
        Ok(())
    }

    /// The nesting depth, to restore once a chain counted with `link` ends.
    pub(crate) fn depth(&self) -> usize {
        self.depth
    }

    /// Restores the nesting depth a chain started at.
    pub(crate) fn restore_depth(&mut self, depth: usize) {
        self.depth = depth;
    }

    /// Tries `parse`; on failure rewinds and returns the error. A gap is never rewound.
    pub(crate) fn attempt<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Parsing<T>,
    ) -> Result<T, ParseError> {
        let (at, gaps, depth) = (self.at, self.gaps.len(), self.depth);
        let (ranges, nesting) = (self.ranges.len(), self.nesting);
        parse(self).inspect_err(|_| {
            if self.gaps.len() == gaps {
                self.at = at;
                self.depth = depth;
                self.ranges.truncate(ranges);
                self.nesting = nesting;
            }
        })
    }

    /// Runs `parse` for the statement at the next token, recording where it was written and reading the statements inside it one level deeper.
    pub(crate) fn ranged(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Parsing<Expression>,
    ) -> Parsing<Expression> {
        let slot: usize = self.ranges.len();
        let start: usize = self.offset();
        self.ranges.push(StatementRange {
            start,
            end: start,
            depth: self.nesting,
        });
        self.nesting += 1;
        let parsed: Parsing<Expression> = parse(self);
        self.nesting -= 1;
        let mut look: usize = 0;
        while self.char_at(look, ';') {
            look += 1;
        }
        let end: usize = match look {
            0 => self.last_end(),
            _ => self
                .peek_at(look - 1)
                .map_or(self.sql.len(), |token| token.end),
        };
        if let Some(range) = self.ranges.get_mut(slot) {
            range.end = end;
        }
        parsed
    }

    /// Whether a gap has been recorded, so parsing has stopped.
    pub(crate) fn gapped(&self) -> bool {
        !self.gaps.is_empty()
    }

    /// `( item, ... )` with at least one item.
    pub(crate) fn parenthesized<T>(
        &mut self,
        item: impl FnMut(&mut Self) -> Parsing<T>,
    ) -> Parsing<Vec<T>> {
        self.expect_char('(')?;
        let items: Vec<T> = self.comma_list(item)?;
        self.expect_char(')')?;
        Ok(items)
    }

    /// One or more items separated by commas.
    pub(crate) fn comma_list<T>(
        &mut self,
        mut item: impl FnMut(&mut Self) -> Parsing<T>,
    ) -> Parsing<Vec<T>> {
        let mut items: Vec<T> = vec![item(self)?];
        while self.eat_char(',') {
            items.push(item(self)?);
        }
        Ok(items)
    }

    /// A name that is not reserved, as an identifier.
    pub(crate) fn name(&mut self) -> Parsing<Identifier> {
        self.expect_token(is_name).map(|token| identifier(&token))
    }

    /// A dotted name of up to `limit` parts; `a..b` leaves the middle part empty.
    pub(crate) fn multipart(&mut self, limit: usize) -> Parsing<Vec<Identifier>> {
        let mut parts: Vec<Identifier> = vec![self.name()?];
        while self.is_char('.') && parts.len() < limit {
            self.at += 1;
            if self.is_char('.') && parts.len() + 1 < limit {
                parts.push(Identifier::empty());
                continue;
            }
            parts.push(self.name()?);
        }
        Ok(parts)
    }
}
