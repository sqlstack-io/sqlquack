//! The parser's state: the tokens, where it is, and which tokens read as names where.

use sqlquack_syntax::Gap;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::keywords::constants::FALLBACK_TOKENS;
use crate::parse::constants::{
    INCOMPLETE_INPUT, MAX_DEPTH, STACK_RED_ZONE, STACK_SEGMENT, TOO_DEEP,
};
use crate::parse::errors::ParseError;
use crate::scan::models::{Token, TokenKind};

pub(crate) type Parsing<T> = Result<T, ParseError>;

/// `ID` after `%fallback`: an unquoted name, a quoted name, or a keyword that falls back to it.
pub(crate) fn is_id(token: &Token) -> bool {
    match token.kind {
        TokenKind::Ident | TokenKind::Quoted(_) => true,
        TokenKind::Keyword(keyword) => FALLBACK_TOKENS.contains(&keyword),
        _ => false,
    }
}

/// `ids`: `ID` or a string.
pub(crate) fn is_ids(token: &Token) -> bool {
    is_id(token) || token.kind == TokenKind::String
}

/// `idj`: `ID`, `INDEXED` or a join keyword.
pub(crate) fn is_idj(token: &Token) -> bool {
    is_id(token) || matches!(token.kind, TokenKind::Keyword("INDEXED" | "JOIN_KW"))
}

/// `nm`: `idj` or a string.
pub(crate) fn is_nm(token: &Token) -> bool {
    is_idj(token) || token.kind == TokenKind::String
}

/// A name token as an identifier; a quoted name or a string used as a name is marked quoted.
pub(crate) fn identifier(token: &Token) -> Identifier {
    match token.kind {
        TokenKind::Quoted(_) | TokenKind::String => Identifier::quoted(token.value.clone()),
        _ => Identifier::new(token.value.clone()),
    }
    .with_span(token.span)
}

