//! Parser ddl: Snowflake ALTER of schemas, stages, file formats, routines, services and project objects.

use super::snowflake_alter::alter_object;
use super::snowflake_alter::property;
use crate::expressions::names::models::Literal;
use crate::expressions::schema_objects::models::ObjectProperty;
use crate::expressions::tree::models::Expression;
use crate::expressions::types::models::Tuple;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// The object kinds this module parses, as the words that follow ALTER.
const RESOURCE_KINDS: &[&[&str]] = &[
    &["COMPUTE", "POOL"],
    &["DBT", "PROJECT"],
    &["DCM", "PROJECT"],
    &["EVENT", "ROUTING", "TABLE"],
    &["EXTERNAL", "AGENT"],
    &["EXTERNAL", "TABLE"],
    &["FILE", "FORMAT"],
    &["GIT", "REPOSITORY"],
    &["MODEL", "MONITOR"],
    &["AGENT"],
    &["CONTACT"],
    &["GATEWAY"],
    &["LISTING"],
    &["MODEL"],
    &["NOTEBOOK"],
    &["PROCEDURE"],
    &["SCHEMA"],
    &["SERVICE"],
    &["STAGE"],
];

/// The operands and properties of one ALTER action.
type ActionParts = (Vec<Expression>, Vec<ObjectProperty>);

impl Parser {
    /// Parse a Snowflake ALTER of a kind in RESOURCE_KINDS, or None when the next words name another kind.
    pub(super) fn parse_alter_snowflake_resource(&mut self) -> Result<Option<Expression>> {
        let model_named_monitor = self.check_identifier("MODEL")
            && self.check_next_identifier("MONITOR")
            && self
                .text_at(2)
                .is_some_and(|text| text == "SET" || text == "UNSET");
        let words: &[&str] = if model_named_monitor && self.match_identifier("MODEL") {
            &["MODEL"]
        } else if let Some(words) = RESOURCE_KINDS
            .iter()
            .find(|words| self.match_text_seq(words))
        {
            words
        } else if let Some(kind) = self.partial_resource_kind() {
            return Err(self.parse_error(format!("Snowflake has no ALTER {kind}")));
        } else {
            return Ok(None);
        };
        let kind = words.join(" ");
        let head = self.parse_alter_object_head()?;
        let arguments = if kind == "PROCEDURE" {
            self.parse_routine_signature()?
        } else {
            Vec::new()
        };
        let (action, parts) = self.parse_resource_action(&kind)?;
        let Expression::AlterObject(mut alter) = alter_object(&kind, head, action, parts) else {
            return Err(self.parse_error("Expected an ALTER statement"));
        };
        alter.arguments = arguments;
        Ok(Some(Expression::AlterObject(alter)))
    }

    /// The words of an incomplete multi-word kind, such as `COMPUTE` without `POOL`, if the next words are one.
    fn partial_resource_kind(&self) -> Option<String> {
        let first = self.text_at(0)?;
        let second = self.text_at(1).unwrap_or_default();
        let complete: &[&str] = match first.as_str() {
            "COMPUTE" => &["POOL"],
            "DBT" | "DCM" => &["PROJECT"],
            "GIT" => &["REPOSITORY"],
            "FILE" => &["FORMAT"],
            "EXTERNAL" => &["AGENT", "TABLE", "VOLUME", "CONSUMER", "ACCESS"],
            "EVENT" => &["ROUTING"],
            _ => return None,
        };
        let complete = complete.contains(&second.as_str())
            && (first != "EVENT" || self.text_at(2).as_deref() == Some("TABLE"));
        (!complete).then(|| format!("{first} {second}").trim_end().to_owned())
    }

