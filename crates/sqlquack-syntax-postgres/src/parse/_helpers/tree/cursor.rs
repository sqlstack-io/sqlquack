//! The parser's state: the tokens, where it is, and how tokens read as names.

use std::collections::HashMap;

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::keywords::models::KeywordCategory;
use crate::parse::constants::{EXPRESSION_KEYWORDS, QUERY_STARTS};
use crate::parse::errors::ParseError;
use crate::parse::models::Gap;
use crate::scan::models::{Token, TokenKind};

pub(crate) type Parsing<T> = Result<T, ParseError>;

/// Recursion deeper than this fails as PostgreSQL's stack depth limit does (54001), not as syntax.
const MAX_DEPTH: usize = 300;

/// The longest flat chain of operators other than AND and OR, or of indirections.
const MAX_CHAIN: usize = 1000;

/// PostgreSQL's message for 54001, which recursion and chains past their limits both raise.
const STACK_DEPTH_EXCEEDED: &str = "stack depth limit exceeded";

/// A construct whose parse depends only on the token it starts at, so it is parsed once there.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Remembered {
    ParenthesizedQuery,
    JsonObjectArguments,
}

/// How a remembered construct's parse ended: its result, where it stopped, and the gaps it found.
struct Outcome {
    result: Parsing<Expression>,
    reached: usize,
    gaps: Vec<Gap>,
}

pub(crate) struct Parser<'sql> {
    pub(crate) sql: &'sql str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) at: usize,
    depth: usize,
    /// The depth limit's error once any path, even one later backtracked, has gone past it.
    too_deep: Option<ParseError>,
    /// Constructs already parsed from a token, replayed when another alternative reaches them.
    remembered: HashMap<(Remembered, usize), Outcome>,
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

/// `NonReservedWord`: an identifier or any keyword that is not reserved.
pub(crate) fn is_non_reserved_word(token: &Token) -> bool {
    plain(token)
        || matches!(
            category(token),
            Some(
                KeywordCategory::Unreserved
                    | KeywordCategory::ColName
                    | KeywordCategory::TypeFuncName
            )
        )
}

/// A reserved keyword, which `def_arg` reads as its own text.
pub(crate) fn is_reserved(token: &Token) -> bool {
    category(token) == Some(KeywordCategory::Reserved)
}

/// `BareColLabel`: an identifier or a keyword marked `BARE_LABEL`.
pub(crate) fn is_bare_label(token: &Token) -> bool {
    plain(token) || matches!(token.kind, TokenKind::Keyword(keyword) if keyword.bare_label)
}

/// A name token as an identifier; quoted names keep their case, others are already folded.
pub(crate) fn identifier(token: &Token) -> Identifier {
    match token.kind {
        TokenKind::QuotedIdent => Identifier::quoted(token.value.clone()),
        _ => Identifier::new(token.value.clone()),
    }
    .with_span(token.span)
}

impl<'sql> Parser<'sql> {
    pub(crate) fn new(sql: &'sql str, tokens: Vec<Token>) -> Self {
        Self {
            sql,
            tokens,
            at: 0,
            depth: 0,
            too_deep: None,
            remembered: HashMap::new(),
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
        self.peek_at(offset).is_some_and(
            |token| matches!(token.kind, TokenKind::Keyword(keyword) if keyword.name == name),
        )
    }

    pub(crate) fn kw(&self, name: &str) -> bool {
        self.kw_at(0, name)
    }

    /// The keyword `offset` tokens ahead, if that token is one.
    pub(crate) fn keyword_at(&self, offset: usize) -> Option<&'static str> {
        match self.peek_at(offset)?.kind {
            TokenKind::Keyword(keyword) => Some(keyword.name),
            _ => None,
        }
    }

    /// `opt_nulls_order`: `Some(true)` after `NULLS FIRST`, `Some(false)` after `NULLS LAST`.
    pub(crate) fn nulls_order(&mut self) -> Option<bool> {
        if !self.kw("nulls") {
            return None;
        }
        let first: bool = match self.keyword_at(1) {
            Some("first") => true,
            Some("last") => false,
            _ => return None,
        };
        self.at += 2;
        Some(first)
    }

    /// Whether the token `offset` ahead is a string constant (`Sconst`): plain, `E''` or dollar.
    pub(crate) fn sconst_at(&self, offset: usize) -> bool {
        self.peek_at(offset).is_some_and(|token| {
            matches!(
                token.kind,
                TokenKind::String | TokenKind::EscapeString | TokenKind::DollarString
            )
        })
    }

