//! The parser's state: the tokens, where it is, and how tokens read as names.

use sqlquack_syntax::expressions::names::models::Identifier;

use crate::keywords::main::classify::classify;
use crate::keywords::models::KeywordClasses;
use crate::parse::constants::QUERY_STARTS;
use crate::parse::errors::ParseError;
use crate::scan::models::{Token, TokenKind};
use sqlquack_syntax::Gap;

pub(crate) type Parsing<T> = Result<T, ParseError>;

/// Nesting deeper than this fails as a parser check, not as a syntax error.
const MAX_DEPTH: usize = 200;

/// The longest flat chain of operators or indirections DuckDB's transformer takes, its expression depth.
const MAX_CHAIN: usize = 1000;

/// DuckDB's refusal of a chain past `MAX_CHAIN`.
const CHAIN_TOO_LONG: &str = "Max expression depth limit of 1000 exceeded. Use \"SET max_expression_depth TO x\" to increase the maximum expression depth.";

/// Stack left below which a deeper level moves to a new segment, so the caller's stack size never decides.
const RED_ZONE: usize = if cfg!(debug_assertions) {
    4 << 20
} else {
    1 << 20
};

/// The size of each stack segment a deeper level allocates.
const STACK_SEGMENT: usize = 8 << 20;

pub(crate) struct Parser<'sql> {
    pub(crate) sql: &'sql str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) at: usize,
    depth: usize,
    pub(crate) gaps: Vec<Gap>,
    /// Whether the statement being parsed holds a gap, so it becomes a `Command`.
    pub(crate) opaque: bool,
    /// Where a type modifier that is no constant was read, which the transformer refuses.
    pub(crate) nonconstant_modifier: Option<usize>,
    /// How many parenthesised subqueries have been read, for the checks that refuse one.
    pub(crate) subqueries: usize,
    /// A transformer refusal that holds only once the statement parses, as DuckDB checks it after.
    pub(crate) refusal: Option<ParseError>,
    /// The depth limit's error once any reading passed it, which refuses the statement whatever reading wins.
    pub(crate) too_deep: Option<ParseError>,
    /// The tallest type read so far among the siblings being read, constructors and array bounds counted together.
    pub(crate) type_height: usize,
    /// Where the last describe atom began, by token, and the statement name its refusals use.
    pub(crate) describe: Option<(usize, &'static str)>,
    /// How many PIVOT ON entries read so far take their values from the data.
    pub(crate) pivot_entries: usize,
    /// How many window definitions enclose the cursor.
    pub(crate) window_definitions: usize,
}

/// Whether `token` is a quoted name, or a word whose keyword classes include one of `allowed`.
fn is_name(token: &Token, allowed: fn(&KeywordClasses) -> bool) -> bool {
    match token.kind {
        TokenKind::Quoted => true,
        TokenKind::Word => {
            let classes: KeywordClasses = classify(&token.value);
            classes == KeywordClasses::default() || allowed(&classes)
        }
        _ => false,
    }
}

/// `Identifier` and `ColId`: a word that is no keyword, or an unreserved or column-name keyword.
pub(crate) fn is_identifier(token: &Token) -> bool {
    is_name(token, |classes| classes.unreserved || classes.column_name)
}

/// `FunctionName`: no keyword, or an unreserved or function-name keyword.
pub(crate) fn is_function_name(token: &Token) -> bool {
    is_name(token, |classes| classes.unreserved || classes.func_name)
}

/// `TypeFuncName` as DuckDB 2.0.0 reads a parameter's name: any name but a reserved keyword.
pub(crate) fn is_type_func_name(token: &Token) -> bool {
    is_name(token, |classes| !classes.reserved)
}

/// `TypeName`: no keyword, or an unreserved or type-name keyword.
pub(crate) fn is_type_name(token: &Token) -> bool {
    is_name(token, |classes| classes.unreserved || classes.type_name)
}

/// `BareColLabel`: no keyword, or a bare-label keyword.
pub(crate) fn is_bare_label(token: &Token) -> bool {
    is_name(token, |classes| classes.bare_label)
}

/// `ReservedKeyword`: an unquoted reserved word.
pub(crate) fn is_reserved_word(token: &Token) -> bool {
    token.kind == TokenKind::Word && classify(&token.value).reserved
}

/// `ColLabel` and the reserved name rules after a qualification: any word or quoted name.
pub(crate) fn is_col_label(token: &Token) -> bool {
    is_name(token, |_| true)
}

