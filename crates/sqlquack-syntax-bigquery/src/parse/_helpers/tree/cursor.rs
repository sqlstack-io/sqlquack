//! The parser's state: the tokens, where it is, and how tokens read as names.

use sqlquack_syntax::Gap;
use sqlquack_syntax::expressions::names::models::Identifier;

use crate::keywords::main::is_reserved::is_reserved;
use crate::parse::constants::{CLAUSE_WORDS, FOR_WORD, PAIRED_CLAUSE_WORDS, QUERY_STARTS};
use crate::parse::errors::ParseError;
use crate::scan::models::{NumberKind, Token, TokenKind};

pub(crate) type Parsing<T> = Result<T, ParseError>;

/// Nesting deeper than this fails as a parser check, not as a syntax error.
const MAX_DEPTH: usize = 200;

pub(crate) struct Parser<'sql> {
    pub(crate) sql: &'sql str,
    pub(crate) tokens: Vec<Token>,
    pub(crate) at: usize,
    depth: usize,
    pub(crate) gaps: Vec<Gap>,
    /// Whether the statement being parsed holds a gap, so it becomes a `Command`.
    pub(crate) opaque: bool,
}

/// `identifier`: a quoted name, or an unquoted word that is not a reserved keyword.
pub(crate) fn is_identifier(token: &Token) -> bool {
    match token.kind {
        TokenKind::Quoted => true,
        TokenKind::Word => !is_reserved(&token.value),
        _ => false,
    }
}

/// A part after a `.` in a path, where reserved words are names too.
pub(crate) fn is_path_word(token: &Token) -> bool {
    matches!(token.kind, TokenKind::Word | TokenKind::Quoted)
}

