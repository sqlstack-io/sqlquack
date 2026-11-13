//! Parser ddl: Snowflake CREATE DATA MOVEMENT RULE, with its type, MAX_ROWS body and comment, checked against its grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE DATA MOVEMENT RULE; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_movement_rule(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules()
            || !self.match_text_seq(&["DATA", "MOVEMENT", "RULE"])
        {
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
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check_identifier("MAX_ROWS") && self.check_next(TokenType::As) {
                properties.push(self.parse_max_rows_body()?);
                continue;
            }
            if self.check(TokenType::Comment) && self.check_next(TokenType::Eq) {
                self.skip();
                self.skip();
                properties.push(property("COMMENT", None, Some(self.parse_string_value()?)));
                continue;
            }
            properties.extend(self.parse_create_property_item()?);
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "DATA MOVEMENT RULE".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: String::new(),
            operands: Vec::new(),
            properties,
        }))))
    }
}
