//! Parser ddl: a Snowflake CREATE DATABASE or CREATE SCHEMA with its OR ALTER and TRANSIENT, and their properties, checked against their grammar.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::tables::models::Property;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE `[ TRANSIENT ] DATABASE | SCHEMA` after CREATE [ OR REPLACE | OR ALTER ], read as the general CREATE reads it, with OR ALTER and TRANSIENT kept; None for any other CREATE.
    pub(crate) fn parse_snowflake_create_container(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        let transient = self.check_identifier("TRANSIENT");
        let kind_at = usize::from(transient);
        let kind = self.peek_nth(kind_at).map(|token| token.token_type);
        if !matches!(kind, Some(TokenType::Database | TokenType::Schema)) {
            return Ok(None);
        }
        let modifier = transient.then_some("TRANSIENT");
        if transient {
            self.skip();
        }
        if !or_alter {
            if let Some(create) = self.parse_snowflake_create_from(or_replace, modifier)? {
                return Ok(Some(create));
            }
            if let Some(create) = self.parse_snowflake_create_clone(or_replace, false, modifier)? {
                return Ok(Some(create));
            }
        }
        let mut create = match kind {
            Some(TokenType::Database) => self.parse_create_database(or_replace)?,
            _ => {
                let comments = self
                    .statement_keyword_at(self.current)
                    .map(|at| self.tokens[at].comments.clone())
                    .unwrap_or_default();
                self.parse_create_schema(comments, or_replace)?
            }
        };
        match &mut create {
            Expression::CreateDatabase(database) => {
                database.or_alter = or_alter;
                database.modifier = modifier.map(str::to_owned);
            }
            Expression::CreateSchema(schema) => {
                schema.or_alter = or_alter;
                schema.modifier = modifier.map(str::to_owned);
            }
            _ => {}
        }
        Ok(Some(create))
    }

    /// The position of the CREATE or DEFINE that starts the statement whose head reaches `at`, looking back over OR REPLACE, OR ALTER and modifiers.
    pub(crate) fn statement_keyword_at(&self, at: usize) -> Option<usize> {
        (at.saturating_sub(8)..at).rev().find(|&index| {
            matches!(
                self.tokens[index].token_type,
                TokenType::Create | TokenType::Define
            )
        })
    }

    /// The properties after a Snowflake database or schema name, each a key and its value: a literal, a name, a parenthesised LINKED_CATALOG list, or a tag or contact list of `<name> = <value>` pairs; WITH stays in the key of a tag list that writes it.
    pub(crate) fn parse_snowflake_container_properties(
        &mut self,
        database: bool,
    ) -> Result<Vec<Expression>> {
        let mut properties: Vec<Expression> = Vec::new();
        let mut tagged = false;
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let with = self.match_token(TokenType::With);
            let first = properties.is_empty();
            let (key, value) = if !database && with && self.match_text_seq(&["MANAGED", "ACCESS"]) {
                (Identifier::new("MANAGED ACCESS"), None)
            } else if database && first && !with && self.match_text_seq(&["FROM", "LISTING"]) {
                (
                    Identifier::new("FROM LISTING"),
                    Some(self.parse_container_value()?),
                )
            } else if self.match_text_seq(&["TAG"]) {
                tagged = true;
                let key = match with {
                    true => "WITH TAG",
                    false => "TAG",
                };
                (
                    Identifier::new(key),
                    Some(self.parse_container_pairs(false)?),
                )
            } else if with && self.match_text_seq(&["CONTACT"]) {
                tagged = true;
                (
                    Identifier::new("CONTACT"),
                    Some(self.parse_container_pairs(true)?),
                )
            } else if with {
                return Err(self.parse_error("Expected MANAGED ACCESS, TAG or CONTACT after WITH"));
            } else if tagged {
                return Err(self.parse_error("A property cannot follow TAG or CONTACT"));
            } else {
                let mut key = self.parse_external_name()?;
                if !key.quoted {
                    key.name = key.name.to_ascii_uppercase();
                }
                self.expect_token(TokenType::Eq)?;
                let value = match database && self.check(TokenType::LParen) {
                    true => self.parse_linked_catalog()?,
                    false => self.parse_container_value()?,
                };
                (key, Some(value))
            };
            properties.push(Expression::Property(Box::new(Property {
                this: Box::new(Expression::Identifier(key)),
                value: value.map(Box::new),
            })));
        }
        Ok(properties)
    }

    /// `( <name> = <value> , ... )`, each pair a `=` node; a contact's value is a name, not a string, and its purpose may be a string.
    fn parse_container_pairs(&mut self, contact: bool) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut pairs: Vec<Expression> = Vec::new();
        loop {
            let name = match contact && self.check(TokenType::String) {
                true => self.parse_primary()?,
                false => Expression::Identifier(self.parse_external_name()?),
            };
            self.expect_token(TokenType::Eq)?;
            if contact && self.check(TokenType::String) {
                return Err(self.parse_error("Expected a name for the contact"));
            }
            let value = self.parse_container_value()?;
            pairs.push(Expression::PropertyEQ(Box::new(BinaryOp::new(name, value))));
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions: pairs })))
    }

    /// LINKED_CATALOG's `( <key> = <value> ... )`, each pair a `=` node whose value may be a parenthesised list.
    fn parse_linked_catalog(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut pairs: Vec<Expression> = Vec::new();
        loop {
            let key = Expression::Identifier(self.parse_external_name()?);
            self.expect_token(TokenType::Eq)?;
            let value = if self.match_token(TokenType::LParen) {
                let mut items: Vec<Expression> = Vec::new();
                loop {
                    items.push(self.parse_container_value()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                Expression::Tuple(Box::new(Tuple { expressions: items }))
            } else {
                self.parse_container_value()?
            };
            pairs.push(Expression::PropertyEQ(Box::new(BinaryOp::new(key, value))));
            if self.check(TokenType::RParen) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions: pairs })))
    }

    /// A value: a literal, a quoted identifier, or a word that is neither reserved nor the key of the next property, as a name with its quoting.
    fn parse_container_value(&mut self) -> Result<Expression> {
        let literal = matches!(
            self.peek().token_type,
            TokenType::String
                | TokenType::DollarString
                | TokenType::Number
                | TokenType::True
                | TokenType::False
        );
        if literal && !self.is_at_end() {
            let start = self.current;
            let value = self.parse_primary()?;
            if self.current == start + 1 {
                return Ok(value);
            }
            self.current = start;
            return Err(self.parse_error("Expected a single literal value"));
        }
        if self.check(TokenType::QuotedIdentifier) && !self.is_at_end() {
            return Ok(Expression::Identifier(self.parse_external_name()?));
        }
        if self.check_next(TokenType::Eq) {
            return Err(self.parse_error("Expected a value"));
        }
        Ok(Expression::Identifier(self.parse_external_name()?))
    }
}
