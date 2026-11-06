//! Parser ddl: Snowflake CREATE EXTERNAL TABLE, checked against its grammar and kept as written.

use crate::expressions::statements::models::Raw;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::SNOWFLAKE_EXTERNAL_TABLE_REQUIRED;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE [OR REPLACE] EXTERNAL TABLE, checked and kept as written; None for any other CREATE.
    pub(crate) fn parse_snowflake_create_external_table(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if or_alter || !self.check_text_seq(&["EXTERNAL", "TABLE"]) {
            return Ok(None);
        }
        let start = self.current;
        self.match_text_seq(&["EXTERNAL", "TABLE"]);
        if self.check(TokenType::If)
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        self.parse_object_name()?;
        let template = self.match_text_seq(&["USING", "TEMPLATE"]);
        match template {
            true => self.check_external_template()?,
            false => self.check_external_columns()?,
        }
        self.check_external_properties(!template)?;
        let mut sql = String::from("CREATE");
        if or_replace {
            sql.push_str(" OR REPLACE");
        }
        sql.push(' ');
        sql.push_str(&self.tokens_to_sql(start, self.current));
        Ok(Some(Expression::Raw(Raw { sql })))
    }

    /// Checks `( <query> )`, `$<variable>` or a string after USING TEMPLATE.
    pub(super) fn check_external_template(&mut self) -> Result<()> {
        if self.match_token(TokenType::String) {
            return Ok(());
        }
        let variable = matches!(
            self.peek().token_type,
            TokenType::Var | TokenType::Identifier
        ) && self.peek().text.len() > 1
            && self.peek().text.starts_with('$');
        if variable {
            self.skip();
            return Ok(());
        }
        if self.check(TokenType::Dollar) {
            let variable = self.peek_nth(1).filter(|token| {
                token.span.start == self.peek().span.end
                    && (matches!(token.token_type, TokenType::Var | TokenType::Identifier)
                        || token.token_type.is_keyword())
            });
            if variable.is_none() {
                return Err(self.parse_error("Expected a variable after $"));
            }
            self.skip();
            self.skip();
            return Ok(());
        }
        self.expect_token(TokenType::LParen)?;
        if !self.check(TokenType::Select) && !self.check(TokenType::With) {
            return Err(self.parse_error("Expected a query in USING TEMPLATE ( )"));
        }
        self.parse_statement()?;
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks `( <name> [<type>] AS <expr> [<constraint>] [ , ... ] )`, and the bare `<name>`, `<name> REFERENCES ...` and `REFERENCES ...` Snowflake parses before refusing.
    fn check_external_columns(&mut self) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            let bare_references = self.check_text_seq(&["REFERENCES"])
                && !self.check_next(TokenType::Comma)
                && !self.check_next(TokenType::RParen);
            if bare_references {
                self.check_external_constraint()?;
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                continue;
            }
            self.expect_external_name()?;
            if self.check(TokenType::Comma)
                || self.check(TokenType::RParen)
                || self.check_text_seq(&["REFERENCES"])
            {
                self.check_external_constraint()?;
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                continue;
            }
            if !self.check(TokenType::As) {
                if self.check(TokenType::String) {
                    return Err(self.parse_error("Expected a data type"));
                }
                self.parse_data_type()?;
            }
            self.expect_token(TokenType::As)?;
            let expression_start = self.current;
            self.parse_expression()?;
            let negated_constraint = self.tokens
                [expression_start..(self.current + 1).min(self.tokens.len())]
                .windows(2)
                .any(|pair| {
                    pair[0].token_type == TokenType::Not && pair[1].token_type == TokenType::Unique
                });
            if negated_constraint {
                return Err(self.parse_error("Expected a column expression before the constraint"));
            }
            self.check_external_constraint()?;
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks an optional inline `[CONSTRAINT <name>] UNIQUE | PRIMARY KEY | [FOREIGN KEY] REFERENCES <table> [( <columns> )]`.
    fn check_external_constraint(&mut self) -> Result<()> {
        let named = self.match_text_seq(&["CONSTRAINT"]);
        if named {
            self.expect_external_name()?;
        }
        if self.match_text_seq(&["UNIQUE"]) || self.match_text_seq(&["PRIMARY", "KEY"]) {
            return Ok(());
        }
        let foreign = self.match_text_seq(&["FOREIGN", "KEY"]);
        if !self.match_text_seq(&["REFERENCES"]) {
            return match named || foreign {
                true => Err(self.parse_error("Expected a constraint")),
                false => Ok(()),
            };
        }
        self.parse_object_name()?;
        if self.match_token(TokenType::LParen) {
            loop {
                self.expect_external_name()?;
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
        }
        Ok(())
    }

    /// Checks one name: an identifier, or a word Snowflake does not reserve.
    pub(super) fn expect_external_name(&mut self) -> Result<()> {
        let token_type = self.peek().token_type;
        let name_like = matches!(
            token_type,
            TokenType::Identifier | TokenType::QuotedIdentifier | TokenType::Var
        ) || token_type.is_keyword();
        if self.is_at_end() || !name_like || self.is_snowflake_reserved_keyword() {
            return Err(self.parse_error("Expected a name"));
        }
        self.skip();
        Ok(())
    }

    /// Checks the properties after the columns or template; the column form needs LOCATION and FILE_FORMAT.
    fn check_external_properties(&mut self, columns: bool) -> Result<()> {
        let mut keys: Vec<String> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if let Some(key) = self.check_external_property()? {
                keys.push(key);
            }
        }
        let missing = SNOWFLAKE_EXTERNAL_TABLE_REQUIRED
            .iter()
            .any(|required| !keys.contains(&required.to_string()));
        if columns && missing {
            return Err(self.parse_error("Expected LOCATION and FILE_FORMAT"));
        }
        Ok(())
    }

    /// Checks one property, returning the key of a `<key> = <value>` one.
    fn check_external_property(&mut self) -> Result<Option<String>> {
        if self.match_text_seq(&["COPY", "GRANTS"]) {
            return Ok(None);
        }
        if self.match_keywords(&[TokenType::Partition, TokenType::By]) {
            self.expect_token(TokenType::LParen)?;
            self.parse_expression_list()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(None);
        }
        if self.match_text_seq(&["ROW", "ACCESS", "POLICY"]) {
            self.parse_object_name()?;
            self.expect_token(TokenType::On)?;
            self.expect_token(TokenType::LParen)?;
            while !self.check(TokenType::RParen) {
                self.expect_external_name()?;
                self.match_token(TokenType::Comma);
            }
            self.expect_token(TokenType::RParen)?;
            return Ok(None);
        }
        if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
            self.check_external_tags()?;
            return Ok(None);
        }
        let key = self.peek().text.to_ascii_uppercase();
        self.expect_external_name()?;
        self.expect_token(TokenType::Eq)?;
        if key == SNOWFLAKE_EXTERNAL_TABLE_REQUIRED[0] {
            self.check_external_stage()?;
        } else {
            self.check_external_value()?;
        }
        Ok(Some(key))
    }

    fn check_external_tags(&mut self) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            self.parse_object_name()?;
            self.expect_token(TokenType::Eq)?;
            match self.match_token(TokenType::String) {
                true => {}
                false => self.check_external_word()?,
            }
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks a stage location: one unbroken run of tokens that starts with @, holds no string and quotes no blank.
    fn check_external_stage(&mut self) -> Result<()> {
        let at = self.check(TokenType::DAt) || self.peek().text.starts_with('@');
        if self.is_at_end() || !at {
            return Err(self.parse_error("Expected a stage location"));
        }
        let blank_quoted = self.check(TokenType::DAt)
            && self.peek_nth(1).is_some_and(|token| {
                token.token_type == TokenType::QuotedIdentifier
                    && token.text.contains(char::is_whitespace)
            });
        if blank_quoted {
            return Err(self.parse_error("Expected a stage name after @"));
        }
        loop {
            self.skip();
            let adjacent = !self.is_at_end()
                && self.tokens[self.current - 1].span.end == self.peek().span.start;
            if !adjacent {
                return Ok(());
            }
            if self.check(TokenType::String) {
                return Err(self.parse_error("A stage name part cannot be a string literal"));
            }
        }
    }

    /// Checks a value: a literal, a word, or a parenthesized list of values and `<key> = <value>` pairs.
    fn check_external_value(&mut self) -> Result<()> {
        if !self.match_token(TokenType::LParen) {
            return self.check_external_scalar();
        }
        while !self.check(TokenType::RParen) {
            if self.check_next(TokenType::Eq) {
                self.expect_external_name()?;
                self.skip();
                self.check_external_value()?;
            } else {
                self.check_external_scalar()?;
            }
            self.match_token(TokenType::Comma);
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    fn check_external_scalar(&mut self) -> Result<()> {
        let literal = matches!(
            self.peek().token_type,
            TokenType::String | TokenType::Number | TokenType::True | TokenType::False
        );
        match literal && !self.is_at_end() {
            true => {
                self.skip();
                Ok(())
            }
            false => self.check_external_word(),
        }
    }

    /// Checks a bare word value, which cannot be the key of the next property.
    fn check_external_word(&mut self) -> Result<()> {
        if self.check(TokenType::QuotedIdentifier) || self.check_next(TokenType::Eq) {
            return Err(self.parse_error("Expected a value"));
        }
        self.expect_external_name()
    }
}
