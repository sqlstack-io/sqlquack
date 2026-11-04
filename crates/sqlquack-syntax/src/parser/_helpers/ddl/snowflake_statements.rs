//! Snowflake's rules on session, transaction, EXPLAIN and SHOW statements.

use crate::expressions::statements::models::Describe;
use crate::expressions::tree::models::Expression;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Whether the parser reads Snowflake.
    pub(crate) fn is_snowflake(&self) -> bool {
        self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
    }

    /// Refuses Snowflake's `START TRANSACTION NAME` without a name after it.
    pub(crate) fn refuse_snowflake_nameless_transaction(&self) -> Result<()> {
        let named = self.peek_nth(1).is_some_and(|token| {
            matches!(
                token.token_type,
                TokenType::Var | TokenType::Identifier | TokenType::QuotedIdentifier
            )
        });
        match self.is_snowflake() && self.check_identifier("NAME") && !named {
            true => Err(self.parse_error("Expected a transaction name after NAME")),
            false => Ok(()),
        }
    }

    /// Snowflake's `EXPLAIN USING TABULAR | JSON | TEXT <statement>`, after USING.
    pub(crate) fn parse_snowflake_explain_using(
        &mut self,
        leading_comments: Vec<String>,
    ) -> Result<Expression> {
        let format = match self.peek_text().to_ascii_uppercase().as_str() {
            format @ ("TABULAR" | "JSON" | "TEXT") => format.to_owned(),
            _ => return Err(self.parse_error("Expected TABULAR, JSON or TEXT after USING")),
        };
        self.skip();
        let (properties, target) = self.parse_describe_target()?;
        Ok(Expression::Describe(Box::new(Describe {
            target,
            extended: false,
            formatted: false,
            kind: None,
            properties,
            style: Some(format!("USING {format}")),
            partition: None,
            leading_comments,
            as_json: false,
            params: Vec::new(),
        })))
    }

    /// Refuses a Snowflake SHOW ... LIKE pattern that is not a string.
    pub(crate) fn expect_snowflake_like_pattern(&self) -> Result<()> {
        match self.is_snowflake() && !self.check(TokenType::String) {
            true => Err(self.parse_error("Expected a string pattern after LIKE")),
            false => Ok(()),
        }
    }

    /// A quoted secondary role, kept quoted as written.
    pub(crate) fn parse_quoted_role(&mut self) -> Result<String> {
        let role = self.advance_text()?;
        Ok(format!("\"{}\"", role.replace('"', "\"\"")))
    }
}
