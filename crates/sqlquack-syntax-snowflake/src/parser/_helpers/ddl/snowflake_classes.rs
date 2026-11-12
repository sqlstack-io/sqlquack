//! Parser ddl: CREATE, ALTER and DROP of a Snowflake class instance, such as `SNOWFLAKE.ML.FORECAST`, checked against its grammar.

use crate::parser::_helpers::ddl::snowflake_alter::{alter_object, property};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// The classes whose instances CREATE, ALTER and DROP name, as the last part of the class.
const SNOWFLAKE_CLASSES: [&str; 7] = [
    "ANOMALY_DETECTION",
    "BUDGET",
    "CLASSIFICATION",
    "CLASSIFICATION_PROFILE",
    "CUSTOM_CLASSIFIER",
    "FORECAST",
    "TOP_INSIGHTS",
];

impl Parser {
    /// The class after CREATE, ALTER or DROP as `SNOWFLAKE.<schema>.<class>`, where an empty schema stays empty; None when the next words name no class, and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_class_kind(&mut self) -> Result<Option<String>> {
        if !self.applies_snowflake_context_rules() {
            return Ok(None);
        }
        if self.check(TokenType::Dot) {
            return Err(self.parse_error("Expected SNOWFLAKE before a class's schema"));
        }
        if !self.check_identifier("SNOWFLAKE") || !self.check_next(TokenType::Dot) {
            return Ok(None);
        }
        self.current += 2;
        let schema = match self.check(TokenType::Dot) {
            true => String::new(),
            false => self.parse_simple_name()?.name.to_ascii_uppercase(),
        };
        self.expect_token(TokenType::Dot)?;
        let class = self.peek_text().to_ascii_uppercase();
        if self.check(TokenType::QuotedIdentifier) || !SNOWFLAKE_CLASSES.contains(&class.as_str()) {
            return Err(self.parse_error("Expected a Snowflake class"));
        }
        self.skip();
        Ok(Some(format!("SNOWFLAKE.{schema}.{class}")))
    }

    /// The kind a Snowflake DROP names before the other kinds are tried: a class, or DATABASE ROLE; None otherwise.
    pub(crate) fn parse_snowflake_drop_kind(&mut self, snowflake: bool) -> Result<Option<String>> {
        if let Some(class) = self.parse_snowflake_class_kind()? {
            return Ok(Some(class));
        }
        let database_role = snowflake && self.match_text_seq(&["DATABASE", "ROLE"]);
        Ok(database_role.then(|| "DATABASE ROLE".to_owned()))
    }

    /// A CREATE [ OR REPLACE ] of a class instance: `<class> [ IF NOT EXISTS ] <name> ( [ <argument> , ... ] ) [ [ WITH ] TAG ( ... ) ] [ [ WITH ] COMMENT = <value> ]`; None for any other CREATE.
    pub(crate) fn parse_snowflake_create_class(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        let Some(kind) = self.parse_snowflake_class_kind()? else {
            return Ok(None);
        };
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        self.expect_token(TokenType::LParen)?;
        let operands = match self.match_token(TokenType::RParen) {
            true => Vec::new(),
            false => {
                let arguments = self.parse_function_arguments()?;
                self.expect_token(TokenType::RParen)?;
                arguments
            }
        };
        let mut properties: Vec<ObjectProperty> = Vec::new();
        if self.match_text_seq(&["WITH", "TAG"]) || self.match_text_seq(&["TAG"]) {
            properties.extend(self.parse_external_tags()?);
        }
        if self.match_text_seq(&["WITH", "COMMENT"]) || self.match_text_seq(&["COMMENT"]) {
            self.expect_token(TokenType::Eq)?;
            properties.push(property("COMMENT", None, Some(self.parse_object_value()?)));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind,
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source: String::new(),
            operands,
            properties,
        }))))
    }

    /// An ALTER of a class instance after its class: `[ IF EXISTS ] <name>`, then RENAME TO, or SET or UNSET of tags and properties.
    pub(super) fn parse_alter_snowflake_class(&mut self, kind: &str) -> Result<Expression> {
        let head = self.parse_alter_object_head()?;
        let action = self.expect_alter_words(&[&["RENAME", "TO"], &["SET"], &["UNSET"]])?;
        let (operands, properties) = match action.as_str() {
            "RENAME TO" => (
                vec![Expression::Table(Box::new(self.parse_object_name()?))],
                Vec::new(),
            ),
            "SET" => (Vec::new(), self.parse_alter_object_properties(true, false)?),
            _ => (
                Vec::new(),
                self.parse_alter_object_properties(false, false)?,
            ),
        };
        Ok(alter_object(kind, head, action, (operands, properties)))
    }
}
