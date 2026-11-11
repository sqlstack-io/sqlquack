//! Parser ddl: Snowflake CREATE STREAM on a table, view, external, dynamic or event table or a stage, checked against its grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::_helpers::ddl::snowflake_alter::property_kind;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The sources of CREATE STREAM, as their words after ON.
const STREAM_SOURCES: [&[&str]; 6] = [
    &["TABLE"],
    &["VIEW"],
    &["EXTERNAL", "TABLE"],
    &["DYNAMIC", "TABLE"],
    &["EVENT", "TABLE"],
    &["STAGE"],
];

/// The kinds a stream's AT or BEFORE takes.
const STREAM_TIME_TRAVEL_KINDS: [&[&str]; 5] = [
    &["TIMESTAMP"],
    &["OFFSET"],
    &["STATEMENT"],
    &["STREAM"],
    &["VERSION"],
];

impl Parser {
    /// A Snowflake CREATE [ OR REPLACE | OR ALTER ] STREAM or DEFINE STREAM other than a clone; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_stream(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() || !self.check_identifier("STREAM") {
            return Ok(None);
        }
        let start = self.current;
        self.skip();
        let if_not_exists = !or_alter && self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_iceberg_object_name()?;
        if !or_alter && self.check_identifier("CLONE") {
            self.current = start;
            return Ok(None);
        }
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
            properties.extend(self.parse_external_tags()?);
        }
        if !or_alter && self.match_text_seq(&["COPY", "GRANTS"]) {
            properties.push(property("COPY GRANTS", None, None));
        }
        if !self.match_token(TokenType::On) {
            return Err(self.parse_error("Expected ON and the stream's source"));
        }
        let sources = match or_alter {
            true => &STREAM_SOURCES[..1],
            false => &STREAM_SOURCES[..],
        };
        let Some(source) = sources.iter().find(|words| self.check_text_seq(words)) else {
            let words = sources
                .iter()
                .map(|words| words.join(" "))
                .collect::<Vec<_>>()
                .join(" or ");
            return Err(self.parse_error(format!("Expected {words}")));
        };
        self.match_text_seq(source);
        let mut operands = vec![Expression::Table(Box::new(
            self.parse_iceberg_object_name()?,
        ))];
        if !or_alter {
            operands.extend(self.parse_time_travel(&STREAM_TIME_TRAVEL_KINDS)?);
        }
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let kind = property_kind(&self.parse_simple_name()?);
            self.expect_token(TokenType::Eq)?;
            let value = match kind.as_str() {
                "COPY" => self.parse_stream_copy()?,
                _ => self.parse_external_scalar()?,
            };
            properties.push(property(&kind, None, Some(value)));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: "STREAM".to_owned(),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: format!("ON {}", source.join(" ")),
            operands,
            properties,
        }))))
    }

    /// A stream's `COPY = ( [ GRANTS ] )` value after its `=`.
    fn parse_stream_copy(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions: Vec<Expression> = Vec::new();
        if !self.check(TokenType::RParen) {
            let grants = self.expect_alter_words(&[&["GRANTS"]])?;
            expressions.push(Expression::Identifier(Identifier::new(grants)));
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }
}
