//! Snowflake's rules on COMMENT ON names and values, and its class instance method calls.

use crate::expressions::names::models::Identifier;
use crate::expressions::operators::models::Call;
use crate::expressions::statements::models::ExecuteObject;
use crate::expressions::statements::models::ExecuteUsingPolicyContext;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::EXECUTE_DCM_PROJECT;
use crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// In Snowflake, refuse an unquoted reserved keyword in any dotted part of a COMMENT ON name.
    pub(super) fn check_snowflake_comment_name(&self) -> Result<()> {
        if !self.is_snowflake() {
            return Ok(());
        }
        let mut offset = 0;
        while let Some(token) = self.tokens.get(self.current + offset) {
            if token.token_type != TokenType::QuotedIdentifier
                && SNOWFLAKE_RESERVED_KEYWORDS
                    .iter()
                    .any(|keyword| token.text.eq_ignore_ascii_case(keyword))
            {
                return Err(self.parse_error("Expected an object name"));
            }
            match self.tokens.get(self.current + offset + 1) {
                Some(next) if next.token_type == TokenType::Dot => offset += 2,
                _ => return Ok(()),
            }
        }
        Ok(())
    }

    /// A called object's dotted name; an empty middle part (`<database>..<name>`) is kept as an empty part.
    pub(crate) fn parse_call_name(&mut self) -> Result<Vec<Identifier>> {
        let mut name = vec![self.expect_identifier_or_keyword_with_quoted()?];
        while self.match_token(TokenType::Dot) {
            if name.len() == 1 && self.match_token(TokenType::Dot) {
                name.push(Identifier::new(""));
            }
            name.push(self.expect_identifier_or_keyword_with_quoted()?);
        }
        for part in &name {
            self.reject_reserved_name_part(part)?;
        }
        Ok(name)
    }

    /// Parse a Snowflake class instance method call, `<instance>!<method>( [ <arg> , ... ] )`, after CALL; None for any other CALL.
    pub(super) fn parse_snowflake_method_call(&mut self) -> Result<Option<Expression>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        if self.check(TokenType::Exclamation) {
            return Err(self.parse_error("Expected an instance name before !"));
        }
        let token_type = |offset: usize| {
            self.tokens
                .get(self.current + offset)
                .map(|token| token.token_type)
        };
        let mut offset = 1;
        while token_type(offset) == Some(TokenType::Dot) {
            offset += if token_type(offset + 1) == Some(TokenType::Dot) {
                3
            } else {
                2
            };
        }
        if token_type(offset) != Some(TokenType::Exclamation) {
            return Ok(None);
        }
        let name = self.parse_call_name()?;
        self.expect_token(TokenType::Exclamation)?;
        if !self.is_identifier_token() {
            return Err(self.parse_error("Expected a method name after !"));
        }
        let member = self.expect_identifier_or_keyword_with_quoted()?;
        self.expect_token(TokenType::LParen)?;
        let arguments = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        self.expect_token(TokenType::RParen)?;
        Ok(Some(Expression::Call(Box::new(Call {
            source_dialect: None,
            name,
            member: Some(member),
            arguments,
            asynchronous: false,
            hints: Vec::new(),
            with: None,
            into: None,
        }))))
    }

    /// The rest of Snowflake `EXECUTE USING POLICY_CONTEXT ( <name> => <value> , ... ) AS <query>`.
    pub(super) fn parse_execute_using_policy_context(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;
        let arguments = self.parse_function_arguments()?;
        if arguments
            .iter()
            .any(|argument| !matches!(argument, Expression::NamedArgument(_)))
        {
            return Err(self.parse_error("POLICY_CONTEXT takes only named arguments"));
        }
        self.expect_token(TokenType::RParen)?;
        self.expect_token(TokenType::As)?;
        if !(self.check(TokenType::Select)
            || self.check(TokenType::With)
            || self.check(TokenType::LParen))
        {
            return Err(self.parse_error("Expected a query after POLICY_CONTEXT ( ... ) AS"));
        }
        let query = self.parse_statement()?;
        Ok(Expression::ExecuteUsingPolicyContext(Box::new(
            ExecuteUsingPolicyContext { arguments, query },
        )))
    }

    /// Snowflake `EXECUTE ALERT | TASK [ GRAPH ] | DCM PROJECT <name>` with its action, or none.
    pub(super) fn parse_snowflake_execute_object(&mut self) -> Result<Option<Expression>> {
        let start = self.current;
        let kind = if self.match_text_seq(&["ALERT"]) {
            "ALERT"
        } else if self.match_text_seq(&["DCM", "PROJECT"]) {
            EXECUTE_DCM_PROJECT
        } else if self.match_text_seq(&["TASK", "GRAPH"]) {
            "TASK GRAPH"
        } else if self.match_text_seq(&["TASK"]) {
            "TASK"
        } else {
            return Ok(None);
        };
        let name = self.parse_object_name_ref()?;
        let mut node = ExecuteObject {
            kind: kind.to_owned(),
            name,
            action: None,
            group: None,
            alias: None,
            config: None,
        };
        if kind.starts_with("TASK") {
            if self.match_text_seq(&["RETRY", "LAST"]) {
                node.action = Some("RETRY LAST".to_owned());
            } else if self.match_text_seq(&["RETRY", "GRAPH", "RUN", "GROUP"]) {
                node.action = Some("RETRY GRAPH RUN".to_owned());
                if !self.check(TokenType::String) {
                    return Err(self.parse_error("Expected the graph run group as a string"));
                }
                node.group = Some(self.parse_primary()?);
            } else if self.match_text_seq(&["USING", "CONFIG"]) {
                self.expect_token(TokenType::Eq)?;
                node.config = Some(self.parse_primary()?);
            }
        } else if kind == EXECUTE_DCM_PROJECT {
            if self.match_text_seq(&["PURGE"]) {
                node.action = Some("PURGE".to_owned());
                if self.match_token(TokenType::As) {
                    node.alias = Some(self.expect_identifier_or_keyword_with_quoted()?);
                }
            } else if self.match_text_seq(&["TEST", "ALL"]) {
                node.action = Some("TEST ALL".to_owned());
            } else {
                self.current = start;
                return Ok(None);
            }
        }
        Ok(Some(Expression::ExecuteObject(Box::new(node))))
    }
}
