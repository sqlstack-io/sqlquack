//! Snowflake's COPY INTO and COPY FILES grammar, checked on the statement's tokens before it is read.

use crate::expressions::statements::models::Command;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Reads a Snowflake COPY that passes the grammar check: the typed tree when it covers the statement, else a command.
    pub(crate) fn parse_snowflake_copy(&mut self) -> Result<Expression> {
        self.check_snowflake_copy()?;
        let start = self.current;
        let end = self.copy_statement_end();
        if let Ok(copy) = self.parse_copy_typed() {
            if self.current == end {
                return Ok(copy);
            }
        }
        self.current = start;
        let mut tokens = vec![("COPY".to_owned(), TokenType::Copy)];
        while self.current < end {
            let token = self.advance()?;
            tokens.push((self.raw_sql_token_text(&token), token.token_type));
        }
        Ok(Expression::Command(Box::new(Command {
            this: self.join_command_tokens(tokens),
        })))
    }

    /// Refuses a Snowflake COPY that breaks `COPY [FILES] INTO <target> [FROM <source>] [<option> ...]`, FROM optional only in a table load; the position is after COPY.
    pub(crate) fn check_snowflake_copy(&self) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let end = self.copy_statement_end();
        if self.copy_path_comment(end) {
            return Ok(());
        }
        let files = self.copy_word(self.current, "FILES");
        let mut at = self.current + usize::from(files);
        if self.copy_type(at) != Some(TokenType::Into) {
            return Err(self.parse_error("Expected INTO after COPY"));
        }
        at += 1;
        let unload = self.copy_is_stage(at) || self.copy_type(at) == Some(TokenType::String);
        at = if self.copy_is_stage(at) {
            self.copy_skip_stage(at, end)?
        } else if files {
            return Err(self.parse_error("Expected a stage after COPY FILES INTO"));
        } else if unload {
            at + 1
        } else {
            self.copy_skip_table(at, end)?
        };
        let from_table_stage =
            !unload && !files && (at >= end || self.copy_type(at + 1) == Some(TokenType::Eq));
        if from_table_stage {
            return self.copy_check_options(at, end);
        }
        if self.copy_type(at) != Some(TokenType::From) {
            return Err(self.parse_error("Expected FROM and the COPY source"));
        }
        at += 1;
        at = if self.copy_type(at) == Some(TokenType::LParen) {
            self.copy_skip_query(at, end, !unload && !files)?
        } else if unload && !files {
            self.copy_skip_name(at)?
        } else if self.copy_is_stage(at) {
            self.copy_skip_stage(at, end)?
        } else if !files
            && matches!(
                self.copy_type(at),
                Some(TokenType::String | TokenType::Parameter | TokenType::Placeholder)
            )
        {
            at + 1
        } else {
            return Err(self.parse_error("Expected a stage or location after FROM"));
        };
        self.copy_check_options(at, end)
    }

    /// The index of the statement's closing semicolon, or of the end of the tokens.
    fn copy_statement_end(&self) -> usize {
        let mut depth = 0usize;
        let mut at = self.current;
        while let Some(token) = self.tokens.get(at) {
            match token.token_type {
                TokenType::LParen => depth += 1,
                TokenType::RParen => depth = depth.saturating_sub(1),
                TokenType::Semicolon if depth == 0 => return at,
                _ => {}
            }
            at += 1;
        }
        at
    }

    /// Whether the tokens end in a stage followed directly by `//`, which Snowflake reads as a path and the tokenizer as a comment.
    fn copy_path_comment(&self, end: usize) -> bool {
        let (Some(source), Some(last)) = (
            self.source.as_deref(),
            end.checked_sub(1).and_then(|at| self.tokens.get(at)),
        ) else {
            return false;
        };
        end == self.tokens.len()
            && source.chars().skip(last.span.end).take(2).eq("//".chars())
            && (self.current..end).any(|at| self.copy_is_stage(at))
    }

    fn copy_type(&self, at: usize) -> Option<TokenType> {
        self.tokens.get(at).map(|token| token.token_type)
    }

    fn copy_word(&self, at: usize, word: &str) -> bool {
        self.tokens.get(at).is_some_and(|token| {
            token.token_type != TokenType::QuotedIdentifier && token.text.eq_ignore_ascii_case(word)
        })
    }

    /// Whether the token at `at` is an unquoted Snowflake reserved keyword.
    fn copy_reserved(&self, at: usize) -> bool {
        self.tokens.get(at).is_some_and(|token| {
            token.token_type != TokenType::QuotedIdentifier
                && SNOWFLAKE_RESERVED_KEYWORDS
                    .iter()
                    .any(|keyword| token.text.eq_ignore_ascii_case(keyword))
        })
    }

    /// Whether the token at `at` can be one part of an object name.
    fn copy_name_part(&self, at: usize) -> bool {
        self.copy_word_like(at) && !self.copy_reserved(at)
    }

    /// Whether the token at `at` is a quoted identifier or a word, reserved or not.
    fn copy_word_like(&self, at: usize) -> bool {
        self.tokens.get(at).is_some_and(|token| {
            token.token_type == TokenType::QuotedIdentifier
                || token.token_type != TokenType::String
                    && !token.text.starts_with('@')
                    && token
                        .text
                        .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        })
    }

    fn copy_is_stage(&self, at: usize) -> bool {
        self.tokens.get(at).is_some_and(|token| {
            token.token_type == TokenType::DAt
                || token.token_type == TokenType::Var && token.text.starts_with('@')
        })
    }

    /// Skips a stage: `@` or `@<name>`, dotted parts that may be empty, `%` or `~`, then a path written without spaces.
    fn copy_skip_stage(&self, at: usize, end: usize) -> Result<usize> {
        let mut at = at;
        let mut part = self.copy_type(at) == Some(TokenType::DAt);
        let mut table = false;
        at += 1;
        loop {
            match self.copy_type(at) {
                Some(TokenType::String) if part => {
                    return Err(self.parse_error("A stage name part cannot be a string literal"));
                }
                Some(TokenType::Tilde) if part => at += 1,
                Some(TokenType::Percent | TokenType::Mod) if part => {
                    at += 1;
                    table = true;
                }
                Some(TokenType::QuotedIdentifier) if part => {
                    if self.tokens[at].text.contains(char::is_whitespace) {
                        return Err(
                            self.parse_error("A quoted stage name part cannot hold a space")
                        );
                    }
                    at += 1;
                    part = false;
                }
                Some(TokenType::Dot) => {
                    at += 1;
                    part = true;
                }
                _ if part
                    && (self.copy_name_part(at) || table && self.copy_word_like(at))
                    && self.copy_adjacent(at) =>
                {
                    at += 1;
                    part = false;
                }
                _ => break,
            }
        }
        while at < end && self.copy_type(at) == Some(TokenType::Slash) && self.copy_adjacent(at) {
            at += 1;
            while at < end
                && self.copy_adjacent(at)
                && !matches!(
                    self.copy_type(at),
                    Some(TokenType::LParen | TokenType::RParen | TokenType::Comma)
                )
            {
                at += 1;
            }
        }
        Ok(at)
    }

    /// Whether the token at `at` follows the previous token with no space between them.
    fn copy_adjacent(&self, at: usize) -> bool {
        at > 0
            && self
                .tokens
                .get(at)
                .is_some_and(|token| token.span.start == self.tokens[at - 1].span.end)
    }

    /// Skips a dotted object name or `IDENTIFIER( ... )`, refusing a stage, string or reserved word in its place.
    fn copy_skip_name(&self, at: usize) -> Result<usize> {
        if self.copy_word(at, "IDENTIFIER") && self.copy_type(at + 1) == Some(TokenType::LParen) {
            return Ok(self.copy_close(at + 1, self.copy_statement_end())? + 1);
        }
        let mut at = at;
        loop {
            if !self.copy_name_part(at) {
                return Err(self.parse_error("Expected a table name"));
            }
            at += 1;
            if self.copy_type(at) != Some(TokenType::Dot) {
                return Ok(at);
            }
            at += 1;
        }
    }

    /// Skips a target table name and its optional `( <column> , ... )` list.
    fn copy_skip_table(&self, at: usize, end: usize) -> Result<usize> {
        let mut at = self.copy_skip_name(at)?;
        if self.copy_type(at) != Some(TokenType::LParen) {
            return Ok(at);
        }
        loop {
            at += 1;
            if at >= end || !self.copy_name_part(at) {
                return Err(self.parse_error("Expected a column name"));
            }
            at += 1;
            match self.copy_type(at) {
                Some(TokenType::Comma) => {}
                Some(TokenType::RParen) => return Ok(at + 1),
                _ => return Err(self.parse_error("Expected , or ) in the column list")),
            }
        }
    }

    /// The index of the parenthesis closing the one at `at`.
    fn copy_close(&self, at: usize, end: usize) -> Result<usize> {
        let mut depth = 0usize;
        for index in at..end {
            match self.copy_type(index) {
                Some(TokenType::LParen) => depth += 1,
                Some(TokenType::RParen) => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Ok(index);
                    }
                }
                _ => {}
            }
        }
        Err(self.parse_error("Expected )"))
    }

    /// Skips `( <query> )`; a load's query also needs a FROM naming a stage, and no `$` before a quoted name.
    fn copy_skip_query(&self, at: usize, end: usize, load: bool) -> Result<usize> {
        let close = self.copy_close(at, end)?;
        if !matches!(
            self.copy_type(at + 1),
            Some(TokenType::Select | TokenType::With)
        ) {
            return Err(self.parse_error("Expected a query in parentheses"));
        }
        let mut depth = 0usize;
        let mut from = None;
        for index in at + 1..close {
            let next = self.copy_type(index + 1);
            let column = self.copy_type(index - 1) == Some(TokenType::Dollar);
            match self.copy_type(index) {
                Some(TokenType::LParen) => depth += 1,
                Some(TokenType::RParen) => depth = depth.saturating_sub(1),
                Some(TokenType::Select) if !column && next == Some(TokenType::From) => {
                    return Err(self.parse_error("Expected a select list"));
                }
                Some(TokenType::Select)
                    if index > at + 1 && self.copy_type(index - 1) == Some(TokenType::Comma) =>
                {
                    return Err(self.parse_error("Expected an expression after ,"));
                }
                Some(TokenType::From) if depth == 0 && from.is_none() => {
                    if index + 1 == close
                        || !self.copy_is_stage(index + 1)
                            && next != Some(TokenType::LParen)
                            && self.copy_reserved(index + 1)
                    {
                        return Err(self.parse_error("Expected a source after FROM"));
                    }
                    from = Some(index);
                }
                Some(TokenType::Dollar)
                    if load
                        && (matches!(
                            next,
                            Some(TokenType::QuotedIdentifier | TokenType::String)
                        ) || next == Some(TokenType::From)) =>
                {
                    return Err(self.parse_error("Expected a column number after $"));
                }
                Some(TokenType::Dot)
                    if load
                        && (next == Some(TokenType::From)
                            || self.copy_type(index - 1) == Some(TokenType::String)
                            || self.copy_reserved(index - 1)) =>
                {
                    return Err(self.parse_error("Expected a name around ."));
                }
                _ => {}
            }
        }
        if !load {
            return Ok(close + 1);
        }
        let Some(from) = from else {
            return Err(self.parse_error("Expected FROM and a stage in the COPY query"));
        };
        if self.copy_is_stage(from + 1) {
            self.copy_skip_stage(from + 1, close)?;
        }
        Ok(close + 1)
    }

    /// Checks the options after the source: `<name> = <value>` or a parenthesized list, and `PARTITION BY <expr>`.
    fn copy_check_options(&self, at: usize, end: usize) -> Result<()> {
        let mut at = at;
        while at < end {
            if self.copy_word(at, "PARTITION") {
                if self.copy_type(at + 1) != Some(TokenType::By) || at + 2 >= end {
                    return Err(self.parse_error("Expected BY and an expression after PARTITION"));
                }
                at += 2;
                while at < end
                    && !(self.copy_name_part(at) && self.copy_type(at + 1) == Some(TokenType::Eq))
                {
                    at += 1;
                }
                continue;
            }
            if !self.copy_name_part(at) || self.copy_type(at + 1) != Some(TokenType::Eq) {
                return Err(self.parse_error("Expected a COPY option"));
            }
            let value = at + 2;
            if value >= end {
                return Err(self.parse_error("Expected a value after ="));
            }
            at = if self.copy_type(value) == Some(TokenType::LParen) {
                let close = self.copy_close(value, end)?;
                let metadata = self.copy_word(at, "INCLUDE_METADATA");
                if metadata {
                    self.copy_check_metadata(value, close)?;
                }
                self.copy_check_list(value, close, metadata)?;
                close + 1
            } else if self.copy_reserved(value)
                || matches!(
                    self.copy_type(value),
                    Some(TokenType::RParen | TokenType::Comma | TokenType::Eq)
                )
            {
                return Err(self.parse_error("Expected an option value"));
            } else {
                value + 1 + usize::from(self.copy_type(value) == Some(TokenType::Dash))
            };
        }
        Ok(())
    }

    /// Checks an option list: `<key> = <value>` items or bare values, none empty; a string key only first in INCLUDE_METADATA.
    fn copy_check_list(&self, open: usize, close: usize, metadata: bool) -> Result<()> {
        let mut at = open + 1;
        while at < close {
            if self.copy_type(at + 1) == Some(TokenType::Eq) {
                let key =
                    self.copy_type(at) == Some(TokenType::String) && metadata && at == open + 1;
                if !key && !self.copy_word_like(at) {
                    return Err(self.parse_error("Expected an option name"));
                }
                let value = at + 2;
                at = match self.copy_type(value) {
                    _ if value >= close => return Err(self.parse_error("Expected a value after =")),
                    Some(TokenType::Eq | TokenType::Comma) => {
                        return Err(self.parse_error("Expected a value after ="));
                    }
                    Some(TokenType::LParen) => {
                        let inner = self.copy_close(value, close)?;
                        self.copy_check_list(value, inner, false)?;
                        inner + 1
                    }
                    Some(TokenType::Dash) => value + 2,
                    _ if self.copy_type(value + 1) == Some(TokenType::Eq) => {
                        return Err(self.parse_error("Expected a value after ="));
                    }
                    _ => value + 1,
                };
            } else if matches!(self.copy_type(at), Some(TokenType::Eq | TokenType::Comma)) {
                return Err(self.parse_error("Expected an option name"));
            } else {
                at += 1;
            }
            if at < close && self.copy_type(at) == Some(TokenType::Comma) {
                at += 1;
                if at == close {
                    return Err(self.parse_error("Expected an option after ,"));
                }
            }
        }
        Ok(())
    }

    /// Refuses an INCLUDE_METADATA key that is a reserved word.
    fn copy_check_metadata(&self, open: usize, close: usize) -> Result<()> {
        let mut at = open + 1;
        while at < close {
            if self.copy_reserved(at) {
                return Err(self.parse_error("Expected a column name in INCLUDE_METADATA"));
            }
            at += 1;
            while at < close && self.copy_type(at) != Some(TokenType::Comma) {
                at += 1;
            }
            at += 1;
        }
        Ok(())
    }
}
