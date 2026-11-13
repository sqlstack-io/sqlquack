//! Parser ddl: the properties of a Snowflake CREATE DATABASE or CREATE SCHEMA, checked against their grammar.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::tables::models::Property;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// The properties after a Snowflake database or schema name, each kept as its key and its value as written.
    pub(crate) fn parse_snowflake_container_properties(
        &mut self,
        database: bool,
    ) -> Result<Vec<Expression>> {
        let mut properties: Vec<Expression> = Vec::new();
        let mut tagged = false;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let start = self.current;
            let with = self.match_token(TokenType::With);
            let first = properties.is_empty();
            let key = if !database && with && self.match_text_seq(&["MANAGED", "ACCESS"]) {
                "MANAGED ACCESS".to_owned()
            } else if database && first && !with && self.match_text_seq(&["FROM", "LISTING"]) {
                self.check_container_value()?;
                "FROM LISTING".to_owned()
            } else if self.match_text_seq(&["TAG"]) {
                self.check_container_pairs(false)?;
                tagged = true;
                "TAG".to_owned()
            } else if with && self.match_text_seq(&["CONTACT"]) {
                self.check_container_pairs(true)?;
                tagged = true;
                "CONTACT".to_owned()
            } else if with {
                return Err(self.parse_error("Expected MANAGED ACCESS, TAG or CONTACT after WITH"));
            } else if tagged {
                return Err(self.parse_error("A property cannot follow TAG or CONTACT"));
            } else {
                let key = self.peek().text.to_ascii_uppercase();
                self.expect_external_name()?;
                self.expect_token(TokenType::Eq)?;
                match database && self.check(TokenType::LParen) {
                    true => self.check_linked_catalog()?,
                    false => self.check_container_value()?,
                }
                key
            };
            let value = self.tokens_to_sql(start, self.current);
            properties.push(Expression::Property(Box::new(Property {
                this: Box::new(Expression::Identifier(Identifier::new(key))),
                value: Some(Box::new(Expression::Raw(Raw { sql: value }))),
            })));
        }
        Ok(properties)
    }

    /// Checks `( <name> = <value> , ... )`; a contact's value is a name, not a string.
    fn check_container_pairs(&mut self, contact: bool) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            if self.check(TokenType::QuotedIdentifier) || contact && self.check(TokenType::String) {
                self.skip();
            } else {
                self.expect_external_name()?;
            }
            self.expect_token(TokenType::Eq)?;
            if contact && self.check(TokenType::String) {
                return Err(self.parse_error("Expected a name for the contact"));
            }
            self.check_container_value()?;
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks `( <key> = <value> ... )` of LINKED_CATALOG, where a value may be a parenthesized list.
    fn check_linked_catalog(&mut self) -> Result<()> {
        self.expect_token(TokenType::LParen)?;
        loop {
            self.expect_external_name()?;
            self.expect_token(TokenType::Eq)?;
            if self.match_token(TokenType::LParen) {
                loop {
                    self.check_container_value()?;
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
            } else {
                self.check_container_value()?;
            }
            if self.check(TokenType::RParen) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(())
    }

    /// Checks a value: a literal, a quoted identifier, or a word that is neither reserved nor the key of the next property.
    fn check_container_value(&mut self) -> Result<()> {
        let literal = matches!(
            self.peek().token_type,
            TokenType::String
                | TokenType::DollarString
                | TokenType::Number
                | TokenType::True
                | TokenType::False
                | TokenType::QuotedIdentifier
        );
        if literal && !self.is_at_end() {
            self.skip();
            return Ok(());
        }
        if self.check_next(TokenType::Eq) {
            return Err(self.parse_error("Expected a value"));
        }
        self.expect_external_name()
    }
}
