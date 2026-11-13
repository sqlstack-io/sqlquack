//! Parser ddl: Snowflake CREATE ALERT, with its condition and action or FROM TEMPLATE, checked against its grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The words an alert's state may be given by, without a value.
const ALERT_STATES: [&str; 2] = ["STARTED", "SUSPENDED"];

impl Parser {
    /// A Snowflake CREATE ALERT other than a clone; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_alert(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() || !self.check_text_seq(&["ALERT"]) {
            return Ok(None);
        }
        let start = self.current;
        self.skip();
        let if_not_exists = self.check(TokenType::If) && !self.check_next(TokenType::LParen);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        if self.check_identifier("CLONE") {
            self.current = start;
            return Ok(None);
        }
        let template = self.match_token(TokenType::From);
        let mut operands: Vec<Expression> = Vec::new();
        if template {
            self.expect_alter_words(&[&["TEMPLATE"]])?;
            operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
        }
        let properties = self.parse_alert_properties(template)?;
        if !template {
            operands.extend(self.parse_alert_condition_and_action()?);
        }
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the end of the statement"));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "ALERT".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: match template {
                true => "FROM TEMPLATE",
                false => "IF EXISTS",
            }
            .to_owned(),
            operands,
            properties,
        }))))
    }

    /// An alert's properties, tag list and state words, up to its IF or, from a template, its end.
    fn parse_alert_properties(&mut self, template: bool) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end()
            && !self.check(TokenType::Semicolon)
            && !(!template && self.check(TokenType::If))
        {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                self.skip();
                if !self.check_identifier("TAG") {
                    return Err(self.parse_error("Expected TAG after WITH"));
                }
            }
            if let Some(state) = ALERT_STATES
                .iter()
                .find(|word| self.check_identifier(word) && !self.check_next(TokenType::Eq))
            {
                self.skip();
                properties.push(property(state, None, None));
                continue;
            }
            properties.extend(self.parse_create_property_item()?);
        }
        Ok(properties)
    }

    /// `IF ( EXISTS ( <condition> ) ) THEN <action>`, as the condition and the action.
    fn parse_alert_condition_and_action(&mut self) -> Result<Vec<Expression>> {
        self.expect_token(TokenType::If)?;
        self.expect_token(TokenType::LParen)?;
        self.expect_token(TokenType::Exists)?;
        self.expect_token(TokenType::LParen)?;
        if self.check(TokenType::RParen) {
            return Err(self.parse_error("Expected the alert's condition"));
        }
        let condition = self.parse_statement()?;
        self.expect_token(TokenType::RParen)?;
        self.expect_token(TokenType::RParen)?;
        self.expect_alter_words(&[&["THEN"]])?;
        if self.is_at_end() || self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the alert's action"));
        }
        Ok(vec![condition, self.parse_statement()?])
    }
}