pub(crate) struct Parser<'sql> {
    pub(crate) sql: &'sql str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) at: usize,
    depth: usize,
    pub(crate) gaps: Vec<Gap>,
    /// Whether the statement being parsed holds a gap, so it becomes a `Command`.
    pub(crate) opaque: bool,
    /// An action's error SQLite raises once the token at this index, which triggered the action, is taken.
    pub(crate) pending: Option<(ParseError, usize)>,
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
            pending: None,
        }
    }

    pub(crate) fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.at + offset)
    }

    pub(crate) fn peek(&self) -> Option<&Token> {
        self.peek_at(0)
    }

    /// The kind of the token `offset` ahead, if there is one.
    pub(crate) fn kind_at(&self, offset: usize) -> Option<TokenKind> {
        self.peek_at(offset).map(|token| token.kind)
    }

    /// Whether the token `offset` ahead is the keyword whose grammar token is `token`.
    pub(crate) fn kw_at(&self, offset: usize, token: &str) -> bool {
        matches!(self.kind_at(offset), Some(TokenKind::Keyword(found)) if found == token)
    }

    pub(crate) fn kw(&self, token: &str) -> bool {
        self.kw_at(0, token)
    }

    /// Whether the token `offset` ahead is the punctuation or operator `token`.
    pub(crate) fn op_at(&self, offset: usize, token: &str) -> bool {
        matches!(self.kind_at(offset), Some(TokenKind::Op(found)) if found == token)
    }

    pub(crate) fn op(&self, token: &str) -> bool {
        self.op_at(0, token)
    }

    /// Whether the current token satisfies `test`.
    pub(crate) fn is(&self, test: fn(&Token) -> bool) -> bool {
        self.peek().is_some_and(test)
    }

    /// Whether the statement ends here: the input's end or `;`.
    pub(crate) fn at_end(&self) -> bool {
        self.peek().is_none() || self.op("SEMI")
    }

    pub(crate) fn advance(&mut self) -> Parsing<Token> {
        match self.peek() {
            Some(token) if token.kind != TokenKind::Fault => {
                let token: Token = token.clone();
                self.at += 1;
                Ok(token)
            }
            _ => Err(self.error()),
        }
    }

    pub(crate) fn eat_kw(&mut self, token: &str) -> bool {
        let found: bool = self.kw(token);
        self.at += usize::from(found);
        found
    }

    pub(crate) fn eat_op(&mut self, token: &str) -> bool {
        let found: bool = self.op(token);
        self.at += usize::from(found);
        found
    }

    pub(crate) fn expect_kw(&mut self, token: &str) -> Parsing<()> {
        match self.eat_kw(token) {
            true => Ok(()),
            false => Err(self.error()),
        }
    }

    pub(crate) fn expect_op(&mut self, token: &str) -> Parsing<()> {
        match self.eat_op(token) {
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

    /// A name `test` accepts, as an identifier.
    pub(crate) fn name(&mut self, test: fn(&Token) -> bool) -> Parsing<Identifier> {
        self.expect_token(test).map(|token| identifier(&token))
    }

    /// The error SQLite reports at the current token.
    pub(crate) fn error(&self) -> ParseError {
        match self.peek() {
            None => self.error_with(INCOMPLETE_INPUT, self.sql.len(), true),
            Some(token) if token.kind == TokenKind::Fault => {
                self.error_with(&token.value, token.start, true)
            }
            Some(token) => self.error_with(
                &format!(
                    "near \"{}\": syntax error",
                    &self.sql[token.start..token.end]
                ),
                token.start,
                true,
            ),
        }
    }

    /// Holds an action's error until the token at the cursor is taken, as SQLite reduces then shifts.
    pub(crate) fn defer(&mut self, error: ParseError) {
        if self.pending.is_none() {
            self.pending = Some((error, self.at));
        }
    }

    /// An error with its own message: a grammar action's check or a limit.
    pub(crate) fn error_with(&self, message: &str, position: usize, syntax: bool) -> ParseError {
        ParseError {
            message: message.to_owned(),
            position,
            syntax,
        }
    }

    /// Byte offset of the current token, or the input's end.
    pub(crate) fn offset(&self) -> usize {
        self.peek().map_or(self.sql.len(), |token| token.start)
    }

    /// The source text of tokens `first` up to the cursor.
    pub(crate) fn text_from(&self, first: usize) -> &'sql str {
        match (self.tokens.get(first), self.at.checked_sub(1)) {
            (Some(from), Some(last)) if last >= first => {
                &self.sql[from.start..self.tokens[last].end]
            }
            _ => "",
        }
    }

    /// Records a construct the shared tree cannot hold, from `start` to here.
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

    /// Runs `parse` one level deeper, failing past the depth limit, growing the stack when low.
    pub(crate) fn deeper<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Parsing<T> {
        if self.depth >= MAX_DEPTH {
            return Err(self.error_with(TOO_DEEP, self.offset(), false));
        }
        self.depth += 1;
        let result: Parsing<T> = stacker::maybe_grow(STACK_RED_ZONE, STACK_SEGMENT, || parse(self));
        self.depth -= 1;
        result
    }

    /// Counts one more level of an operator chain's tree, failing past the depth limit.
    pub(crate) fn lengthen(&mut self) -> Parsing<()> {
        if self.depth >= MAX_DEPTH {
            return Err(self.error_with(TOO_DEEP, self.offset(), false));
        }
        self.depth += 1;
        Ok(())
    }

    pub(crate) fn release(&mut self, levels: usize) {
        self.depth -= levels;
    }

    /// Skips a gap statement's tokens, stopping at text the tokenizer cannot read.
    pub(crate) fn skip_statement(&mut self, to_end: bool) -> Parsing<()> {
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Fault => return Err(self.error()),
                TokenKind::Op("SEMI") if !to_end => return Ok(()),
                _ => self.at += 1,
            }
        }
        Ok(())
    }

    /// One or more items separated by commas.
    pub(crate) fn comma_list<T>(
        &mut self,
        mut item: impl FnMut(&mut Self) -> Parsing<T>,
    ) -> Parsing<Vec<T>> {
        let mut items: Vec<T> = vec![item(self)?];
        while self.eat_op("COMMA") {
            items.push(item(self)?);
        }
        Ok(items)
    }

    /// Whether a query starts here: `SELECT`, `VALUES` or `WITH`.
    pub(crate) fn starts_select_at(&self, offset: usize) -> bool {
        self.kw_at(offset, "SELECT") || self.kw_at(offset, "VALUES") || self.kw_at(offset, "WITH")
    }
}