    /// Whether the token `offset` ahead can begin an `a_expr`.
    pub(crate) fn starts_expression_at(&self, offset: usize) -> bool {
        let Some(token) = self.peek_at(offset) else {
            return false;
        };
        match token.kind {
            TokenKind::Keyword(keyword) => {
                keyword.category != KeywordCategory::Reserved
                    || EXPRESSION_KEYWORDS.contains(&keyword.name)
            }
            TokenKind::Char(character) => "(+-".contains(character),
            TokenKind::Fault(_)
            | TokenKind::Typecast
            | TokenKind::DotDot
            | TokenKind::ColonEquals
            | TokenKind::EqualsGreater
            | TokenKind::LessEquals
            | TokenKind::GreaterEquals
            | TokenKind::NotEquals => false,
            _ => true,
        }
    }

    /// Whether `(`s from `offset` on lead to a word that starts a query.
    pub(crate) fn starts_query_at(&self, offset: usize) -> bool {
        let mut offset: usize = offset;
        while self.char_at(offset, '(') {
            offset += 1;
        }
        self.keyword_at(offset)
            .is_some_and(|name| QUERY_STARTS.contains(&name))
    }

    /// The index of the `)` closing the `(` at token index `open`, if it is closed.
    pub(crate) fn closing(&self, open: usize) -> Option<usize> {
        let mut depth: usize = 0;
        for (index, token) in self.tokens.iter().enumerate().skip(open) {
            match token.kind {
                TokenKind::Char('(') | TokenKind::Char('[') => depth += 1,
                TokenKind::Char(')') | TokenKind::Char(']') => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(index);
                    }
                }
                _ => {}
            }
        }
        None
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

    /// Eat a noise word such as `SAVEPOINT` only when more follows; last, it is the name (gram.y).
    pub(crate) fn eat_noise_kw(&mut self, name: &str) -> bool {
        let more: bool = self
            .peek_at(1)
            .is_some_and(|token| token.kind != TokenKind::Char(';'));
        more && self.eat_kw(name)
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

    /// Runs `parse` one level deeper; past the limit it and every later call fail, as 54001.
    pub(crate) fn deeper<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Parsing<T> {
        if let Some(error) = &self.too_deep {
            return Err(error.clone());
        }
        if self.depth >= MAX_DEPTH {
            return Err(self.refuse_depth());
        }
        self.depth += 1;
        let result: Parsing<T> = self.with_stack(parse);
        self.depth -= 1;
        result
    }

    /// Refuses the statement when `levels` more levels here, such as array bounds, pass the limit.
    pub(crate) fn within_depth_by(&mut self, levels: usize) -> Parsing<()> {
        match self.depth + levels > MAX_DEPTH {
            true => Err(self.refuse_depth()),
            false => Ok(()),
        }
    }

    /// A flat chain's link count after one more link, refusing the statement past `MAX_CHAIN`.
    pub(crate) fn link(&mut self, links: usize) -> Parsing<usize> {
        match links < MAX_CHAIN {
            true => Ok(links + 1),
            false => Err(self.refuse_depth()),
        }
    }

    /// The depth limit's error, recorded so every later level and the statement fail with it.
    fn refuse_depth(&mut self) -> ParseError {
        let error: ParseError = ParseError {
            message: STACK_DEPTH_EXCEEDED.to_owned(),
            position: self.offset(),
            syntax: false,
        };
        self.too_deep.get_or_insert(error).clone()
    }

    /// `parse` of `construct` from the cursor, or a replay of its earlier parse from there.
    pub(crate) fn remember(
        &mut self,
        construct: Remembered,
        parse: impl FnOnce(&mut Self) -> Parsing<Expression>,
    ) -> Parsing<Expression> {
        let key: (Remembered, usize) = (construct, self.at);
        if let Some(outcome) = self.remembered.get(&key) {
            self.at = outcome.reached;
            self.opaque |= !outcome.gaps.is_empty();
            self.gaps.extend(outcome.gaps.iter().cloned());
            return outcome.result.clone();
        }
        let gaps: usize = self.gaps.len();
        let result: Parsing<Expression> = parse(self);
        let outcome: Outcome = Outcome {
            result: result.clone(),
            reached: self.at,
            gaps: self.gaps[gaps.min(self.gaps.len())..].to_vec(),
        };
        self.remembered.insert(key, outcome);
        result
    }

    /// `result`, or the depth limit's error if any path went past it.
    pub(crate) fn unless_too_deep<T>(&self, result: Parsing<T>) -> Parsing<T> {
        match &self.too_deep {
            Some(error) => Err(error.clone()),
            None => result,
        }
    }

    /// Runs `parse` on a grown stack when this one is nearly used up, whatever the thread's size.
    #[inline(always)]
    fn with_stack<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Parsing<T> {
        #[cfg(feature = "stacker")]
        {
            let red_zone: usize = match cfg!(debug_assertions) {
                true => 4 * 1024 * 1024,
                false => 1024 * 1024,
            };
            stacker::maybe_grow(red_zone, 8 * 1024 * 1024, || parse(self))
        }

        #[cfg(not(feature = "stacker"))]
        {
            parse(self)
        }
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
        self.expect_token(test).map(|token| identifier(&token))
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
