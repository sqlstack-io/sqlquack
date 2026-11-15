//! Parser ddl: Snowflake CREATE OPENFLOW CONNECTOR, DEPLOYMENT and RUNTIME, checked against their grammar.

use crate::parser::_helpers::ddl::snowflake_alter::property;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

/// What a CREATE OPENFLOW property's value is.
#[derive(Clone, Copy)]
enum OpenflowValue {
    /// A string, number, boolean or name, kept as written.
    Scalar,
    /// One of a fixed set of words, such as NODE_TYPE's SMALL, kept as a keyword.
    Keyword,
    /// `( [ <name or string> [ , ... ] ] )`, a comma needing an item after it.
    List,
}

/// The properties each CREATE OPENFLOW kind takes, after its head, with the kind of their values.
const CONNECTOR_PROPERTIES: &[(&str, OpenflowValue)] = &[
    ("DISPLAY_NAME", OpenflowValue::Scalar),
    ("COMMENT", OpenflowValue::Scalar),
];
const DEPLOYMENT_PROPERTIES: &[(&str, OpenflowValue)] = &[
    ("DEPLOYMENT_TYPE", OpenflowValue::Keyword),
    ("VPC_TYPE", OpenflowValue::Scalar),
    ("USE_PRIVATE_LINK", OpenflowValue::Scalar),
    ("USE_USER_AUTH_OVER_PRIVATE_LINK", OpenflowValue::Scalar),
    ("CUSTOM_INGRESS_HOSTNAME", OpenflowValue::Scalar),
    ("DISPLAY_NAME", OpenflowValue::Scalar),
    ("COMMENT", OpenflowValue::Scalar),
    ("EVENT_TABLE", OpenflowValue::Scalar),
];
const RUNTIME_PROPERTIES: &[(&str, OpenflowValue)] = &[
    ("NODE_TYPE", OpenflowValue::Keyword),
    ("NODE_TYPE_TIER", OpenflowValue::Scalar),
    ("MIN_NODES", OpenflowValue::Scalar),
    ("MAX_NODES", OpenflowValue::Scalar),
    ("EXECUTE_AS_ROLE", OpenflowValue::Scalar),
    ("EXTERNAL_ACCESS_INTEGRATIONS", OpenflowValue::List),
    ("DISPLAY_NAME", OpenflowValue::Scalar),
    ("COMMENT", OpenflowValue::Scalar),
];

impl Parser {
    /// A Snowflake CREATE OPENFLOW CONNECTOR, DEPLOYMENT or RUNTIME; None for any other CREATE and outside the faithful Snowflake modes.
    pub(crate) fn parse_snowflake_create_openflow(
        &mut self,
        or_replace: bool,
        or_alter: bool,
    ) -> Result<Option<Expression>> {
        if !self.applies_snowflake_context_rules() || !self.check_identifier("OPENFLOW") {
            return Ok(None);
        }
        let Some((kind, keys)) = [
            ("CONNECTOR", CONNECTOR_PROPERTIES),
            ("DEPLOYMENT", DEPLOYMENT_PROPERTIES),
            ("RUNTIME", RUNTIME_PROPERTIES),
        ]
        .into_iter()
        .find(|(kind, _)| self.check_text_seq(&["OPENFLOW", kind])) else {
            return Ok(None);
        };
        self.skip();
        self.skip();
        let if_not_exists = self.check(TokenType::If);
        if if_not_exists
            && !self.match_keywords(&[TokenType::If, TokenType::Not, TokenType::Exists])
        {
            return Err(self.parse_error("Expected IF NOT EXISTS"));
        }
        let name = self.parse_object_name()?;
        let mut operands: Vec<Expression> = Vec::new();
        let mut properties: Vec<ObjectProperty> = Vec::new();
        let mut source = String::new();
        match kind {
            "CONNECTOR" => {
                self.expect_openflow_words(&["IN", "RUNTIME"])?;
                let runtime = Expression::Table(Box::new(self.parse_object_name()?));
                properties.push(property("IN RUNTIME", None, Some(runtime)));
                self.expect_token(TokenType::From)?;
                if self.match_text_seq(&["DEFINITION"]) {
                    source = "FROM DEFINITION".to_owned();
                    operands.push(Expression::Table(Box::new(self.parse_object_name()?)));
                } else {
                    source = "FROM".to_owned();
                    operands.push(self.parse_string_value()?);
                }
            }
            "RUNTIME" => {
                self.expect_openflow_words(&["IN", "DEPLOYMENT"])?;
                let deployment = Expression::Table(Box::new(self.parse_object_name()?));
                properties.push(property("IN DEPLOYMENT", None, Some(deployment)));
            }
            _ => {}
        }
        while !self.is_statement_end_at(0) {
            let key = self.peek_text().to_ascii_uppercase();
            let Some(&(_, value)) = keys.iter().find(|(name, _)| *name == key) else {
                return Err(self.parse_error(format!("Expected a property of an Openflow {kind}")));
            };
            if self.peek().token_type == TokenType::QuotedIdentifier {
                return Err(self.parse_error("Expected a property name"));
            }
            self.skip();
            self.expect_token(TokenType::Eq)?;
            let value = match value {
                OpenflowValue::Scalar => self.parse_object_value()?,
                OpenflowValue::Keyword => self.parse_openflow_keyword()?,
                OpenflowValue::List => self.parse_openflow_list()?,
            };
            properties.push(property(&key, None, Some(value)));
        }
        Ok(Some(Expression::CreateObject(Box::new(CreateObject {
            kind: format!("OPENFLOW {kind}"),
            modifier: None,
            or_replace,
            or_alter,
            if_not_exists,
            name,
            source,
            operands,
            properties,
            ..Default::default()
        }))))
    }

    /// Consume `words`, or refuse the statement.
    fn expect_openflow_words(&mut self, words: &[&str]) -> Result<()> {
        match self.match_text_seq(words) {
            true => Ok(()),
            false => Err(self.parse_error(format!("Expected {}", words.join(" ")))),
        }
    }

    /// An unquoted word naming one of a property's fixed choices, kept as an upper-case keyword.
    fn parse_openflow_keyword(&mut self) -> Result<Expression> {
        if !matches!(
            self.peek().token_type,
            TokenType::Identifier | TokenType::Var
        ) || self.is_statement_end_at(0)
        {
            return Err(self.parse_error("Expected a word"));
        }
        let word = self.advance_text()?.to_ascii_uppercase();
        Ok(Expression::Var(Box::new(Var { this: word })))
    }

    /// `( [ <name or string> [ , ... ] ] )`, a comma needing an item after it.
    fn parse_openflow_list(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let mut expressions: Vec<Expression> = Vec::new();
        if !self.check(TokenType::RParen) {
            loop {
                expressions.push(self.parse_object_value()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Tuple(Box::new(Tuple { expressions })))
    }
}