/// A name token as an identifier; a quoted name is marked so.
pub(crate) fn identifier(token: &Token) -> Identifier {
    match token.kind {
        TokenKind::Quoted => Identifier::quoted(token.value.clone()),
        _ => Identifier::new(token.value.clone()),
    }
    .with_span(token.span)
}

/// Whether `token` is the unquoted word `name`, compared without case.
pub(crate) fn is_word(token: &Token, name: &str) -> bool {
    token.kind == TokenKind::Word && token.value.eq_ignore_ascii_case(name)
}

impl<'sql> Parser<'sql> {
    /// A parser over `tokens`, with each one-character operator read as that character.
    pub(crate) fn new(sql: &'sql str, tokens: Vec<Token>) -> Self {
        let tokens: Vec<Token> = tokens
            .into_iter()
            .map(|token| match (token.kind, token.value.as_bytes()) {
                (TokenKind::Op, [byte]) => Token {
                    kind: TokenKind::Char(char::from(*byte)),
                    ..token
                },
                _ => token,
            })
            .collect();
        Self {
            sql,
            tokens,
            at: 0,
            depth: 0,
            gaps: Vec::new(),
            opaque: false,
            nonconstant_modifier: None,
            subqueries: 0,
            refusal: None,
            too_deep: None,
            type_height: 0,
            describe: None,
            pivot_entries: 0,
            window_definitions: 0,
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

    /// Whether the token `offset` ahead is the multi-character operator `text`.
    pub(crate) fn op_at(&self, offset: usize, text: &str) -> bool {
        self.peek_at(offset)
            .is_some_and(|token| token.kind == TokenKind::Op && token.value == text)
    }

    pub(crate) fn is_op(&self, text: &str) -> bool {
        self.op_at(0, text)
    }

    /// Whether the current token satisfies `test`.
    pub(crate) fn is(&self, test: fn(&Token) -> bool) -> bool {
        self.peek().is_some_and(test)
    }

    /// Whether the token `offset` ahead satisfies `test`.
    pub(crate) fn is_at(&self, offset: usize, test: fn(&Token) -> bool) -> bool {
        self.peek_at(offset).is_some_and(test)
    }

    /// Whether the input or the statement ends at the token `offset` ahead.
    pub(crate) fn ends_at(&self, offset: usize) -> bool {
        self.peek_at(offset)
            .is_none_or(|token| token.kind == TokenKind::Char(';'))
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

    pub(crate) fn eat_op(&mut self, text: &str) -> bool {
        let found: bool = self.is_op(text);
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
                message: "Syntax error at or near end of input".to_owned(),
                position: self.sql.len(),
                syntax: true,
            },
            Some(token) => match token.kind {
                TokenKind::Fault(_) => ParseError {
                    message: token.value.clone(),
                    position: token.start,
                    syntax: true,
                },
                _ => ParseError {
                    message: format!(
                        "Syntax error at or near '{}'",
                        &self.sql[token.start..token.end]
                    ),
                    position: token.start,
                    syntax: true,
                },
            },
        }
    }

    /// An error the parser's own checks raise, with their own message.
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

    /// Keeps the first refusal of the statement, for when it parses.
    pub(crate) fn refuse_later(&mut self, message: &str, position: usize) {
        if self.refusal.is_none() {
            self.refusal = Some(self.error_with(message, position, false));
        }
    }

    /// The depth limit's error, kept so that the statement is refused with it whatever reading wins.
    pub(crate) fn nests_too_deeply(&mut self) -> ParseError {
        let error: ParseError = ParseError {
            message: "the statement nests too deeply".to_owned(),
            position: self.offset(),
            syntax: false,
        };
        self.too_deep.get_or_insert(error).clone()
    }

    /// Whether a type `height` levels tall, constructors and bounds together, passes the depth limit.
    pub(crate) fn type_too_tall(height: usize) -> bool {
        height >= MAX_DEPTH
    }

    /// Runs `parse` one level deeper, failing past the depth limit.
    pub(crate) fn deeper<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Parsing<T> {
        if let Some(too_deep) = &self.too_deep {
            return Err(too_deep.clone());
        }
        if self.depth >= MAX_DEPTH {
            return Err(self.nests_too_deeply());
        }
        self.depth += 1;
        let result: Parsing<T> = stacker::maybe_grow(RED_ZONE, STACK_SEGMENT, || parse(self));
        self.depth -= 1;
        result
    }

    /// A flat chain's link count after one more link, refusing the statement once it reaches `MAX_CHAIN`.
    pub(crate) fn link(&mut self, links: usize) -> Parsing<usize> {
        if links + 1 < MAX_CHAIN {
            return Ok(links + 1);
        }
        let error: ParseError = ParseError {
            message: CHAIN_TOO_LONG.to_owned(),
            position: self.offset(),
            syntax: false,
        };
        Err(self.too_deep.get_or_insert(error).clone())
    }

    /// An AND, OR or set-operation chain's link count after one more, refused past `MAX_CHAIN`.
    pub(crate) fn flat_link(&mut self, links: usize) -> Parsing<usize> {
        match links + 1 < MAX_CHAIN {
            true => Ok(links + 1),
            false => Err(self.nests_too_deeply()),
        }
    }

    /// Tries `parse`; on failure rewinds and returns the error and how far it got.
    pub(crate) fn attempt<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Parsing<T>,
    ) -> Result<T, (ParseError, usize)> {
        let (at, gaps, opaque, depth) = (self.at, self.gaps.len(), self.opaque, self.depth);
        let (modifier, subqueries) = (self.nonconstant_modifier, self.subqueries);
        let (refusal, describe) = (self.refusal.clone(), self.describe);
        let pivot_entries: usize = self.pivot_entries;
        parse(self).map_err(|error| {
            self.pivot_entries = pivot_entries;
            self.subqueries = subqueries;
            self.refusal = refusal;
            self.describe = describe;
            let reached: usize = self.at;
            self.nonconstant_modifier = modifier;
            self.at = at;
            self.gaps.truncate(gaps);
            self.opaque = opaque;
            self.depth = depth;
            (error, reached)
        })
    }