    /// Parse the action of an ALTER of `kind` and return its keywords with its operands and properties.
    fn parse_resource_action(&mut self, kind: &str) -> Result<(String, ActionParts)> {
        let choices: &[&[&str]] = match kind {
            "AGENT" => &[&["MODIFY", "LIVE", "VERSION", "SET"], &["SET"], &["UNSET"]],
            "COMPUTE POOL" => &[
                &["STOP", "ALL"],
                &["SUSPEND"],
                &["RESUME"],
                &["SET"],
                &["UNSET"],
            ],
            "DBT PROJECT" => &[&["DEPLOY", "FROM"], &["RENAME", "TO"], &["SET"], &["UNSET"]],
            "DCM PROJECT" => &[&["DROP", "DEPLOYMENT"], &["SET"], &["UNSET"]],
            "EVENT ROUTING TABLE" => &[
                &["MODIFY", "RULE"],
                &["RENAME", "TO"],
                &["FORCE", "SET", "RULES"],
                &["FORCE", "SET", "RULE"],
                &["FORCE", "UNSET", "RULE"],
                &["SET", "RULES"],
                &["SET", "RULE"],
                &["UNSET", "RULE"],
            ],
            "EXTERNAL AGENT" => &[&["ADD", "VERSION"], &["SET"], &["UNSET"]],
            "EXTERNAL TABLE" => &[
                &["ADD", "FILES"],
                &["REMOVE", "FILES"],
                &["ADD", "PARTITION"],
                &["DROP", "PARTITION", "LOCATION"],
                &["REFRESH"],
                &["SET"],
                &["UNSET"],
            ],
            "GATEWAY" => &[&["FROM", "SPECIFICATION"]],
            "GIT REPOSITORY" => &[&["FETCH"], &["SET"], &["UNSET"]],
            "LISTING" => &[
                &["PUBLISH"],
                &["UNPUBLISH"],
                &["REVIEW"],
                &["ADD", "TARGETS"],
                &["REMOVE", "TARGETS"],
                &["ADD", "VERSION"],
                &["AS"],
                &["RENAME", "TO"],
                &["SET"],
                &["UNSET"],
            ],
            "MODEL" => &[
                &["ADD", "VERSION"],
                &["DROP", "VERSION"],
                &["MODIFY", "VERSION"],
                &["VERSION"],
                &["RENAME", "TO"],
                &["SET"],
                &["UNSET"],
            ],
            "MODEL MONITOR" => &[
                &["ADD"],
                &["DROP"],
                &["SUSPEND"],
                &["RESUME"],
                &["SET"],
                &["UNSET"],
            ],
            "PROCEDURE" => &[&["EXECUTE", "AS"], &["RENAME", "TO"], &["SET"], &["UNSET"]],
            "SCHEMA" => &[
                &["ENABLE", "MANAGED", "ACCESS"],
                &["DISABLE", "MANAGED", "ACCESS"],
                &["RENAME", "TO"],
                &["SWAP", "WITH"],
                &["SET"],
                &["UNSET"],
            ],
            "SERVICE" => &[
                &["FROM"],
                &["RESTORE", "VOLUME"],
                &["SUSPEND"],
                &["RESUME"],
                &["SET"],
                &["UNSET"],
            ],
            "STAGE" => &[&["REFRESH"], &["RENAME", "TO"], &["SET"], &["UNSET"]],
            _ => &[&["RENAME", "TO"], &["SET"], &["UNSET"]],
        };
        let mut action = self.expect_alter_words(choices)?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        match action.as_str() {
            "RENAME TO" | "SWAP WITH" | "DROP DEPLOYMENT" | "DROP VERSION" => {
                operands.push(self.parse_name_operand()?);
            }
            "SET" if kind == "AGENT" && !self.check_identifier("TAG") => {
                let name = self.parse_simple_name()?;
                self.expect_token(TokenType::Eq)?;
                if !self.is_literal_value() {
                    return Err(self.parse_error("Expected a literal value"));
                }
                let value = self.parse_object_value()?;
                properties.push(property(&name.name.to_ascii_uppercase(), None, Some(value)));
            }
            "SET" | "MODIFY LIVE VERSION SET" => properties = self.parse_resource_set()?,
            "UNSET" => properties = self.parse_resource_unset()?,
            "STOP ALL" => {
                if self.match_text_seq(&["OF", "TYPE"]) {
                    action.push_str(" OF TYPE");
                    operands = self.parse_object_name_list()?;
                }
            }
            "DEPLOY FROM" | "DROP PARTITION LOCATION" | "AS" => {
                operands.push(self.parse_location_value()?);
                if action == "AS" {
                    properties = self.parse_resource_properties()?;
                }
            }
            "FROM SPECIFICATION" => operands.push(self.parse_object_value()?),
            "MODIFY RULE" => {
                operands.push(self.parse_name_operand()?);
                action.push(' ');
                action.push_str(&self.expect_alter_words(&[&["RENAME", "TO"]])?);
                operands.push(self.parse_name_operand()?);
            }
            "SET RULE" | "FORCE SET RULE" => {
                operands.push(self.parse_name_operand()?);
                properties = self.parse_resource_set()?;
            }
            "SET RULES" | "FORCE SET RULES" => loop {
                let rule = self.parse_simple_name()?;
                self.expect_token(TokenType::Eq)?;
                properties.push(property("RULE", Some(rule), Some(self.parse_value_list()?)));
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            },
            "UNSET RULE" | "FORCE UNSET RULE" => operands.push(self.parse_name_operand()?),
            "ADD FILES" | "REMOVE FILES" => operands.push(self.parse_value_list()?),
            "ADD PARTITION" => {
                self.expect_token(TokenType::LParen)?;
                loop {
                    let column = self.parse_simple_name()?;
                    self.expect_token(TokenType::Eq)?;
                    properties.push(property(
                        "PARTITION",
                        Some(column),
                        Some(self.parse_object_value()?),
                    ));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.expect_token(TokenType::RParen)?;
                action.push_str(" LOCATION");
                self.expect_alter_words(&[&["LOCATION"]])?;
                operands.push(self.parse_location_value()?);
            }
            "REFRESH" if kind == "STAGE" => properties = self.parse_resource_properties()?,
            "REFRESH" => {
                if self.check(TokenType::String) {
                    operands.push(self.parse_string_value()?);
                }
            }
            "ADD TARGETS" | "REMOVE TARGETS" => operands.push(self.parse_resource_value()?),
            "ADD VERSION" if kind == "EXTERNAL AGENT" => operands.push(self.parse_name_operand()?),
            "ADD VERSION" if kind == "LISTING" => {
                if self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
                    action.push_str(" IF NOT EXISTS");
                }
                if !self.check(TokenType::From) {
                    operands.push(self.parse_name_operand()?);
                }
                self.expect_token(TokenType::From)?;
                operands.push(self.parse_location_value()?);
                properties = self.parse_resource_properties()?;
            }
            "ADD VERSION" => {
                operands.push(self.parse_name_operand()?);
                self.expect_token(TokenType::From)?;
                if self.match_identifier("MODEL") {
                    action.push_str(" FROM MODEL");
                    operands.push(self.parse_name_operand()?);
                    if self.match_identifier("VERSION") {
                        operands.push(self.parse_name_operand()?);
                    }
                } else {
                    operands.push(self.parse_location_value()?);
                }
            }
            "MODIFY VERSION" => {
                operands.push(self.parse_name_operand()?);
                self.expect_alter_words(&[&["SET"]])?;
                action.push_str(" SET");
                properties = self.parse_resource_set()?;
            }
            "VERSION" => {
                operands.push(self.parse_name_operand()?);
                action.push(' ');
                action
                    .push_str(&self.expect_alter_words(&[&["SET", "ALIAS"], &["UNSET", "ALIAS"]])?);
                if action.starts_with("VERSION SET") {
                    self.expect_token(TokenType::Eq)?;
                    operands.push(self.parse_name_operand()?);
                }
            }
            "ADD" | "DROP" => {
                action.push_str(" SEGMENT_COLUMN");
                self.expect_alter_words(&[&["SEGMENT_COLUMN"]])?;
                self.expect_token(TokenType::Eq)?;
                operands.push(self.parse_object_value()?);
            }
            "EXECUTE AS" => {
                action.push(' ');
                action.push_str(&self.expect_alter_words(&[
                    &["OWNER"],
                    &["CALLER"],
                    &["RESTRICTED", "CALLER"],
                ])?);
            }
            "FROM" => {
                if self.is_stage_path() {
                    operands.push(self.parse_stage_path()?);
                } else if !self.check(TokenType::Identifier) && !self.check(TokenType::Var) {
                    operands.push(self.parse_object_value()?);
                    return Ok((action, (operands, properties)));
                }
                properties = self.parse_service_source()?;
                if properties.is_empty() && operands.is_empty() {
                    return Err(self.parse_error("Expected a specification"));
                }
            }
            "RESTORE VOLUME" => {
                operands.push(self.parse_name_operand()?);
                self.expect_alter_words(&[&["INSTANCES"]])?;
                let mut instances = vec![self.parse_object_value()?];
                while self.match_token(TokenType::Comma) {
                    instances.push(self.parse_object_value()?);
                }
                operands.push(Expression::Tuple(Box::new(Tuple {
                    expressions: instances,
                })));
                self.expect_alter_words(&[&["FROM", "SNAPSHOT"]])?;
                operands.push(self.parse_name_operand()?);
            }
            _ => {}
        }
        Ok((action, (operands, properties)))
    }

