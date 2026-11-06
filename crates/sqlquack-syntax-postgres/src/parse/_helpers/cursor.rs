//! The parser's state: the tokens, where it is, and how tokens read as names.

use sqlquack_syntax::expressions::names::models::Identifier;

use crate::keywords::models::KeywordCategory;
use crate::parse::models::{Gap, ParseError};
use crate::scan::models::{Token, TokenKind};

pub(crate) type Parsing<T> = Result<T, ParseError>;

/// Recursion deeper than this fails as PostgreSQL's stack depth limit does (54001), not as syntax.
const MAX_DEPTH: usize = 300;

pub(crate) struct Parser<'sql> {
    pub(crate) sql: &'sql str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) at: usize,
    depth: usize,
    pub(crate) gaps: Vec<Gap>,
    /// Whether the statement being parsed holds a gap, so it becomes a `Command`.
    pub(crate) opaque: bool,
}

/// The keyword category of a token, if it is a keyword.
fn category(token: &Token) -> Option<KeywordCategory> {
    match token.kind {
        TokenKind::Keyword(keyword) => Some(keyword.category),
        _ => None,
    }
}

fn plain(token: &Token) -> bool {
    matches!(token.kind, TokenKind::Ident | TokenKind::QuotedIdent)
}

/// `ColId`: an identifier, an unreserved keyword or a column-name keyword.
pub(crate) fn is_col_id(token: &Token) -> bool {
    plain(token)
        || matches!(
            category(token),
            Some(KeywordCategory::Unreserved | KeywordCategory::ColName)
        )
}

/// `type_function_name`: an identifier, an unreserved keyword or a type-or-function keyword.
pub(crate) fn is_type_function_name(token: &Token) -> bool {
    plain(token)
        || matches!(
            category(token),
            Some(KeywordCategory::Unreserved | KeywordCategory::TypeFuncName)
        )
}

/// `ColLabel`: an identifier or any keyword.
pub(crate) fn is_col_label(token: &Token) -> bool {
    plain(token) || category(token).is_some()
}

/// `BareColLabel`: an identifier or a keyword marked `BARE_LABEL`.
pub(crate) fn is_bare_label(token: &Token) -> bool {
    plain(token) || matches!(token.kind, TokenKind::Keyword(keyword) if keyword.bare_label)
}

/// `NonReservedWord`: an identifier or any keyword that is not reserved.
pub(crate) fn is_non_reserved(token: &Token) -> bool {
    plain(token) || matches!(category(token), Some(category) if category != KeywordCategory::Reserved)
}

