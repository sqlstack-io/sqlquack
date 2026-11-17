//! Snowflake's COPY INTO and COPY FILES grammar, checked on the statement's tokens before it is read.

use crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::BooleanLiteral;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::operators::models::Placeholder;
use sqlquack_syntax::expressions::statements::models::Command;
use sqlquack_syntax::expressions::statements::models::CopyParameter;
use sqlquack_syntax::expressions::statements::models::CopyStmt;
use sqlquack_syntax::expressions::statements::models::Schema;
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Reads a Snowflake COPY that passes the grammar check: the typed tree when it covers the statement, else a command.
    pub(crate) fn parse_snowflake_copy(&mut self) -> Result<Expression> {
        self.check_snowflake_copy()?;
        let start = self.current;
        let end = self.copy_statement_end();
        if let Ok(copy) = self.parse_snowflake_copy_tree() {
            if self.current == end {
                return Ok(copy);
            }
        }
        self.current = start;
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

    /// Reads `[FILES] INTO <target> [FROM <source>] [<option> ...]`; the position is after COPY.
    fn parse_snowflake_copy_tree(&mut self) -> Result<Expression> {
        let copy_files = self.match_identifier("FILES");
        self.expect_token(TokenType::Into)?;
        let this = self.parse_copy_location(true)?;
        let kind = self.match_token(TokenType::From);
        let files = match kind {
            true => vec![self.parse_copy_location(false)?],
            false => Vec::new(),
        };
        let params = self.parse_snowflake_copy_options()?;
        Ok(Expression::Copy(Box::new(CopyStmt {
            this,
            kind,
            files,
            params,
            credentials: None,
            is_into: true,
            with_wrapped: false,
            where_clause: None,
            copy_files,
            by_position: false,
            columns: Vec::new(),
            target_credentials: None,
        })))
    }

    /// A COPY target or source: a stage, a string location, a bind parameter, a query in parentheses, or a table, which as the target may have a column list.
    fn parse_copy_location(&mut self, target: bool) -> Result<Expression> {
        let stage = self.check(TokenType::DAt)
            || (self.check(TokenType::Var) || self.check(TokenType::String))
                && self.peek_text().starts_with('@');
        if stage {
            return self.parse_stage_location();
        }
        if self.check(TokenType::String) {
            return Ok(Expression::Literal(Box::new(Literal::String(
                self.advance_text()?,
            ))));
        }
        if self.match_token(TokenType::Parameter) {
            return Ok(Expression::Placeholder(Placeholder { index: None }));
        }
        if self.check(TokenType::LParen) {
            return self.parse_primary();
        }
        let table = Expression::Table(Box::new(self.parse_table_ref()?));
        if !target || !self.match_token(TokenType::LParen) {
            return Ok(table);
        }
        let mut columns: Vec<Expression> = Vec::new();
        loop {
            columns.push(Expression::Identifier(
                self.expect_identifier_or_keyword_with_quoted()?,
            ));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Schema(Box::new(Schema {
            this: Some(Box::new(table)),
            expressions: columns,
        })))
    }

    /// The options after the locations: `PARTITION BY <expr>`, `NAME = value` or `NAME = ( ... )`.
    fn parse_snowflake_copy_options(&mut self) -> Result<Vec<CopyParameter>> {
        let mut params: Vec<CopyParameter> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.match_text_seq(&["PARTITION", "BY"]) {
                params.push(CopyParameter {
                    name: "PARTITION BY".to_string(),
                    value: Some(self.parse_expression()?),
                    values: Vec::new(),
                    eq: false,
                });
                continue;
            }
            let name = self.expect_identifier_or_keyword()?;
            self.expect_token(TokenType::Eq)?;
            let (value, values) = match self.match_token(TokenType::LParen) {
                true => (None, self.parse_copy_option_list(&name)?),
                false => (Some(self.parse_copy_option_value(&name)?), Vec::new()),
            };
            params.push(CopyParameter {
                name,
                value,
                values,
                eq: true,
            });
        }
        Ok(params)
    }

    /// The items of a parenthesised option value up to its closing parenthesis: `KEY = value` options, INCLUDE_METADATA's `<column> = <metadata field>` pairs, `<value> = <value>` pairs, or single values, with optional commas between them.
    fn parse_copy_option_list(&mut self, option: &str) -> Result<Vec<Expression>> {
        let mut items: Vec<Expression> = Vec::new();
        while !self.check(TokenType::RParen) {
            if self.is_at_end() {
                return Err(self.parse_error("Expected ) after the option values"));
            }
            let word = self.check(TokenType::Var)
                || self.check_keyword()
                || self.check(TokenType::QuotedIdentifier);
            let keyed = word && self.check_next(TokenType::Eq);
            let item = if keyed && option.eq_ignore_ascii_case("INCLUDE_METADATA") {
                let column = self.expect_identifier_or_keyword_with_quoted()?;
                self.expect_token(TokenType::Eq)?;
                copy_option_pair(
                    Expression::Identifier(column),
                    self.parse_copy_option_value(option)?,
                )
            } else if keyed && !self.check(TokenType::QuotedIdentifier) {
                let key = self.advance_text()?;
                self.skip();
                let value = match self.match_token(TokenType::LParen) {
                    true => Expression::Tuple(Box::new(Tuple {
                        expressions: self.parse_copy_option_list(&key)?,
                    })),
                    false => self.parse_copy_option_value(&key)?,
                };
                copy_option_pair(Expression::Var(Box::new(Var { this: key })), value)
            } else {
                let value = self.parse_copy_option_value(option)?;
                match self.match_token(TokenType::Eq) {
                    true => copy_option_pair(value, self.parse_copy_option_value(option)?),
                    false => value,
                }
            };
            items.push(item);
            self.match_token(TokenType::Comma);
        }
        self.expect_token(TokenType::RParen)?;
        Ok(items)
    }

    /// One option value: a string, a number, TRUE or FALSE, a name (quoted, qualified, or the value of a file format or storage integration option), or else a keyword such as CSV or NONE.
    fn parse_copy_option_value(&mut self, option: &str) -> Result<Expression> {
        if self.check(TokenType::String) {
            return Ok(Expression::Literal(Box::new(Literal::String(
                self.advance_text()?,
            ))));
        }
        if self.check(TokenType::Number) {
            return Ok(Expression::Literal(Box::new(Literal::Number(
                self.advance_text()?,
            ))));
        }
        if self.match_token(TokenType::True) {
            return Ok(Expression::Boolean(BooleanLiteral { value: true }));
        }
        if self.match_token(TokenType::False) {
            return Ok(Expression::Boolean(BooleanLiteral { value: false }));
        }
        let word = self.check(TokenType::Var) || self.check_keyword();
        let name = self.check(TokenType::QuotedIdentifier)
            || word
                && (self.check_next(TokenType::Dot)
                    || COPY_NAME_OPTIONS
                        .iter()
                        .any(|named| option.eq_ignore_ascii_case(named)));
        if name {
            return Ok(Expression::Table(Box::new(self.parse_table_ref()?)));
        }
        if word {
            return Ok(Expression::Var(Box::new(Var {
                this: self.advance_text()?,
            })));
        }
        self.parse_primary()
    }

    /// Refuses a Snowflake COPY that breaks `COPY [FILES] INTO <target> [FROM <source>] [<option> ...]`, FROM optional only in a table load; the position is after COPY.
    pub(crate) fn check_snowflake_copy(&self) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let end = self.copy_statement_end();
        let files = self.copy_word(self.current, "FILES");
        let mut at = self.current + usize::from(files);
        if self.copy_type(at) != Some(TokenType::Into) {
            return Err(self.parse_error("Expected INTO after COPY"));
        }
        at += 1;
        let unload = self.copy_is_stage(at) || self.copy_type(at) == Some(TokenType::String);
        at = if self.copy_is_stage(at) {
            self.copy_skip_stage(at, end)?
        } else if files && self.copy_is_quoted_stage(at) {
            at + 1
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
        } else if (files && self.copy_is_quoted_stage(at))
            || (!files
                && matches!(
                    self.copy_type(at),
                    Some(TokenType::String | TokenType::Parameter | TokenType::Placeholder)
                ))
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

    /// Whether the token at `at` is a stage written as a string literal, such as `'@"s1 Q"/p1'`.
    fn copy_is_quoted_stage(&self, at: usize) -> bool {
        self.tokens.get(at).is_some_and(|token| {
            token.token_type == TokenType::String && token.text.starts_with('@')
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
    pub(crate) fn copy_adjacent(&self, at: usize) -> bool {
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
                        && (matches!(next, Some(TokenType::From | TokenType::String))
                            || matches!(
                                self.copy_type(index - 1),
                                Some(TokenType::String | TokenType::Comma | TokenType::Select)
                            )
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

    /// Checks the options after the source: `<name> = <value>` or a parenthesized list, and a leading `PARTITION BY <expr>` (errno 1003 after another option).
    fn copy_check_options(&self, at: usize, end: usize) -> Result<()> {
        let first = at;
        let mut at = at;
        while at < end {
            if self.copy_word(at, "PARTITION") {
                if at != first {
                    return Err(self.parse_error("Expected PARTITION BY before the other options"));
                }
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

/// The options whose bare word value names an object rather than choosing a keyword.
const COPY_NAME_OPTIONS: &[&str] = &["FILE_FORMAT", "FORMAT_NAME", "STORAGE_INTEGRATION"];

/// A `<key> = <value>` item inside a parenthesised option value.
fn copy_option_pair(left: Expression, right: Expression) -> Expression {
    Expression::Eq(Box::new(BinaryOp {
        left,
        right,
        left_comments: Vec::new(),
        operator_comments: Vec::new(),
        trailing_comments: Vec::new(),
        inferred_type: None,
        span: SourceSpan::default(),
        written_as: None,
    }))
}
