//! Snowflake function, procedure and object-name rules that the shared parsers do not apply.

use crate::parser::constants::{
    SNOWFLAKE_RESERVED_KEYWORDS, SNOWFLAKE_ROUTINE_NAME_LIST_PROPERTIES,
    SNOWFLAKE_ROUTINE_NAME_MAP_PROPERTIES, SNOWFLAKE_ROUTINE_NAME_PROPERTIES,
    SNOWFLAKE_ROUTINE_NUMBER_PROPERTIES, SNOWFLAKE_ROUTINE_TEXT_LIST_PROPERTIES,
    SNOWFLAKE_ROUTINE_TEXT_MAP_PROPERTIES, SNOWFLAKE_ROUTINE_TEXT_NAME_PROPERTIES,
    SNOWFLAKE_ROUTINE_TEXT_PROPERTIES, SNOWFLAKE_ROUTINE_WORD_PROPERTIES,
};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

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
            || self.match_identifier("IMMUTABLE")
            || self.match_identifier("MEMOIZABLE");
        if phrase {
            return Ok(Some(self.tokens_to_sql(start, self.current)));
        }
        if !self.check_next(TokenType::Eq) {
            return Ok(None);
        }
        let key = self.peek_text().to_ascii_uppercase();
        let listed = |keys: &[&str]| keys.contains(&key.as_str());
        let words = listed(SNOWFLAKE_ROUTINE_WORD_PROPERTIES);
        let value: fn(&mut Self) -> Result<()> =
            if words && listed(SNOWFLAKE_ROUTINE_TEXT_PROPERTIES) {
                Self::expect_routine_text_or_word
            } else if words && listed(SNOWFLAKE_ROUTINE_TEXT_LIST_PROPERTIES) {
                |parser| parser.skip_routine_list(Self::expect_routine_text_or_word)
            } else if listed(SNOWFLAKE_ROUTINE_TEXT_NAME_PROPERTIES) {
                Self::skip_routine_name_or_text_value
            } else if listed(SNOWFLAKE_ROUTINE_TEXT_PROPERTIES) {
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

    /// A string value, or a bare word that is neither reserved nor the key of the next property.
    fn expect_routine_text_or_word(&mut self) -> Result<()> {
        if !self.check(TokenType::String) && self.check_next(TokenType::Eq) {
            return Err(self.parse_error("Expected a string or a word"));
        }
        self.expect_routine_key()
    }

    /// A string or a bare word that is not reserved, such as the key of a map entry.
    fn expect_routine_key(&mut self) -> Result<()> {
        if self.match_token(TokenType::String) {
            return Ok(());
        }
        let word = matches!(
            self.peek().token_type,
            TokenType::Identifier | TokenType::Var
        ) || self.peek().token_type.is_keyword();
        if self.is_at_end() || !word || self.is_snowflake_reserved_keyword() {
            return Err(self.parse_error("Expected a string or a word"));
        }
        self.skip();
        Ok(())
    }

    /// A name, or a string naming the object.
    fn skip_routine_name_or_text(&mut self) -> Result<()> {
        match self.match_token(TokenType::String) {
            true => Ok(()),
            false => self.skip_snowflake_name(),
        }
    }

    /// The value of a property whose names may be strings: one name, a list of names, or a map to names.
    fn skip_routine_name_or_text_value(&mut self) -> Result<()> {
        if !self.check(TokenType::LParen) {
            return self.skip_routine_name_or_text();
        }
        let map = self
            .tokens
            .get(self.current + 2)
            .is_some_and(|token| token.token_type == TokenType::Eq);
        match map {
            true => self.skip_routine_list(Self::skip_routine_name_entry),
            false => self.skip_routine_list(Self::skip_routine_name_or_text),
        }
    }

    /// In a Snowflake parse, whether the next tokens are `<key> = <value>` with a value that is not a string.
    pub(crate) fn names_snowflake_word_value(&self) -> bool {
        self.is_snowflake()
            && self.check_next(TokenType::Eq)
            && self
                .tokens
                .get(self.current + 2)
                .is_some_and(|token| token.token_type != TokenType::String)
    }

    /// In a Snowflake parse, `NOT NULL` or `NULL` after a function's return type, as its source text.
    pub(crate) fn match_snowflake_return_nullability(&mut self) -> Option<String> {
        if !self.is_snowflake() {
            return None;
        }
        let start = self.current;
        let matched = self.match_keywords(&[TokenType::Not, TokenType::Null])
            || (self.check(TokenType::Null)
                && !self.check_next(TokenType::On)
                && self.match_token(TokenType::Null));
        matched.then(|| self.tokens_to_sql(start, self.current))
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

    /// `'<key>' = <name>`, where the key may be a bare word and the name a string.
    fn skip_routine_name_entry(&mut self) -> Result<()> {
        self.expect_routine_key()?;
        self.expect_token(TokenType::Eq)?;
        self.skip_routine_name_or_text()
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

    /// In a Snowflake parse, refuses `TABLE ( ... )` with a reserved column name.
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
            if opens_column && reserved {
                return Err(self.parse_error("Expected a column name in TABLE ( ... )"));
            }
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