/// A name token as an identifier; quoted names keep their case, others are already folded.
pub(crate) fn identifier(token: &Token) -> Identifier {
    match token.kind {
        TokenKind::QuotedIdent => Identifier::quoted(token.value.clone()),
        _ => Identifier::new(token.value.clone()),
    }
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
        }
    }

    pub(crate) fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.at + offset)
    }

    pub(crate) fn peek(&self) -> Option<&Token> {
        self.peek_at(0)
    }

    /// Whether the token `offset` ahead is the keyword `name`.
    pub(crate) fn kw_at(&self, offset: usize, name: &str) -> bool {
        self.peek_at(offset).is_some_and(|token| {
            matches!(token.kind, TokenKind::Keyword(keyword) if keyword.name == name)
        })
    }

    pub(crate) fn kw(&self, name: &str) -> bool {
        self.kw_at(0, name)
    }

    pub(crate) fn char_at(&self, offset: usize, character: char) -> bool {
        self.peek_at(offset)
            .is_some_and(|token| token.kind == TokenKind::Char(character))
    }

    pub(crate) fn is_char(&self, character: char) -> bool {
        self.char_at(0, character)
    }

    pub(crate) fn is_kind(&self, kind: TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == kind)
    }

    /// Whether the current token satisfies `test`.
    pub(crate) fn is(&self, test: fn(&Token) -> bool) -> bool {
        self.peek().is_some_and(test)
    }

    pub(crate) fn advance(&mut self) -> Parsing<Token> {
        match self.peek() {
            Some(token) if matches!(token.kind, TokenKind::Fault(_)) => Err(self.error()),
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

    pub(crate) fn eat_char(&mut self, character: char) -> bool {
        let found: bool = self.is_char(character);
        self.at += usize::from(found);
        found
    }

    pub(crate) fn eat_kind(&mut self, kind: TokenKind) -> bool {
        let found: bool = self.is_kind(kind);
        self.at += usize::from(found);
        found
    }

    pub(crate) fn expect_kw(&mut self, name: &str) -> Parsing<()> {
        match self.eat_kw(name) {
            true => Ok(()),
            false => Err(self.error()),
        }
    }

    pub(crate) fn expect_char(&mut self, character: char) -> Parsing<()> {
        match self.eat_char(character) {
            true => Ok(()),
            false => Err(self.error()),
        }
    }

    /// The current token if it satisfies `test`, consumed; a syntax error otherwise.
    pub(crate) fn expect(&mut self, test: fn(&Token) -> bool) -> Parsing<Token> {
        match self.is(test) {
            true => self.advance(),
            false => Err(self.error()),
        }
    }

    /// The syntax error at the current token, or the scanner fault it stands for.
    pub(crate) fn error(&self) -> ParseError {
        match self.peek() {
            None => ParseError {
                message: "syntax error at end of input".to_owned(),
                position: self.sql.len(),
                syntax: true,
            },
            Some(token) => match token.kind {
                TokenKind::Fault(syntax) => ParseError {
                    message: token.value.clone(),
                    position: token.start,
                    syntax,
                },
                _ => ParseError {
                    message: format!(
                        "syntax error at or near \"{}\"",
                        &self.sql[token.start..token.end]
                    ),
                    position: token.start,
                    syntax: true,
                },
            },
        }
    }

    /// A syntax error PostgreSQL's grammar actions raise, with their own message.
    pub(crate) fn error_with(&self, message: &str, position: usize) -> ParseError {
        ParseError {
            message: message.to_owned(),
            position,
            syntax: true,
        }
    }

    /// Byte offset of the current token, or the input's end.
    pub(crate) fn offset(&self) -> usize {
        self.peek().map_or(self.sql.len(), |token| token.start)
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

    /// Runs `parse` one level deeper, failing past the depth limit as PostgreSQL does.
    pub(crate) fn deeper<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Parsing<T> {
        if self.depth >= MAX_DEPTH {
            return Err(ParseError {
                message: "stack depth limit exceeded".to_owned(),
                position: self.offset(),
                syntax: false,
            });
        }
        self.depth += 1;
        let result: Parsing<T> = parse(self);
        self.depth -= 1;
        result
    }

    /// Tries `parse`; on failure rewinds and returns the error and how far it got.
    pub(crate) fn attempt<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Parsing<T>,
    ) -> Result<T, (ParseError, usize)> {
        let (at, gaps, opaque) = (self.at, self.gaps.len(), self.opaque);
        parse(self).map_err(|error| {
            let reached: usize = self.at;
            self.at = at;
            self.gaps.truncate(gaps);
            self.opaque = opaque;
            (error, reached)
        })
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

    /// `item, ...` with at least one item.
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

    /// A name `test` accepts, as an identifier.
    pub(crate) fn name(&mut self, test: fn(&Token) -> bool) -> Parsing<Identifier> {
        self.expect(test).map(|token| identifier(&token))
    }

    /// `name_list`: `ColId, ...` in parentheses.
    pub(crate) fn name_list(&mut self) -> Parsing<Vec<Identifier>> {
        self.parenthesized(|parser| parser.name(is_col_id))
    }

    /// `any_name` or `qualified_name`: `ColId [ . ColLabel ... ]`.
    pub(crate) fn qualified_name(&mut self) -> Parsing<Vec<Identifier>> {
        let mut parts: Vec<Identifier> = vec![self.name(is_col_id)?];
        while self.is_char('.') && self.peek_at(1).is_some_and(is_col_label) {
            self.at += 1;
            parts.push(self.name(is_col_label)?);
        }
        Ok(parts)
    }
}