    /// Parse an object name operand.
    fn parse_name_operand(&mut self) -> Result<Expression> {
        Ok(Expression::Table(Box::new(self.parse_object_name()?)))
    }

    /// Parse a location: a stage path or a string.
    fn parse_location_value(&mut self) -> Result<Expression> {
        if self.is_stage_path() {
            return self.parse_stage_path();
        }
        self.parse_string_value()
    }

    /// Whether the current token starts a stage path.
    fn is_stage_path(&self) -> bool {
        !self.is_at_end() && self.peek_text().starts_with('@')
    }

    /// Parse a stage path, which like Snowflake's ends at the first whitespace: `@"s 1"` is not one path.
    fn parse_stage_path(&mut self) -> Result<Expression> {
        let mut path = self.peek_text().to_owned();
        let mut end = self.peek().span.end;
        self.skip();
        while !self.is_at_end()
            && self.peek().span.start == end
            && !matches!(
                self.peek().token_type,
                TokenType::Semicolon | TokenType::Comma | TokenType::LParen | TokenType::RParen
            )
        {
            if self.check(TokenType::String) || self.peek_text().chars().any(char::is_whitespace) {
                return Err(self.parse_error("A stage path ends at whitespace"));
            }
            path.push_str(self.peek_text());
            end = self.peek().span.end;
            self.skip();
        }
        Ok(Expression::Literal(Box::new(Literal::String(path))))
    }

