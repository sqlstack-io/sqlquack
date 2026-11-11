//! Parser ddl: Snowflake CREATE EVENT TABLE, CREATE EVENT ROUTING TABLE and CREATE ONLINE FEATURE TABLE, checked against their grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::constants::SNOWFLAKE_DYNAMIC_TABLE_REQUIRED;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::BinaryOp;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The fields of an event routing rule, in the order they are written.
const ROUTING_RULE_FIELDS: [&str; 3] = ["REGION_GROUP", "REGIONS", "DESTINATION_ACCOUNT"];

impl Parser {
    /// A Snowflake CREATE [OR REPLACE] EVENT TABLE, EVENT ROUTING TABLE or ONLINE FEATURE TABLE other than a clone; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_event_table(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if or_alter || !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        let start = self.current;
        if self.names_a_partial_event_kind() {
            return Err(self.parse_error("Expected a complete object kind after CREATE"));
        }
        let kind = if self.match_text_seq(&["EVENT", "TABLE"]) {
            "EVENT TABLE"
        } else if self.match_text_seq(&["EVENT", "ROUTING", "TABLE"]) {
            "EVENT ROUTING TABLE"
        } else if self.match_text_seq(&["ONLINE", "FEATURE", "TABLE"]) {
            "ONLINE FEATURE TABLE"
        } else {
            return Ok(None);
        };
        let if_not_exists = kind == "EVENT TABLE" && self.check(TokenType::If);
        if if_not_exists {
            if !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists]) {
                return Err(self.parse_error("Expected IF NOT EXISTS"));
            }
            if or_replace {
                return Err(self.parse_error("OR REPLACE and IF NOT EXISTS cannot be combined"));
            }
        }
        let name = self.parse_iceberg_object_name()?;
        if kind == "EVENT TABLE" && self.check_identifier("CLONE") {
            self.current = start;
            return Ok(None);
        }
        let (source, operands, properties) = match kind {
            "EVENT TABLE" => (
                String::new(),
                Vec::new(),
                self.parse_event_table_properties()?,
            ),
            "EVENT ROUTING TABLE" => (
                "WITH RULES".to_owned(),
                Vec::new(),
                self.parse_event_routing_rules()?,
            ),
            _ => {
                let (key, properties, source) = self.parse_online_feature_table()?;
                ("FROM".to_owned(), vec![key, source], properties)
            }
        };
        if !self.is_at_end() && !self.check(TokenType::Semicolon) {
            return Err(self.parse_error("Expected the end of the statement"));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: kind.to_owned(),
            modifier: None,
            or_replace,
            or_alter: false,
            if_not_exists,
            name,
            source,
            operands,
            properties,
        }))))
    }

    /// Whether the words at the cursor are part of EVENT TABLE, EVENT ROUTING TABLE or ONLINE FEATURE TABLE but not all of it: a run of those words ending in TABLE, or EVENT before a name and the end.
    fn names_a_partial_event_kind(&self) -> bool {
        let word = |offset: usize| {
            self.tokens
                .get(self.current + offset)
                .map(|token| token.text.to_ascii_uppercase())
                .unwrap_or_default()
        };
        let mut words: Vec<String> = Vec::new();
        while ["EVENT", "ROUTING", "ONLINE", "FEATURE"].contains(&word(words.len()).as_str()) {
            words.push(word(words.len()));
        }
        let kinds: [&[&str]; 3] = [&["EVENT"], &["EVENT", "ROUTING"], &["ONLINE", "FEATURE"]];
        if !words.is_empty() && word(words.len()) == "TABLE" {
            return !kinds.contains(
                &words
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
        }
        let ends = |offset: usize| {
            self.tokens
                .get(self.current + offset)
                .is_none_or(|token| token.token_type == TokenType::Semicolon)
        };
        words == ["EVENT"] && ends(2)
    }

    /// The properties of an event table: those a table takes after its columns.
    fn parse_event_table_properties(&mut self) -> Result<Vec<ObjectProperty>> {
        let mut properties: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            properties.extend(self.parse_checked_table_property()?);
        }
        Ok(properties)
    }

    /// `WITH RULES <rule> = ( REGION_GROUP = <group> , REGIONS = ( <region> , ... ) , DESTINATION_ACCOUNT = <org>.<account> ) [ ... ]`, one RULE property per rule.
    fn parse_event_routing_rules(&mut self) -> Result<Vec<ObjectProperty>> {
        if !self.match_text_seq(&["WITH", "RULES"]) {
            return Err(self.parse_error("Expected WITH RULES"));
        }
        let mut rules: Vec<ObjectProperty> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let rule = self.parse_external_name()?;
            self.expect_token(TokenType::Eq)?;
            self.expect_token(TokenType::LParen)?;
            let mut fields: Vec<Expression> = Vec::new();
            for (position, field) in ROUTING_RULE_FIELDS.iter().enumerate() {
                if position > 0 {
                    self.expect_token(TokenType::Comma)?;
                }
                if !self.match_text_seq(&[field]) {
                    return Err(self.parse_error(format!("Expected {field}")));
                }
                self.expect_token(TokenType::Eq)?;
                let value = match *field {
                    "REGIONS" => self.parse_routing_regions()?,
                    "DESTINATION_ACCOUNT" => self.parse_routing_account()?,
                    _ => Expression::Identifier(self.parse_external_name()?),
                };
                fields.push(Expression::PropertyEQ(Box::new(BinaryOp::new(
                    Expression::Identifier(Identifier::new(*field)),
                    value,
                ))));
            }
            self.expect_token(TokenType::RParen)?;
            let value = Expression::Tuple(Box::new(Tuple {
                expressions: fields,
            }));
            rules.push(property("RULE", Some(rule), Some(value)));
        }
        if rules.is_empty() {
            return Err(self.parse_error("Expected a rule after WITH RULES"));
        }
        Ok(rules)
    }

    /// `( <region> [ , ... ] )`, each a string or a name; the list may be empty.
    fn parse_routing_regions(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions: Vec<Expression> = Vec::new();
        while !self.check(TokenType::RParen) {
            expressions.push(match self.check(TokenType::String) {
                true => self.parse_primary()?,
                false => Expression::Identifier(self.parse_external_name()?),
            });
            if !self.match_token(TokenType::Comma) {
                break;
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }

    /// `<organization> . <account>`, both names.
    fn parse_routing_account(&mut self) -> Result<Expression> {
        let organization = self.parse_external_name()?;
        self.expect_token(TokenType::Dot)?;
        let account = self.parse_external_name()?;
        Ok(Expression::Tuple(Box::new(Tuple {
            expressions: vec![
                Expression::Identifier(organization),
                Expression::Identifier(account),
            ],
        })))
    }

    /// `PRIMARY KEY ( <columns> ) <property> ... FROM <source>`: the key, the properties with TARGET_LAG and WAREHOUSE required, and the source.
    fn parse_online_feature_table(
        &mut self,
    ) -> Result<(Expression, Vec<ObjectProperty>, Expression)> {
        if !self.match_keywords(&[TokenType::PrimaryKey, TokenType::Key])
            && !self.match_text_seq(&["PRIMARY", "KEY"])
        {
            return Err(self.parse_error("Expected PRIMARY KEY"));
        }
        let key = self.parse_iceberg_name_list(false)?;
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        while !self.is_at_end() && !self.check(TokenType::Semicolon) && !self.check(TokenType::From)
        {
            let (found_key, found) = self.parse_interactive_property()?;
            keys.extend(found_key);
            properties.extend(found);
        }
        let missing = SNOWFLAKE_DYNAMIC_TABLE_REQUIRED
            .iter()
            .any(|required| !keys.contains(&required.to_string()));
        if missing {
            return Err(self.parse_error("Expected TARGET_LAG and WAREHOUSE"));
        }
        self.expect_token(TokenType::From)?;
        let source = Expression::Table(Box::new(self.parse_iceberg_object_name()?));
        Ok((key, properties, source))
    }
}
