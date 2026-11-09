//! Snowflake's rules on session, transaction, EXPLAIN and SHOW statements.

use crate::parser::constants::{SHARED_DESCRIBE_KINDS, SNOWFLAKE_DESCRIBE_KINDS};
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::functions::models::Operator;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::statements::models::Describe;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Whether the parser reads Snowflake.
    pub(crate) fn is_snowflake(&self) -> bool {
        self.config.dialect == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
    }

    /// Chains Snowflake statements with the flow operator, as in `SELECT 1 ->> SELECT * FROM $1`.
    pub(crate) fn parse_snowflake_flow(&mut self, first: Expression) -> Result<Expression> {
        let mut flow = first;
        while self.applies_snowflake_context_rules() && self.match_token(TokenType::DArrow) {
            let next = self.parse_statement()?;
            flow = Expression::Operator(Box::new(Operator {
                this: Box::new(flow),
                operator: Some(Box::new(Expression::Identifier(Identifier::new("->>")))),
                expression: Box::new(next),
                comments: Vec::new(),
            }));
        }
        Ok(flow)
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

    /// Snowflake's DESCRIBE of a kind the shared one does not read; None for TABLE, VIEW and its other kinds.
    pub(crate) fn parse_snowflake_describe(
        &mut self,
        leading_comments: &[String],
    ) -> Result<Option<Expression>> {
        if self.applies_snowflake_context_rules() && self.is_statement_end_at(1) {
            return Ok(Some(Expression::Describe(Box::new(Describe {
                target: Expression::Table(Box::new(self.parse_object_name()?)),
                extended: false,
                formatted: false,
                kind: None,
                properties: Vec::new(),
                style: None,
                partition: None,
                leading_comments: leading_comments.to_vec(),
                as_json: false,
                params: Vec::new(),
            }))));
        }
        let as_resource = self.match_text_seq(&["AS", "RESOURCE"]);
        let Some(kind) = SNOWFLAKE_DESCRIBE_KINDS
            .iter()
            .find(|kind| self.match_text_seq(kind))
            .map(|kind| {
                let shorter = &kind[..kind.len() - 1];
                let kind_word_named = kind.len() > 1
                    && self.is_statement_end_at(0)
                    && SNOWFLAKE_DESCRIBE_KINDS.contains(&shorter);
                if kind_word_named {
                    self.current -= 1;
                    return shorter.join(" ");
                }
                kind.join(" ")
            })
        else {
            let shared = SHARED_DESCRIBE_KINDS
                .iter()
                .any(|kind| self.peek_text().eq_ignore_ascii_case(kind));
            return match shared && !as_resource {
                true => Ok(None),
                false => Err(self.parse_error("Expected an object kind after DESCRIBE")),
            };
        };
        if as_resource && !matches!(kind.as_str(), "AGENT") {
            return Err(self.parse_error("Expected AGENT after DESCRIBE AS RESOURCE"));
        }
        let target = match kind.as_str() {
            "TRANSACTION" if self.check(TokenType::Number) => self.parse_primary()?,
            "RESULT"
                if self.check(TokenType::String)
                    || (self.check_identifier("LAST_QUERY_ID")
                        && self.check_next(TokenType::LParen)) =>
            {
                self.parse_primary()?
            }
            "TRANSACTION" => return Err(self.parse_error("Expected a transaction ID")),
            "RESULT" if self.check_next(TokenType::LParen) => {
                return Err(self.parse_error("Expected a query ID or LAST_QUERY_ID()"))
            }
            _ => Expression::Table(Box::new(self.parse_object_name()?)),
        };
        let properties = match kind.as_str() {
            "EXTERNAL TABLE" | "ICEBERG TABLE" => {
                self.parse_describe_choice("TYPE", &["COLUMNS", "STAGE"])?
            }
            "LISTING" => self.parse_describe_choice("REVISION", &["DRAFT", "PUBLISHED"])?,
            "CONFIGURATION" | "SPECIFICATION" if self.match_text_seq(&["IN", "APPLICATION"]) => {
                let start = self.current;
                self.parse_object_name()?;
                vec![("in_application".to_owned(), self.written_since(start))]
            }
            _ => Vec::new(),
        };
        Ok(Some(Expression::Describe(Box::new(Describe {
            target,
            extended: false,
            formatted: false,
            kind: Some(kind),
            properties,
            style: as_resource.then(|| "AS RESOURCE".to_owned()),
            partition: None,
            leading_comments: leading_comments.to_vec(),
            as_json: false,
            params: Vec::new(),
        }))))
    }

    /// An optional `<name> = <choice>` after a DESCRIBE target, as the shared DESCRIBE keeps one.
    fn parse_describe_choice(
        &mut self,
        name: &str,
        choices: &[&str],
    ) -> Result<Vec<(String, String)>> {
        if !self.match_identifier(name) {
            return Ok(Vec::new());
        }
        self.expect_token(TokenType::Eq)?;
        match choices
            .iter()
            .any(|choice| self.peek_text().eq_ignore_ascii_case(choice))
        {
            true => Ok(vec![(name.to_ascii_lowercase(), self.advance_text()?)]),
            false => {
                Err(self.parse_error(format!("Expected {} after {name} =", choices.join(" or "))))
            }
        }
    }

    /// The tokens read since `start`, joined as written, quoted identifiers keeping their quotes.
    fn written_since(&self, start: usize) -> String {
        self.tokens[start..self.current]
            .iter()
            .map(|token| match token.token_type {
                TokenType::QuotedIdentifier => format!("\"{}\"", token.text.replace('"', "\"\"")),
                _ => token.text.to_string(),
            })
            .collect()
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