/// An alias position takes the same names as `identifier`; GoogleSQL has no stricter class.
pub(crate) fn is_strict_identifier(token: &Token) -> bool {
    is_identifier(token)
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

    /// Whether the tokens at `left` and `left + 1` touch, with nothing between them.
    pub(crate) fn adjacent(&self, left: usize) -> bool {
        match (self.peek_at(left), self.peek_at(left + 1)) {
            (Some(first), Some(second)) => first.end == second.start,
            _ => false,
        }
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
                TokenKind::Fault(syntax) => ParseError {
                    message: token.value.clone(),
                    position: token.start,
                    syntax,
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

    /// Runs `parse` one level deeper, failing past the depth limit.
    pub(crate) fn deeper<T>(&mut self, parse: impl FnOnce(&mut Self) -> Parsing<T>) -> Parsing<T> {
        if self.depth >= MAX_DEPTH {
            return Err(ParseError {
                message: "the statement nests too deeply".to_owned(),
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
        let (at, gaps, opaque, depth) = (self.at, self.gaps.len(), self.opaque, self.depth);
        parse(self).map_err(|error| {
            let reached: usize = self.at;
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
                TokenKind::Char(';') if depth == 0 => return Ok(()),
                TokenKind::Char('(') => depth += 1,
                TokenKind::Char(')') => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.at += 1;
        }
        Ok(())
    }

    /// Skips a script block through its `END [word]`, nested blocks and `CASE ... END` included.
    pub(crate) fn skip_block(&mut self) -> Parsing<()> {
        let mut blocks: usize = 0;
        let mut cases: usize = 0;
        let mut statement_start: bool = true;
        while let Some(token) = self.peek() {
            if let TokenKind::Fault(_) = token.kind {
                return Err(self.error());
            }
            let word: Option<String> = self.word_at(0);
            let opens: bool = statement_start
                && match word.as_deref() {
                    Some("BEGIN") => !(self.kw_at(1, "TRANSACTION") || self.ends_at(1)),
                    Some("IF" | "LOOP" | "WHILE" | "REPEAT" | "FOR" | "CASE") => true,
                    _ => false,
                };
            if opens {
                blocks += 1;
            } else if word.as_deref() == Some("CASE") {
                cases += 1;
            } else if word.as_deref() == Some("END") {
                if cases > 0 {
                    cases -= 1;
                } else {
                    blocks = blocks.saturating_sub(1);
                    if self.kw_in(1, &["IF", "LOOP", "WHILE", "REPEAT", "FOR", "CASE"]) {
                        self.at += 1;
                    }
                    if blocks == 0 {
                        self.at += 1;
                        while self.is(is_identifier) {
                            self.at += 1;
                        }
                        return Ok(());
                    }
                }
            }
            statement_start = self.is_char(';')
                || self.kw_in(0, &["THEN", "ELSE", "DO", "LOOP", "REPEAT"])
                || (opens && word.as_deref() == Some("BEGIN"))
                || self.is_char(':');
            self.at += 1;
        }
        Err(self.error())
    }

    /// Skips one balanced `( ... )` group.
    pub(crate) fn skip_parenthesized(&mut self) -> Parsing<()> {
        self.expect_char('(')?;
        let mut depth: usize = 1;
        while depth > 0 {
            match self.advance()?.kind {
                TokenKind::Char('(') => depth += 1,
                TokenKind::Char(')') => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    /// Whether a pipe set operation starts here: `[FULL|LEFT|INNER [OUTER]] UNION|EXCEPT|INTERSECT`.
    pub(crate) fn pipe_set_ahead(&self) -> bool {
        const SETS: [&str; 3] = ["UNION", "EXCEPT", "INTERSECT"];
        if self.kw_in(0, &SETS) {
            return true;
        }
        let width: usize = self.set_outer_mode_width();
        width > 0 && self.kw_in(width, &SETS)
    }

    /// The tokens of a set operation's outer mode at the cursor (`opt_corresponding_outer_mode`).
    pub(crate) fn set_outer_mode_width(&self) -> usize {
        match self.word_at(0).as_deref() {
            Some("FULL" | "LEFT") => 1 + usize::from(self.kw_at(1, "OUTER")),
            Some("INNER" | "OUTER") => 1,
            _ => 0,
        }
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

    /// `errorCapturingIdentifier`: an identifier; `a-b` is refused as an invalid identifier.
    pub(crate) fn ident(&mut self) -> Parsing<Identifier> {
        self.name(is_identifier)
    }

    /// `identifierList`: `( identifier, ... )`.
    pub(crate) fn identifier_list(&mut self) -> Parsing<Vec<Identifier>> {
        self.parenthesized(|parser| parser.ident())
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

    /// Whether the token `offset` ahead is a string literal.
    pub(crate) fn string_at(&self, offset: usize) -> bool {
        self.peek_at(offset)
            .is_some_and(|token| matches!(token.kind, TokenKind::String { .. }))
    }

    /// Whether the token `offset` ahead ends a list item: the end, `;`, `,`, `)` or `|>`.
    pub(crate) fn terminal_at(&self, offset: usize) -> bool {
        self.ends_at(offset)
            || self.char_at(offset, ',')
            || self.char_at(offset, ')')
            || self.op_at(offset, "|>")
    }

    /// Whether the word `offset` ahead begins a clause that can follow here, so it is no alias.
    pub(crate) fn starts_clause(&self, offset: usize) -> bool {
        let Some(word) = self.word_at(offset) else {
            return false;
        };
        let before_for: bool = word != FOR_WORD && self.kw_at(offset + 1, "FOR");
        if !CLAUSE_WORDS.contains(&word.as_str()) || self.terminal_at(offset + 1) || before_for {
            return false;
        }
        match PAIRED_CLAUSE_WORDS.iter().find(|(first, _)| *first == word) {
            Some((_, seconds)) => self.kw_in(offset + 1, seconds),
            None => true,
        }
    }

    /// Whether the token `offset` ahead is an alias written without `AS`: a name no clause takes.
    pub(crate) fn bare_alias_at(&self, offset: usize, strict: bool) -> bool {
        let test: fn(&Token) -> bool = match strict {
            true => is_strict_identifier,
            false => is_identifier,
        };
        self.is_at(offset, test) && !self.starts_clause(offset)
    }

    /// `path_expression`: `identifier ( . identifier )*`, a lone backquoted `a.b.c` being its parts.
    pub(crate) fn multipart_identifier(&mut self) -> Parsing<Vec<Identifier>> {
        if !self.is(is_identifier) {
            return Err(self.error());
        }
        let alone: bool = !(self.char_at(1, '.') && self.is_at(2, is_path_word));
        let (mut parts, mut dotted) = self.path_part(true, alone)?;
        while dotted || (self.is_char('.') && self.is_at(1, is_path_word)) {
            self.at += usize::from(!dotted);
            let (more, _) = self.path_part(false, false)?;
            parts.extend(more);
            dotted = false;
        }
        Ok(parts)
    }

    /// One part of a path (a name, a split backquoted path, or a dashed first part); true when dotted.
    fn path_part(&mut self, first: bool, split: bool) -> Parsing<(Vec<Identifier>, bool)> {
        let token: Token = self.expect_token(is_path_word)?;
        if split && token.kind == TokenKind::Quoted && token.value.contains('.') {
            return Ok((
                token
                    .value
                    .split('.')
                    .map(|part| Identifier::quoted(part.to_owned()).with_span(token.span))
                    .collect(),
                false,
            ));
        }
        let mut name: String = token.value.clone();
        let mut dotted: bool = false;
        while first
            && token.kind == TokenKind::Word
            && self.is_char('-')
            && self.adjacent(0)
            && self.adjacent_back()
        {
            let Some(next) = self.peek_at(1) else {
                break;
            };
            let digits: &str = next.value.strip_suffix('.').unwrap_or(&next.value);
            let integer: bool =
                !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());
            let split: bool = integer
                && digits.len() < next.value.len()
                && self
                    .peek_at(2)
                    .is_some_and(|after| after.start == next.end && is_path_word(after));
            let part: &str = match next.kind {
                TokenKind::Word => &next.value,
                TokenKind::Number(NumberKind::Integer) => digits,
                TokenKind::Number(NumberKind::Float) if split => digits,
                _ => break,
            };
            name.push('-');
            name.push_str(part);
            self.at += 2;
            if split {
                dotted = true;
                break;
            }
        }
        Ok((
            vec![match token.kind {
                TokenKind::Quoted => identifier(&token),
                _ => Identifier::new(name).with_span(token.span),
            }],
            dotted,
        ))
    }

    /// Whether the token before the cursor touches it.
    fn adjacent_back(&self) -> bool {
        self.at
            .checked_sub(1)
            .and_then(|before| self.tokens.get(before))
            .zip(self.peek())
            .is_some_and(|(before, here)| before.end == here.start)
    }
}
