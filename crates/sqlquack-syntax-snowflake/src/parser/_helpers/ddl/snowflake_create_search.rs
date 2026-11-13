//! Parser ddl: Snowflake CREATE CORTEX SEARCH SERVICE, ON one search column or with TEXT and VECTOR INDEXES, checked against its grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE CORTEX SEARCH SERVICE; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_search_service(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        if self.check_text_seq(&["SEARCH", "SERVICE"]) {
            return Err(self.parse_error("Expected CORTEX SEARCH SERVICE"));
        }
        if !self.match_text_seq(&["CORTEX", "SEARCH", "SERVICE"]) {
            return Ok(None);
        }
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let source = if self.match_token(TokenType::On) {
            properties.push(property("ON", None, Some(self.search_column()?)));
            "ON"
        } else {
            self.expect_alter_words(&[&["TEXT", "INDEXES"]])?;
            properties.push(property("TEXT INDEXES", None, Some(self.search_columns()?)));
            self.expect_alter_words(&[&["VECTOR", "INDEXES"]])?;
            properties.push(property(
                "VECTOR INDEXES",
                None,
                Some(self.search_columns()?),
            ));
            "TEXT INDEXES"
        };
        if self.match_text_seq(&["PRIMARY", "KEY"]) {
            self.expect_token(TokenType::LParen)?;
            properties.push(property("PRIMARY KEY", None, Some(self.search_columns()?)));
            self.expect_token(TokenType::RParen)?;
        }
        if self.match_text_seq(&["ATTRIBUTES"]) {
            properties.push(property("ATTRIBUTES", None, Some(self.search_columns()?)));
        }
        while !self.is_at_end() && !self.check(TokenType::As) && !self.check(TokenType::Semicolon) {
            properties.extend(self.parse_create_property_item()?);
        }
        self.expect_token(TokenType::As)?;
        if !self.check(TokenType::Select) && !self.check(TokenType::With) {
            return Err(self.parse_error("Expected a query after AS"));
        }
        let query = self.parse_statement()?;
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the end of the statement"));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "CORTEX SEARCH SERVICE".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: source.to_owned(),
            operands: vec![query],
            properties,
        }))))
    }

    /// One column name: an identifier, never a string or a reserved keyword.
    fn search_column(&mut self) -> Result<Expression> {
        Ok(Expression::Identifier(self.parse_simple_name()?))
    }

    /// A comma-separated list of column names, as a tuple.
    fn search_columns(&mut self) -> Result<Expression> {
        let mut expressions = vec![self.search_column()?];
        while self.match_token(TokenType::Comma) {
            expressions.push(self.search_column()?);
        }
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }
}
