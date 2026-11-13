//! Parser ddl: Snowflake CREATE SERVICE in a compute pool from a specification or template, and CREATE APPLICATION SERVICE from an artifact repository package, checked against their grammars.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE SERVICE or APPLICATION SERVICE; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_service(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        let application = self.check_text_seq(&["APPLICATION", "SERVICE"]);
        if !application && !self.check_text_seq(&["SERVICE"]) {
            return Ok(None);
        }
        self.match_text_seq(&["APPLICATION"]);
        self.skip();
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        let (kind, source, operands, mut properties) = match application {
            true => {
                self.expect_token(TokenType::From)?;
                self.expect_alter_words(&[&["ARTIFACT", "REPOSITORY"]])?;
                let repository = Expression::Table(Box::new(self.parse_object_name()?));
                self.expect_alter_words(&[&["PACKAGE"]])?;
                let package = Expression::Identifier(self.parse_simple_name()?);
                let mut properties = vec![property("PACKAGE", None, Some(package))];
                if self.match_text_seq(&["VERSION"]) {
                    let version = Expression::Identifier(self.parse_simple_name()?);
                    properties.push(property("VERSION", None, Some(version)));
                }
                (
                    "APPLICATION SERVICE",
                    "FROM ARTIFACT REPOSITORY",
                    vec![repository],
                    properties,
                )
            }
            false => {
                self.expect_alter_words(&[&["IN", "COMPUTE", "POOL"]])?;
                let pool = Expression::Table(Box::new(self.parse_object_name()?));
                self.expect_token(TokenType::From)?;
                (
                    "SERVICE",
                    "IN COMPUTE POOL",
                    vec![pool],
                    self.parse_service_specification()?,
                )
            }
        };
        properties.extend(self.parse_service_options()?);
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: kind.to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: source.to_owned(),
            operands,
            properties,
        }))))
    }

    /// A service's specification after FROM: `[ @<stage> ] SPECIFICATION_FILE = <path>`, `[ SPECIFICATION ] <text>`, or a template file or text followed by USING ( <key> => <value> [ , ... ] ).
    fn parse_service_specification(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let staged = self.is_stage_path();
        if staged {
            properties.push(property("FROM", None, Some(self.parse_stage_path()?)));
        }
        let file_template = self.check_text_seq(&["SPECIFICATION_TEMPLATE_FILE"]);
        let template = if file_template || self.check_text_seq(&["SPECIFICATION_FILE"]) {
            let key = match file_template {
                true => "SPECIFICATION_TEMPLATE_FILE",
                false => "SPECIFICATION_FILE",
            };
            self.skip();
            self.expect_token(TokenType::Eq)?;
            properties.push(property(key, None, Some(self.parse_object_value()?)));
            file_template
        } else if staged {
            return Err(
                self.parse_error("Expected SPECIFICATION_FILE or SPECIFICATION_TEMPLATE_FILE")
            );
        } else {
            let template = self.match_text_seq(&["SPECIFICATION_TEMPLATE"]);
            if !template {
                self.match_text_seq(&["SPECIFICATION"]);
            }
            let key = match template {
                true => "SPECIFICATION_TEMPLATE",
                false => "SPECIFICATION",
            };
            properties.push(property(
                key,
                None,
                Some(self.parse_execute_specification()?),
            ));
            template
        };
        if template {
            if !self.match_token(TokenType::Using) && !self.match_text_seq(&["USING"]) {
                return Err(self.parse_error("Expected USING after the specification template"));
            }
            properties.push(property("USING", None, Some(self.parse_execute_using()?)));
        }
        Ok(properties)
    }

    /// The `<name> = <value>` options of a service, with a tag list, which may follow WITH, ending the statement.
    fn parse_service_options(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::With) && !self.check_next(TokenType::Eq) {
                self.skip();
                if !self.check_identifier("TAG") {
                    return Err(self.parse_error("Expected TAG after WITH"));
                }
            }
            let tags = self.check_identifier("TAG") && self.check_next(TokenType::LParen);
            properties.extend(self.parse_create_property_item()?);
            if tags && !self.is_at_end() && !self.check(TokenType::Semicolon) {
                return Err(self.parse_error("Expected the statement end after the tag list"));
            }
        }
        Ok(properties)
    }
}
