//! Parser ddl: Snowflake CREATE EXTERNAL LISTING and ORGANIZATION LISTING, for an optional share or application package, AS a manifest or FROM its stage location, and CREATE STREAMLIT from an optional source, checked against their grammars.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The listing kinds this parser reads; a bare LISTING keeps its own path.
const LISTING_KINDS: [&[&str]; 2] = [&["EXTERNAL", "LISTING"], &["ORGANIZATION", "LISTING"]];

impl Parser {
    /// A Snowflake CREATE EXTERNAL or ORGANIZATION LISTING; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_listing(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        let Some(kind) = LISTING_KINDS.iter().find(|kind| self.check_text_seq(kind)) else {
            return Ok(None);
        };
        self.match_text_seq(kind);
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if self.match_text_seq(&["SHARE"]) {
            let share = Expression::Table(Box::new(self.parse_object_name()?));
            properties.push(property("SHARE", None, Some(share)));
        } else if self.match_text_seq(&["APPLICATION", "PACKAGE"]) {
            let package = Expression::Table(Box::new(self.parse_object_name()?));
            properties.push(property("APPLICATION PACKAGE", None, Some(package)));
        }
        let from = !self.match_token(TokenType::As);
        if from {
            self.expect_token(TokenType::From)?;
        }
        let manifest = match from && self.is_stage_path() {
            true => self.parse_stage_path()?,
            false => self.parse_execute_specification()?,
        };
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            properties.extend(self.parse_create_property_item()?);
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: kind.join(" "),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: match from {
                true => "FROM",
                false => "AS",
            }
            .to_owned(),
            operands: vec![manifest],
            properties,
        }))))
    }

    /// A Snowflake CREATE STREAMLIT: a name, an optional FROM <source location>, then its properties; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_streamlit(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() || !self.match_text_seq(&["STREAMLIT"]) {
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
        if self.match_token(TokenType::From) {
            operands.push(match self.is_stage_path() {
                true => self.parse_stage_path()?,
                false => self.parse_string_value()?,
            });
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            properties.extend(self.parse_create_property_item()?);
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "STREAMLIT".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: String::new(),
            operands,
            properties,
        }))))
    }
}
