//! Snowflake function, procedure and object-name rules that the shared parsers do not apply.

use crate::expressions::names::models::TableRef;
use crate::expressions::schema_objects::models::FunctionParameter;
use crate::parser::constants::{
    SNOWFLAKE_RESERVED_KEYWORDS, SNOWFLAKE_ROUTINE_NAME_LIST_PROPERTIES,
    SNOWFLAKE_ROUTINE_NAME_MAP_PROPERTIES, SNOWFLAKE_ROUTINE_NAME_PROPERTIES,
    SNOWFLAKE_ROUTINE_NUMBER_PROPERTIES, SNOWFLAKE_ROUTINE_TEXT_LIST_PROPERTIES,
    SNOWFLAKE_ROUTINE_TEXT_MAP_PROPERTIES, SNOWFLAKE_ROUTINE_TEXT_PROPERTIES,
};
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// One Snowflake routine property or option phrase as its source text; None when the next tokens are not one.
    pub(crate) fn parse_snowflake_routine_property(&mut self) -> Result<Option<String>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        let start = self.current;
        let phrase = self.match_text_seq(&["COPY", "GRANTS"])
            || self.match_text_seq(&["CALLED", "ON", "NULL", "INPUT"])
            || self.match_text_seq(&["RETURNS", "NULL", "ON", "NULL", "INPUT"])
            || self.match_identifier("STRICT")
            || self.match_identifier("VOLATILE")
            || self.match_identifier("IMMUTABLE");
        if phrase {
            return Ok(Some(self.tokens_to_sql(start, self.current)));
        }
        if !self.check_next(TokenType::Eq) {
            return Ok(None);
        }
        let key = self.peek_text().to_ascii_uppercase();
        let listed = |keys: &[&str]| keys.contains(&key.as_str());
        let value: fn(&mut Self) -> Result<()> = if listed(SNOWFLAKE_ROUTINE_TEXT_PROPERTIES) {
            Self::expect_routine_text
        } else if listed(SNOWFLAKE_ROUTINE_NAME_PROPERTIES) {
            Self::skip_snowflake_name
        } else if listed(SNOWFLAKE_ROUTINE_NUMBER_PROPERTIES) {
            |parser| parser.expect_token(TokenType::Number).map(|_| ())
        } else if listed(SNOWFLAKE_ROUTINE_TEXT_LIST_PROPERTIES) {
            |parser| parser.skip_routine_list(Self::expect_routine_text)
        } else if listed(SNOWFLAKE_ROUTINE_NAME_LIST_PROPERTIES) {
            |parser| parser.skip_routine_list(Self::skip_snowflake_name)
        } else if listed(SNOWFLAKE_ROUTINE_NAME_MAP_PROPERTIES) {
            |parser| parser.skip_routine_list(Self::skip_routine_name_entry)
        } else if listed(SNOWFLAKE_ROUTINE_TEXT_MAP_PROPERTIES) {
            |parser| parser.skip_routine_list(Self::skip_routine_text_entry)
        } else {
            return Ok(None);
        };
        self.skip();
        self.skip();
        value(self)?;
        Ok(Some(self.tokens_to_sql(start, self.current)))
    }

    /// A string value.
    fn expect_routine_text(&mut self) -> Result<()> {
        self.expect_token(TokenType::String).map(|_| ())
    }

    /// `( <item> , ... )`, possibly empty.
    fn skip_routine_list(&mut self, item: fn(&mut Self) -> Result<()>) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        if !self.check(TokenType::RParen) {
            item(self)?;
            while self.match_token(TokenType::Comma) {
                item(self)?;
            }
        }
        self.expect_token(TokenType::RParen).map(|_| ())
    }

    /// `'<key>' = <name>`.
    fn skip_routine_name_entry(&mut self) -> Result<()> {
        self.expect_routine_text()?;
        self.expect_token(TokenType::Eq)?;
        self.skip_snowflake_name()
    }

    /// `'<key>' = '<value>'`.
    fn skip_routine_text_entry(&mut self) -> Result<()> {
        self.expect_routine_text()?;
        self.expect_token(TokenType::Eq)?;
        self.expect_routine_text()
    }

    /// In a Snowflake parse, refuses an object name with a reserved word or a string as a part.
    pub(crate) fn check_snowflake_object_name(&mut self) -> Result<()> {
        if self.is_snowflake() {
            let start = self.current;
            self.skip_snowflake_name()?;
            self.current = start;
        }
        Ok(())
    }

    /// In a Snowflake parse, refuses `TABLE ( ... )` with a reserved column name or a column lacking its type.
    pub(crate) fn check_snowflake_table_columns(&self) -> Result<()> {
        if !self.is_snowflake()
            || !self.check(TokenType::Table)
            || !self.check_next(TokenType::LParen)
        {
            return Ok(());
        }
        let mut depth = 0;
        for (offset, token) in self.tokens[self.current + 1..].iter().enumerate() {
            match token.token_type {
                TokenType::LParen => depth += 1,
                TokenType::RParen if depth == 1 => return Ok(()),
                TokenType::RParen => depth -= 1,
                _ => {}
            }
            let opens_column =
                depth == 1 && matches!(token.token_type, TokenType::LParen | TokenType::Comma);
            let next = self.tokens.get(self.current + offset + 2);
            let reserved = next.is_some_and(|next| {
                next.token_type != TokenType::QuotedIdentifier
                    && SNOWFLAKE_RESERVED_KEYWORDS
                        .iter()
                        .any(|word| next.text.eq_ignore_ascii_case(word))
            });
            let untyped = next.is_some_and(|next| next.token_type != TokenType::RParen)
                && self
                    .tokens
                    .get(self.current + offset + 3)
                    .is_some_and(|after| {
                        matches!(after.token_type, TokenType::Comma | TokenType::RParen)
                    });
            if opens_column && (reserved || untyped) {
                return Err(self.parse_error("Expected a column name and type in TABLE ( ... )"));
            }
        }
        Ok(())
    }

    /// In a Snowflake parse, refuses a routine argument that lacks its name or its type.
    pub(crate) fn check_snowflake_routine_arguments(
        &self,
        arguments: &[FunctionParameter],
    ) -> Result<()> {
        if self.is_snowflake() && arguments.iter().any(|argument| argument.name.is_none()) {
            return Err(self.parse_error("Expected a name and a type for each argument"));
        }
        Ok(())
    }

    /// In a Snowflake parse, refuses a ROW ACCESS POLICY body that is not `<name> ON ( <column> , ... )`.
    pub(crate) fn check_snowflake_row_access_policy(&mut self) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let start = self.current;
        self.skip_snowflake_name()?;
        self.expect_token(TokenType::On)?;
        self.skip_routine_list(Self::skip_snowflake_name)?;
        self.current = start;
        Ok(())
    }

    /// A view's name, whose parts may not be strings or Snowflake reserved words in a Snowflake parse.
    pub(crate) fn parse_view_name(&mut self) -> Result<TableRef> {
        self.check_snowflake_object_name()?;
        self.parse_table_ref()
    }

    /// Whether AS follows; a Snowflake view always needs AS before its query.
    pub(crate) fn match_view_as(&mut self) -> Result<bool> {
        let has_as = self.match_token(TokenType::As);
        if !has_as && self.is_snowflake() {
            return Err(self.parse_error("Expected AS and the view's query"));
        }
        Ok(has_as)
    }
}
