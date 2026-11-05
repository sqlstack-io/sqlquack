//! Snowflake CREATE of a database, schema or table from a backup set, a share, a replica or an archive.

use crate::expressions::names::models::TableRef;
use crate::expressions::schema_objects::models::CreateObject;
use crate::expressions::schema_objects::models::ObjectProperty;
use crate::expressions::tree::models::Expression;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// A Snowflake CREATE from another source, after CREATE and its modifiers; None leaves every other CREATE to its own parser.
    pub(crate) fn parse_snowflake_create_from(
        &mut self,
        or_replace: bool,
        modifier: Option<&str>,
    ) -> Result<Option<Expression>> {
        let start = self.current;
        if modifier.is_none() {
            if let Some(create) = self.parse_snowflake_create_ml_object(or_replace)? {
                return Ok(Some(create));
            }
            self.current = start;
        }
        let plain = matches!(modifier, None | Some("TRANSIENT"));
        let dynamic = matches!(modifier, Some("DYNAMIC"));
        let table = self.check(TokenType::Table);
        let (kind, database) = if dynamic && table {
            ("DYNAMIC TABLE", false)
        } else if plain && table {
            ("TABLE", false)
        } else if plain && self.check_text_seq(&["DATABASE"]) {
            ("DATABASE", true)
        } else if plain && self.check_text_seq(&["SCHEMA"]) {
            ("SCHEMA", false)
        } else {
            return Ok(None);
        };
        self.skip();
        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let Ok(name) = self.parse_object_name() else {
            self.current = start;
            return Ok(None);
        };
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let source = if self.match_text_seq(&["FROM", "BACKUP"]) {
            self.expect_alter_words(&[&["SET"]])?;
            operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
            self.expect_alter_words(&[&["IDENTIFIER"]])?;
            operands.push(self.parse_string_value()?);
            "BACKUP SET"
        } else if database && self.match_text_seq(&["FROM", "SHARE"]) {
            operands.push(Expression::Table(Box::new(self.parse_two_part_name()?)));
            "SHARE"
        } else if database && self.match_text_seq(&["AS", "REPLICA"]) {
            self.expect_alter_words(&[&["OF"]])?;
            operands.push(Expression::Table(Box::new(self.parse_two_part_name()?)));
            if !self.is_at_end() && !self.check(TokenType::Semicolon) {
                properties = self.parse_alter_object_properties(true, false)?;
            }
            "REPLICA"
        } else if plain && table && self.match_text_seq(&["FROM", "ARCHIVE"]) {
            self.expect_alter_words(&[&["OF"]])?;
            let mut archive = self.parse_object_name()?;
            let explicit = self.match_token(TokenType::As);
            if explicit || !self.check(TokenType::Where) {
                archive.alias = Some(self.parse_simple_name()?);
                archive.alias_explicit_as = explicit;
            }
            operands.push(Expression::Table(Box::new(archive)));
            self.expect_token(TokenType::Where)?;
            operands.push(self.parse_expression()?);
            "ARCHIVE"
        } else {
            self.current = start;
            return Ok(None);
        };
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error(format!("Unexpected token after CREATE {kind} FROM")));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: kind.to_owned(),
            modifier: modifier.filter(|_| !dynamic).map(str::to_owned),
            or_replace,
            if_not_exists,
            name,
            source: source.to_owned(),
            operands,
            properties,
        }))))
    }

    /// `MODEL <name> [ WITH VERSION <v> ] FROM MODEL <source> [ VERSION <v> ]`, `GATEWAY <name> FROM SPECIFICATION <text>`
    /// or a bare `EXPERIMENT <name>`; None (position unchanged by the caller) for a model from a stage or a typed experiment.
    fn parse_snowflake_create_ml_object(&mut self, or_replace: bool) -> Result<Option<Expression>> {
        let kind = if self.check_identifier("MODEL") && !self.check_next_identifier("MONITOR") {
            "MODEL"
        } else if self.check_identifier("GATEWAY") {
            "GATEWAY"
        } else if self.check_identifier("EXPERIMENT") {
            "EXPERIMENT"
        } else {
            return Ok(None);
        };
        self.skip();
        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name = self.parse_object_name()?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let source = match kind {
            "MODEL" => {
                if self.match_token(TokenType::With) {
                    self.expect_alter_words(&[&["VERSION"]])?;
                    let version = Expression::Identifier(self.parse_simple_name()?);
                    properties.push(property("WITH VERSION", None, Some(version)));
                }
                if self.check(TokenType::From) && !self.check_next_identifier("MODEL") {
                    if self
                        .peek_nth(1)
                        .is_some_and(|token| token.text.starts_with('@'))
                    {
                        return Ok(None);
                    }
                    return Err(self.parse_error("Expected FROM MODEL"));
                }
                self.expect_token(TokenType::From)?;
                self.expect_alter_words(&[&["MODEL"]])?;
                operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
                if self.match_identifier("VERSION") {
                    let version = Expression::Identifier(self.parse_simple_name()?);
                    properties.push(property("VERSION", None, Some(version)));
                }
                "MODEL"
            }
            "GATEWAY" => {
                self.expect_token(TokenType::From)?;
                self.expect_alter_words(&[&["SPECIFICATION"]])?;
                if !self.check(TokenType::String) && !self.check(TokenType::DollarString) {
                    return Err(self.parse_error("Expected the specification text"));
                }
                operands.push(self.parse_primary()?);
                "SPECIFICATION"
            }
            _ if self.check(TokenType::Type) || self.check_identifier("TYPE") => return Ok(None),
            _ => "",
        };
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error(format!("Unexpected token after CREATE {kind}")));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: kind.to_owned(),
            modifier: None,
            or_replace,
            if_not_exists,
            name,
            source: source.to_owned(),
            operands,
            properties,
        }))))
    }

    /// Parse a name of at least two parts, such as `<account>.<share>`.
    fn parse_two_part_name(&mut self) -> Result<TableRef> {
        let name = self.parse_object_name()?;
        if name.schema.is_none() {
            return Err(self.parse_error("Expected a qualified name"));
        }
        Ok(name)
    }
}
