//! Snowflake CREATE of a database, schema or table from a backup set, a share, a replica or an archive.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::TableRef;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::CreateObject;
use sqlquack_syntax::expressions::schema_objects::models::ObjectProperty;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

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
            or_alter: false,
            if_not_exists,
            name,
            source: source.to_owned(),
            operands,
            properties,
        }))))
    }

    /// Parses `MODEL ... FROM MODEL`, `{ GATEWAY | EXPERIMENT } ... FROM SPECIFICATION` or a bare `EXPERIMENT`; None for other forms.
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
            "GATEWAY" if !self.check(TokenType::From) => {
                return Err(self.parse_error("Expected FROM SPECIFICATION"));
            }
            _ if self.check(TokenType::From) => {
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
            or_alter: false,
            if_not_exists,
            name,
            source: source.to_owned(),
            operands,
            properties,
        }))))
    }

    /// Parses Snowflake `CREATE ... { STAGE | FILE FORMAT } <name>` with options and tags; None leaves CLONE and other kinds.
    pub(crate) fn parse_snowflake_create_stage_or_format(
        &mut self,
        or_replace: bool,
        or_alter: bool,
        temporary: bool,
    ) -> Result<Option<Expression>> {
        let start = self.current;
        let kind = if self.check_identifier("STAGE") {
            self.skip();
            "STAGE"
        } else if (self.check_identifier("FILE") || self.check(TokenType::File))
            && self
                .peek_nth(1)
                .is_some_and(|token| token.text.eq_ignore_ascii_case("FORMAT"))
        {
            self.skip();
            self.skip();
            "FILE FORMAT"
        } else {
            return Ok(None);
        };
        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        let name = self.parse_object_name()?;
        if self.check_identifier("CLONE") {
            self.current = start;
            return Ok(None);
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let tag = self.match_text_seq(&["WITH", "TAG"])
                || (self.check_identifier("TAG")
                    && self.check_next(TokenType::LParen)
                    && self.match_identifier("TAG"));
            if tag {
                self.expect_token(TokenType::LParen)?;
                properties.extend(self.parse_tag_list(true)?);
                self.expect_token(TokenType::RParen)?;
                continue;
            }
            let option = self.parse_simple_name()?;
            self.expect_token(TokenType::Eq)?;
            let value = self.parse_option_value()?;
            properties.push(property(
                &option.name.to_ascii_uppercase(),
                None,
                Some(value),
            ));
            if self.check(TokenType::Comma)
                && self
                    .peek_nth(1)
                    .is_some_and(|token| token.token_type != TokenType::Semicolon)
            {
                self.skip();
            }
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: kind.to_owned(),
            modifier: temporary.then(|| "TEMPORARY".to_owned()),
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: String::new(),
            operands: Vec::new(),
            properties,
        }))))
    }

    /// Parses an option value: a literal or name, a parenthesized option list, or a possibly empty value list.
    fn parse_option_value(&mut self) -> Result<Expression> {
        if !self.match_token(TokenType::LParen) {
            return self.parse_object_value();
        }
        let mut expressions: Vec<Expression> = Vec::new();
        if !self.check(TokenType::String) && self.check_next(TokenType::Eq) {
            while !self.check(TokenType::RParen) {
                let option = Expression::Identifier(self.parse_simple_name()?);
                self.expect_token(TokenType::Eq)?;
                let value = self.parse_option_value()?;
                expressions.push(Expression::Eq(Box::new(BinaryOp::new(option, value))));
                self.match_token(TokenType::Comma);
            }
        } else if !self.check(TokenType::RParen) {
            expressions.push(self.parse_object_value()?);
            while self.match_token(TokenType::Comma) {
                expressions.push(self.parse_object_value()?);
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
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
