//! Snowflake function, procedure and object-name rules that the shared parsers do not apply.

use crate::parser::constants::{
    SNOWFLAKE_RESERVED_KEYWORDS, SNOWFLAKE_ROUTINE_NAME_LIST_PROPERTIES,
    SNOWFLAKE_ROUTINE_NAME_MAP_PROPERTIES, SNOWFLAKE_ROUTINE_NAME_PROPERTIES,
    SNOWFLAKE_ROUTINE_NUMBER_PROPERTIES, SNOWFLAKE_ROUTINE_TEXT_LIST_PROPERTIES,
    SNOWFLAKE_ROUTINE_TEXT_NAME_PROPERTIES, SNOWFLAKE_ROUTINE_TEXT_PROPERTIES,
    SNOWFLAKE_ROUTINE_WORD_PROPERTIES,
};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::ColumnDef;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::schema_objects::models::CreateObject;
use sqlquack_syntax::expressions::tables::models::ConstraintType;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::{DataType, Tuple};
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE PROCEDURE SCOPED { TEMP | TEMPORARY } TABLE <name> ( <column> <type> [ [ NOT ] NULL ] [ , ... ] ) from PROCEDURE; None when SCOPED TEMP does not follow it.
    pub(crate) fn parse_scoped_temp_table(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules()
            || !self.check(TokenType::Procedure)
            || self.text_at(1).as_deref() != Some("SCOPED")
            || !["TEMP", "TEMPORARY"]
                .iter()
                .any(|word| self.text_at(2).as_deref() == Some(*word))
        {
            return Ok(None);
        }
        self.skip();
        self.skip();
        self.skip();
        self.expect_token(TokenType::Table)?;
        let name = self.parse_iceberg_object_name()?;
        self.expect_token(TokenType::LParen)?;
        let mut columns: Vec<Expression> = Vec::new();
        loop {
            let mut column = ColumnDef::new("", DataType::Unknown);
            column.name = self.parse_iceberg_object_name()?.name;
            if self.check(TokenType::String) {
                return Err(self.parse_error("Expected a data type"));
            }
            column.data_type = self.parse_data_type()?;
            if self.match_keywords(&[TokenType::Not, TokenType::Null]) {
                column.nullable = Some(false);
                column.constraint_order.push(ConstraintType::NotNull);
            } else if self.match_token(TokenType::Null) {
                column.nullable = Some(true);
                column.constraint_order.push(ConstraintType::Null);
            }
            columns.push(Expression::ColumnDef(Box::new(column)));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "PROCEDURE SCOPED TEMPORARY TABLE".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists: false,
            name,
            source: String::new(),
            operands: vec![Expression::Tuple(Box::new(Tuple {
                expressions: columns,
            }))],
            properties: Vec::new(),
        }))))
    }

    /// A function's `FROM EXPERIMENT <name> RUN <name>` source, as its text.
    pub(crate) fn parse_experiment_run_source(&mut self) -> Result<String> {
        let start = self.current;
        self.expect_token(TokenType::From)?;
        self.skip();
        self.skip_snowflake_name()?;
        if !self.match_identifier("RUN") {
            return Err(self.parse_error("Expected RUN"));
        }
        self.skip_snowflake_name()?;
        Ok(self.tokens_to_sql(start, self.current))
    }

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
