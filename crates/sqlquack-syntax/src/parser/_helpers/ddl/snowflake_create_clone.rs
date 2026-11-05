//! Snowflake CREATE ... CLONE and CREATE ... AS REPLICA OF for kinds without a CREATE node of their own.

use crate::expressions::names::models::Identifier;
use crate::expressions::schema_objects::models::CreateObject;
use crate::expressions::schema_objects::models::ObjectProperty;
use crate::expressions::statements::models::HistoricalData;
use crate::expressions::tables::models::Property;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// The options a clone documents after its source.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloneOptions {
    /// None: ALERT, STAGE, FILE FORMAT, SEQUENCE, DATABASE ROLE, EVENT TABLE.
    Bare,
    /// COPY GRANTS, COPY TAGS and COPY = ( ... ): tables and streams.
    Copy,
    /// The copy options, TARGET_LAG, WAREHOUSE, EXECUTE AS USER and USE SECONDARY ROLES: dynamic tables.
    Dynamic,
    /// IGNORE TABLES WITH INSUFFICIENT DATA RETENTION, IGNORE HYBRID TABLES, INCLUDE INTERNAL STAGES.
    Container,
}

impl Parser {
    /// A clone's `{ AT | BEFORE } ( { TIMESTAMP | OFFSET | STATEMENT } => <expr> )`, if present.
    pub(crate) fn parse_clone_time_travel(&mut self) -> Result<Option<Expression>> {
        if !self.check_identifier("AT") && !self.check(TokenType::Before) {
            return Ok(None);
        }
        let this = self.advance_text()?.to_ascii_uppercase();
        self.expect_token(TokenType::LParen)?;
        let kind = self.expect_alter_words(&[&["TIMESTAMP"], &["OFFSET"], &["STATEMENT"]])?;
        self.expect_token(TokenType::FArrow)?;
        let expression = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::HistoricalData(Box::new(HistoricalData {
            this: Box::new(Expression::Identifier(Identifier::new(this))),
            kind,
            expression: Box::new(expression),
        }))))
    }

    /// The options after a clone's source and time travel, to the end; any other word is an error.
    pub(crate) fn parse_clone_options(
        &mut self,
        options: CloneOptions,
    ) -> Result<Vec<ObjectProperty>> {
        let copy = matches!(options, CloneOptions::Copy | CloneOptions::Dynamic);
        let dynamic = options == CloneOptions::Dynamic;
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if copy && self.match_token(TokenType::Copy) {
                if self.match_token(TokenType::Eq) {
                    self.expect_token(TokenType::LParen)?;
                    let mut kinds: Vec<Expression> = Vec::new();
                    while !self.check(TokenType::RParen) || !kinds.is_empty() {
                        let kind =
                            self.expect_alter_words(&[&["COMMENTS"], &["TAGS"], &["GRANTS"]])?;
                        kinds.push(Expression::Identifier(Identifier::new(kind)));
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    properties.push(property(
                        "COPY",
                        None,
                        Some(Expression::Tuple(Box::new(Tuple { expressions: kinds }))),
                    ));
                } else {
                    let kind = self.expect_alter_words(&[&["GRANTS"], &["TAGS"]])?;
                    properties.push(property(&format!("COPY {kind}"), None, None));
                }
            } else if dynamic && self.match_text_seq(&["TARGET_LAG"]) {
                self.expect_token(TokenType::Eq)?;
                let value = if self.match_text_seq(&["DOWNSTREAM"]) {
                    Expression::Identifier(Identifier::new("DOWNSTREAM"))
                } else {
                    self.parse_string_value()?
                };
                properties.push(property("TARGET_LAG", None, Some(value)));
            } else if dynamic && self.match_text_seq(&["WAREHOUSE"]) {
                self.expect_token(TokenType::Eq)?;
                let warehouse = self.parse_object_name()?;
                properties.push(property(
                    "WAREHOUSE",
                    None,
                    Some(Expression::Table(Box::new(warehouse))),
                ));
            } else if dynamic && self.match_text_seq(&["EXECUTE"]) {
                self.expect_alter_words(&[&["AS", "USER"]])?;
                let user = self.parse_object_name()?;
                properties.push(property(
                    "EXECUTE AS USER",
                    None,
                    Some(Expression::Table(Box::new(user))),
                ));
            } else if dynamic && self.match_text_seq(&["USE", "SECONDARY", "ROLES"]) {
                let value = if self.match_text_seq(&["ALL"]) || self.match_text_seq(&["NONE"]) {
                    Expression::Identifier(Identifier::new(
                        self.previous_text().to_ascii_uppercase(),
                    ))
                } else {
                    let mut roles: Vec<Expression> = Vec::new();
                    loop {
                        roles.push(Expression::Table(Box::new(self.parse_object_name()?)));
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    Expression::Tuple(Box::new(Tuple { expressions: roles }))
                };
                properties.push(property("USE SECONDARY ROLES", None, Some(value)));
            } else if options == CloneOptions::Container
                && (self.check_text_seq(&["IGNORE"]) || self.check_text_seq(&["INCLUDE"]))
            {
                let words = self.expect_alter_words(&[
                    &[
                        "IGNORE",
                        "TABLES",
                        "WITH",
                        "INSUFFICIENT",
                        "DATA",
                        "RETENTION",
                    ],
                    &["IGNORE", "HYBRID", "TABLES"],
                    &["INCLUDE", "INTERNAL", "STAGES"],
                ])?;
                properties.push(property(&words, None, None));
            } else {
                return Err(self.parse_error("Unexpected token after the CLONE source"));
            }
        }
        Ok(properties)
    }

    /// The clone options as table or schema properties: `COPY TAGS` or `COPY = (COMMENTS, TAGS)`.
    pub(crate) fn clone_options_as_properties(options: Vec<ObjectProperty>) -> Vec<Expression> {
        options
            .into_iter()
            .map(|option| {
                Expression::Property(Box::new(Property {
                    this: Box::new(Expression::Identifier(Identifier::new(option.kind))),
                    value: option.value.map(Box::new),
                }))
            })
            .collect()
    }

    /// A Snowflake database or schema clone's IGNORE and INCLUDE options, when `cloned`.
    pub(crate) fn parse_clone_container_properties(
        &mut self,
        cloned: bool,
    ) -> Result<Vec<Expression>> {
        if !(cloned && self.is_snowflake()) {
            return Ok(Vec::new());
        }
        Ok(Self::clone_options_as_properties(
            self.parse_clone_options(CloneOptions::Container)?,
        ))
    }

    /// `CREATE <kind> [ IF NOT EXISTS ] <name> CLONE <source> ...` for kinds without their own node, or None.
    pub(crate) fn parse_snowflake_create_clone(
        &mut self,
        or_replace: bool,
        temporary: bool,
        table_modifier: Option<&str>,
    ) -> Result<Option<Expression>> {
        let start = self.current;
        if table_modifier.is_none() && !temporary && !or_replace && self.statement_names_a_replica()
        {
            if let Some(create) = self.parse_snowflake_create_replica()? {
                return Ok(Some(create));
            }
        }
        if table_modifier.is_none() && !temporary && self.check_text_seq(&["ROLE"]) {
            if self
                .tokens
                .get(self.current + 2)
                .is_some_and(|t| t.text.eq_ignore_ascii_case("CLONE"))
            {
                return Err(
                    self.parse_error("A role has no CLONE; Snowflake clones a DATABASE ROLE")
                );
            }
            return Ok(None);
        }
        let mut modifier: Option<String> = table_modifier.map(str::to_owned);
        if table_modifier.is_none() && !temporary {
            if self.match_text_seq(&["LOCAL"]) || self.match_text_seq(&["GLOBAL"]) {
                let scope = self.previous_text().to_ascii_uppercase();
                if !(self.match_text_seq(&["TEMPORARY"]) || self.match_text_seq(&["TEMP"])) {
                    self.current = start;
                    return Ok(None);
                }
                modifier = Some(format!("{scope} TEMPORARY"));
            } else if self.match_text_seq(&["VOLATILE"]) {
                modifier = Some("VOLATILE".to_owned());
            }
        }
        let (kind, options) = match modifier.as_deref() {
            Some("DYNAMIC") if self.match_text_seq(&["ICEBERG", "TABLE"]) => {
                modifier = Some("DYNAMIC ICEBERG".to_owned());
                ("TABLE", CloneOptions::Dynamic)
            }
            Some("TRANSIENT") if self.match_text_seq(&["DYNAMIC", "TABLE"]) => {
                modifier = Some("TRANSIENT DYNAMIC".to_owned());
                ("TABLE", CloneOptions::Dynamic)
            }
            Some("DYNAMIC") if self.match_token(TokenType::Table) => {
                ("TABLE", CloneOptions::Dynamic)
            }
            Some("ICEBERG" | "LOCAL TEMPORARY" | "GLOBAL TEMPORARY" | "VOLATILE")
                if self.match_token(TokenType::Table) =>
            {
                ("TABLE", CloneOptions::Copy)
            }
            None if !temporary && self.match_text_seq(&["EVENT", "TABLE"]) => {
                ("EVENT TABLE", CloneOptions::Bare)
            }
            None if !temporary && self.match_text_seq(&["ALERT"]) => ("ALERT", CloneOptions::Bare),
            None if !temporary && self.match_text_seq(&["STAGE"]) => ("STAGE", CloneOptions::Bare),
            None if !temporary && self.match_text_seq(&["STREAM"]) => {
                ("STREAM", CloneOptions::Copy)
            }
            None if !temporary && self.match_text_seq(&["SEQUENCE"]) => {
                ("SEQUENCE", CloneOptions::Bare)
            }
            None if !temporary && self.match_text_seq(&["FILE", "FORMAT"]) => {
                ("FILE FORMAT", CloneOptions::Bare)
            }
            None if !temporary && self.names_a_database_role() => {
                self.match_text_seq(&["DATABASE", "ROLE"]);
                ("DATABASE ROLE", CloneOptions::Bare)
            }
            _ => {
                let partial = (modifier.is_none() && self.check_text_seq(&["EVENT"]))
                    || (modifier.as_deref() == Some("DYNAMIC")
                        && self.check_text_seq(&["ICEBERG"]));
                if partial
                    && self
                        .tokens
                        .get(self.current + 2)
                        .is_some_and(|t| t.text.eq_ignore_ascii_case("CLONE"))
                {
                    return Err(self.parse_error("Expected TABLE before the name"));
                }
                self.current = start;
                return Ok(None);
            }
        };
        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        if self.check(TokenType::If) {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        if !self.match_identifier("CLONE") {
            let lone_word = self.is_identifier_token()
                && !self.check_keyword()
                && self
                    .tokens
                    .get(self.current + 1)
                    .is_none_or(|t| t.token_type == TokenType::Semicolon);
            if lone_word && !matches!(kind, "SEQUENCE" | "FILE FORMAT") {
                return Err(self.parse_error(format!(
                    "Expected CLONE or the definition after CREATE {kind} <name>"
                )));
            }
            self.current = start;
            return Ok(None);
        }
        let mut operands = vec![Expression::Table(Box::new(self.parse_object_name()?))];
        if matches!(kind, "TABLE" | "EVENT TABLE") {
            if let Some(time_travel) = self.parse_clone_time_travel()? {
                operands.push(time_travel);
            }
        }
        let properties = self.parse_clone_options(options)?;
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: kind.to_owned(),
            modifier,
            or_replace,
            if_not_exists,
            name,
            source: "CLONE".to_owned(),
            operands,
            properties,
        }))))
    }

    /// Whether DATABASE ROLE starts a role; alone or before CLONE, ROLE names a database.
    fn names_a_database_role(&self) -> bool {
        self.check_text_seq(&["DATABASE", "ROLE"])
            && self.tokens.get(self.current + 2).is_some_and(|t| {
                t.token_type != TokenType::Semicolon && !t.text.eq_ignore_ascii_case("CLONE")
            })
    }

    /// Whether the rest of the statement contains REPLICA or AS OF, which make it a replica.
    fn statement_names_a_replica(&self) -> bool {
        self.tokens[self.current..]
            .iter()
            .take_while(|t| t.token_type != TokenType::Semicolon)
            .collect::<Vec<_>>()
            .windows(2)
            .any(|pair| {
                pair[1].text.eq_ignore_ascii_case("REPLICA")
                    || (pair[0].token_type == TokenType::As
                        && pair[1].text.eq_ignore_ascii_case("OF"))
            })
    }

    /// `CREATE { CONNECTION | FAILOVER GROUP | REPLICATION GROUP } ... AS REPLICA OF o.a.n`, or None.
    fn parse_snowflake_create_replica(&mut self) -> Result<Option<Expression>> {
        let (kind, takes_comment) = if self.match_text_seq(&["CONNECTION"]) {
            ("CONNECTION", true)
        } else if self.match_text_seq(&["FAILOVER", "GROUP"]) {
            ("FAILOVER GROUP", false)
        } else if self.match_text_seq(&["REPLICATION", "GROUP"]) {
            ("REPLICATION GROUP", false)
        } else if self.check_text_seq(&["FAILOVER"])
            || self.check_text_seq(&["REPLICATION"])
            || self.check_text_seq(&["GROUP"])
        {
            return Err(
                self.parse_error("Expected CONNECTION, FAILOVER GROUP or REPLICATION GROUP")
            );
        } else {
            return Ok(None);
        };
        let if_not_exists =
            self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]);
        if self.check(TokenType::If) {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        self.expect_token(TokenType::As)?;
        self.expect_alter_words(&[&["REPLICA", "OF"]])?;
        let primary = self.parse_object_name()?;
        if primary.catalog.is_none()
            || primary
                .schema
                .as_ref()
                .is_none_or(|part| part.name.is_empty())
        {
            return Err(self.parse_error("Expected <organization>.<account>.<name>"));
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if takes_comment && self.match_token(TokenType::Comment) {
            self.expect_token(TokenType::Eq)?;
            properties.push(property("COMMENT", None, Some(self.parse_string_value()?)));
        }
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error(format!(
                "Unexpected token after CREATE {kind} AS REPLICA OF"
            )));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: kind.to_owned(),
            modifier: None,
            or_replace: false,
            if_not_exists,
            name,
            source: "REPLICA".to_owned(),
            operands: vec![Expression::Table(Box::new(primary))],
            properties,
        }))))
    }
}