    /// Parse a service's source after FROM: SPECIFICATION or SPECIFICATION_TEMPLATE with a value, `<name> = <value>` and USING ( ... ).
    fn parse_service_source(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            if self.check(TokenType::Using) || self.check_identifier("USING") {
                self.skip();
                properties.push(property("USING", None, Some(self.parse_value_list()?)));
                continue;
            }
            let name = self.parse_simple_name()?;
            let upper = name.name.to_ascii_uppercase();
            let bare = matches!(upper.as_str(), "SPECIFICATION" | "SPECIFICATION_TEMPLATE")
                && !self.check(TokenType::Eq);
            if !bare {
                self.expect_token(TokenType::Eq)?;
            }
            properties.push(property(&upper, None, Some(self.parse_object_value()?)));
        }
        Ok(properties)
    }

    /// Whether the current token is a literal: a string, a number, TRUE or FALSE.
    fn is_literal_value(&self) -> bool {
        self.check(TokenType::String)
            || self.check(TokenType::DollarString)
            || self.check(TokenType::Number)
            || self.check(TokenType::True)
            || self.check(TokenType::False)
    }

    /// Parse the properties of a SET: at least one TAG pair, CONTACT pair or `<name> = <value>`.
    fn parse_resource_set(&mut self) -> Result<Vec<ObjectProperty>> {
        let properties = self.parse_resource_properties()?;
        if properties.is_empty() {
            return Err(self.parse_error("Expected a property to set"));
        }
        let pairs = properties
            .iter()
            .filter(|item| item.name.is_some())
            .map(|item| item.kind.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        if !pairs.is_empty()
            && (pairs.len() > 1 || properties.iter().any(|item| item.name.is_none()))
        {
            return Err(self.parse_error("A SET of TAG or CONTACT pairs sets nothing else"));
        }
        Ok(properties)
    }

    /// Parse zero or more TAG pairs, CONTACT pairs and `<name> = <value>` properties, optionally comma-separated.
    fn parse_resource_properties(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let pairs = ["TAG", "CONTACT"]
                .into_iter()
                .find(|word| self.check_identifier(word) && !self.check_next(TokenType::Eq));
            if let Some(word) = pairs {
                self.skip();
                loop {
                    let name = self.parse_simple_qualified_name()?;
                    self.expect_token(TokenType::Eq)?;
                    let value = if word == "CONTACT" {
                        self.parse_name_operand()?
                    } else {
                        self.parse_object_value()?
                    };
                    properties.push(property(word, Some(name), Some(value)));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            } else {
                let name = self.parse_simple_name()?;
                self.expect_token(TokenType::Eq)?;
                let value = self.parse_resource_value()?;
                properties.push(property(&name.name.to_ascii_uppercase(), None, Some(value)));
                self.match_token(TokenType::Comma);
            }
        }
        Ok(properties)
    }

    /// Parse the names an UNSET removes: TAG names, CONTACT purposes or properties, comma-separated.
    fn parse_resource_unset(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        loop {
            let pairs = ["TAG", "CONTACT"].into_iter().find(|word| {
                self.check_identifier(word)
                    && !self.check_next(TokenType::Comma)
                    && !self.check_next(TokenType::Semicolon)
                    && self.current + 1 < self.tokens.len()
            });
            if let Some(word) = pairs {
                self.skip();
                loop {
                    let name = self.parse_simple_qualified_name()?;
                    properties.push(property(word, Some(name), None));
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            } else {
                let name = self.parse_simple_name()?;
                properties.push(property(&name.name.to_ascii_uppercase(), None, None));
            }
            if !self.match_token(TokenType::Comma) {
                return Ok(properties);
            }
        }
    }

    /// Parse a property value: a parenthesized list, a stage path or a single value.
    fn parse_resource_value(&mut self) -> Result<Expression> {
        if self.check(TokenType::LParen) {
            return self.parse_value_list();
        }
        if self.is_stage_path() {
            return self.parse_stage_path();
        }
        self.parse_object_value()
    }

    /// Parse `( [ <item> [ , ] ... ] )`, each item a value or a `<key> = <value>` or `<key> => <value>` pair.
    fn parse_value_list(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut items: Vec<Expression> = Vec::new();
        while !self.match_token(TokenType::RParen) {
            let item = self.parse_resource_value()?;
            items.push(
                if self.match_token(TokenType::Eq) || self.match_token(TokenType::FArrow) {
                    Expression::Tuple(Box::new(Tuple {
                        expressions: vec![item, self.parse_resource_value()?],
                    }))
                } else {
                    item
                },
            );
            self.match_token(TokenType::Comma);
            if self.is_at_end() {
                return Err(self.parse_error("Expected )"));
            }
        }
        Ok(Expression::Tuple(Box::new(Tuple { expressions: items })))
    }
}