    /// Skips to the `;` ending the statement at this depth; a scanner fault stops it.
    pub(crate) fn skip_statement_rest(&mut self) -> Parsing<()> {
        let mut depth: usize = 0;
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Fault(_) => return Err(self.error()),
                TokenKind::Char(';' | ')') if depth == 0 => return Ok(()),
                TokenKind::Char('(') => depth += 1,
                TokenKind::Char(')') => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.at += 1;
        }
        Ok(())
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

    /// `List(item)`: one or more items; a comma before a non-item is trailing.
    pub(crate) fn comma_list<T>(
        &mut self,
        mut item: impl FnMut(&mut Self) -> Parsing<T>,
    ) -> Parsing<Vec<T>> {
        let mut items: Vec<T> = vec![item(self)?];
        while self.eat_char(',') {
            let start: usize = self.at;
            match self.attempt(&mut item) {
                Ok(next) => items.push(next),
                Err((_, reached)) if reached == start => break,
                Err((error, _)) => return Err(error),
            }
        }
        Ok(items)
    }

    /// A name `test` accepts, as an identifier.
    pub(crate) fn name(&mut self, test: fn(&Token) -> bool) -> Parsing<Identifier> {
        self.expect_token(test).map(|token| identifier(&token))
    }

    /// Whether `(`s from `offset` on lead to a word that starts a query.
    pub(crate) fn starts_query_at(&self, offset: usize) -> bool {
        let mut offset: usize = offset;
        while self.char_at(offset, '(') {
            offset += 1;
        }
        self.word_at(offset)
            .is_some_and(|word| QUERY_STARTS.contains(&word.as_str()))
    }

    /// Whether the token `offset` ahead is a string written in single quotes, a table name's last part.
    pub(crate) fn single_quoted_at(&self, offset: usize) -> bool {
        self.peek_at(offset).is_some_and(|token| {
            matches!(token.kind, TokenKind::String(_))
                && self.sql[token.start..token.end].starts_with('\'')
        })
    }

    /// Whether the token `offset` ahead is a string literal.
    pub(crate) fn string_at(&self, offset: usize) -> bool {
        self.peek_at(offset).is_some_and(|token| {
            matches!(token.kind, TokenKind::String(_) | TokenKind::DollarString)
        })
    }

    /// Whether the current token is a plain string continuing the one before it, on a later line.
    pub(crate) fn continues_string(&self) -> bool {
        let (Some(before), Some(token)) = (
            self.at
                .checked_sub(1)
                .and_then(|last| self.tokens.get(last)),
            self.peek(),
        ) else {
            return false;
        };
        let gap: &str = &self.sql[before.end..token.start];
        token.kind == TokenKind::String(crate::scan::models::StringPrefix::Plain)
            && gap.contains(['\n', '\r'])
            && !gap.contains("/*")
    }

    /// Whether the token `offset` ahead ends a list item: the end, `;`, `,` or `)`.
    pub(crate) fn terminal_at(&self, offset: usize) -> bool {
        self.ends_at(offset) || self.char_at(offset, ',') || self.char_at(offset, ')')
    }
}
