//! Parser ddl: Snowflake CREATE STREAM on a table, view, external, dynamic or event table or a stage, checked against its grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// A source a stream can read: its words after ON, whether it takes AT or BEFORE, and the properties it takes after.
struct StreamSource {
    words: &'static [&'static str],
    time_travel: bool,
    properties: &'static [&'static [&'static str]],
}

/// The sources of CREATE STREAM, each with the properties its grammar documents.
const STREAM_SOURCES: [StreamSource; 6] = [
    StreamSource {
        words: &["TABLE"],
        time_travel: true,
        properties: &[
            &["APPEND_ONLY"],
            &["SHOW_INITIAL_ROWS"],
            &["COMMENT"],
            &["COPY"],
        ],
    },
    StreamSource {
        words: &["VIEW"],
        time_travel: true,
        properties: &[
            &["APPEND_ONLY"],
            &["SHOW_INITIAL_ROWS"],
            &["COMMENT"],
            &["COPY"],
        ],
    },
    StreamSource {
        words: &["EXTERNAL", "TABLE"],
        time_travel: true,
        properties: &[&["INSERT_ONLY"], &["COMMENT"], &["COPY"]],
    },
    StreamSource {
        words: &["DYNAMIC", "TABLE"],
        time_travel: false,
        properties: &[&["COMMENT"], &["COPY"]],
    },
    StreamSource {
        words: &["EVENT", "TABLE"],
        time_travel: false,
        properties: &[&["COMMENT"], &["COPY"]],
    },
    StreamSource {
        words: &["STAGE"],
        time_travel: false,
        properties: &[&["COMMENT"], &["COPY"]],
    },
];

/// CREATE OR ALTER STREAM and DEFINE STREAM: on a table only, with these properties.
const STREAM_OR_ALTER_SOURCE: StreamSource = StreamSource {
    words: &["TABLE"],
    time_travel: false,
    properties: &[&["APPEND_ONLY"], &["SHOW_INITIAL_ROWS"], &["COMMENT"]],
};

/// The kinds a stream's AT or BEFORE takes.
const STREAM_TIME_TRAVEL_KINDS: [&[&str]; 4] =
    [&["TIMESTAMP"], &["OFFSET"], &["STATEMENT"], &["STREAM"]];

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
        if !or_alter {
            if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
                properties.extend(self.parse_external_tags()?);
            }
            if self.match_text_seq(&["COPY", "GRANTS"]) {
                properties.push(property("COPY GRANTS", None, None));
            }
        }
        if !self.match_token(TokenType::On) {
            return Err(self.parse_error("Expected ON and the stream's source"));
        }
        let sources = match or_alter {
            true => std::slice::from_ref(&STREAM_OR_ALTER_SOURCE),
            false => &STREAM_SOURCES[..],
        };
        let Some(source) = sources
            .iter()
            .find(|source| self.check_text_seq(source.words))
        else {
            let words = sources
                .iter()
                .map(|source| source.words.join(" "))
                .collect::<Vec<_>>()
                .join(" or ");
            return Err(self.parse_error(format!("Expected {words}")));
        };
        self.match_text_seq(source.words);
        let mut operands = vec![Expression::Table(Box::new(
            self.parse_iceberg_object_name()?,
        ))];
        if source.time_travel {
            operands.extend(self.parse_time_travel(&STREAM_TIME_TRAVEL_KINDS)?);
        }
        while !self.is_at_end() && !self.check(TokenType::Semicolon) {
            let kind = self.expect_alter_words(source.properties)?;
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
            source: format!("ON {}", source.words.join(" ")),
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
