//! Parser ddl: Snowflake CREATE POSTGRES INSTANCE, with its FORK source, time travel, properties and tags, checked against its grammar.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE POSTGRES INSTANCE; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_postgres_instance(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        if self.check_identifier("INSTANCE") {
            return Err(self.parse_error("Snowflake has no CREATE INSTANCE"));
        }
        if !self.match_text_seq(&["POSTGRES", "INSTANCE"]) {
            return Ok(None);
        }
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        let mut operands: Vec<Expression> = Vec::new();
        let fork = self.match_identifier("FORK");
        if fork {
            operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
            if let Some(time_travel) = self.parse_time_travel(&[&["TIMESTAMP"], &["OFFSET"]])? {
                operands.push(time_travel);
            }
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_statement_end_at(0) {
            if self.check(TokenType::With) && self.check_next_identifier("TAG") {
                self.skip();
            }
            properties.extend(self.parse_create_property_item()?);
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "POSTGRES INSTANCE".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: if fork { "FORK" } else { "" }.to_owned(),
            operands,
            properties,
        }))))
    }
}
