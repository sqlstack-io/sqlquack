//! Snowflake's rules on COMMENT ON names and values, and its class instance method calls.

use crate::expressions::names::models::Identifier;
use crate::expressions::operators::models::Call;
use crate::expressions::statements::models::{Grant, GrantPrincipal, Revoke};
use crate::expressions::tree::models::Expression;
use crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// The roles a Snowflake GRANT or REVOKE without ON hands out, each with the grantee kinds it takes.
const ROLE_GRANTS: &[(&[&str], &[&[&str]])] = &[
    (&["ROLE"], &[&["ROLE"], &["USER"]]),
    (
        &["DATABASE", "ROLE"],
        &[
            &["DATABASE", "ROLE"],
            &["ROLE"],
            &["USER"],
            &["APPLICATION"],
            &["SHARE"],
        ],
    ),
    (
        &["APPLICATION", "ROLE"],
        &[
            &["APPLICATION", "ROLE"],
            &["APPLICATION"],
            &["USER"],
            &["ROLE"],
        ],
    ),
];

impl Parser {
    /// In Snowflake, parse a GRANT (or, when `revoke`, a REVOKE) of a role after its first word; None outside Snowflake or for a SERVICE ROLE.
    pub(super) fn parse_snowflake_role_grant(
        &mut self,
        revoke: bool,
    ) -> Result<Option<Expression>> {
        if !self.is_snowflake() || self.check_text_seq(&["SERVICE", "ROLE"]) {
            return Ok(None);
        }
        let grant_option = revoke && self.match_text_seq(&["GRANT"]) && {
            self.expect_alter_words(&[&["OPTION", "FOR"]])?;
            true
        };
        let Some((kind, grantees)) = ROLE_GRANTS
            .iter()
            .find(|(kind, _)| self.match_text_seq(kind))
        else {
            return Err(self.parse_error("Expected ROLE, DATABASE ROLE or APPLICATION ROLE"));
        };
        let role = securable_identifier(self.parse_object_name()?);
        self.expect_token(if revoke {
            TokenType::From
        } else {
            TokenType::To
        })?;
        let grantee = self.expect_alter_words(grantees)?;
        let principal = GrantPrincipal {
            name: securable_identifier(self.parse_object_name()?),
            is_role: grantee == "ROLE",
            is_group: false,
            is_share: grantee == "SHARE",
            kind: (!matches!(grantee.as_str(), "ROLE" | "SHARE")).then_some(grantee),
        };
        let kind = Some(kind.join(" "));
        let securable = role;
        if revoke {
            let cascade = self.match_token(TokenType::Cascade);
            let restrict = !cascade && self.match_token(TokenType::Restrict);
            return Ok(Some(Expression::Revoke(Box::new(Revoke {
                privileges: Vec::new(),
                kind,
                securable,
                function_params: Vec::new(),
                principals: vec![principal],
                grant_option,
                cascade,
                restrict,
            }))));
        }
        let grant_option = self.match_token(TokenType::With) && {
            self.expect_token(TokenType::Grant)?;
            self.expect_alter_words(&[&["OPTION"]])?;
            true
        };
        Ok(Some(Expression::Grant(Box::new(Grant {
            privileges: Vec::new(),
            kind,
            securable,
            function_params: Vec::new(),
            principals: vec![principal],
            grant_option,
            as_principal: None,
        }))))
    }

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

    /// Parse a Snowflake class instance method call, `<instance>!<method>( [ <arg> , ... ] )`, after CALL; None for any other CALL.
    pub(super) fn parse_snowflake_method_call(&mut self) -> Result<Option<Expression>> {
        if !self.is_snowflake() {
            return Ok(None);
        }
        if self.check(TokenType::Exclamation) {
            return Err(self.parse_error("Expected an instance name before !"));
        }
        let mut offset = 0;
        while self
            .tokens
            .get(self.current + offset + 1)
            .map(|token| token.token_type)
            == Some(TokenType::Dot)
        {
            offset += 2;
        }
        if self
            .tokens
            .get(self.current + offset + 1)
            .map(|token| token.token_type)
            != Some(TokenType::Exclamation)
        {
            return Ok(None);
        }
        let mut name = vec![self.expect_identifier_or_keyword_with_quoted()?];
        while self.match_token(TokenType::Dot) {
            name.push(self.expect_identifier_or_keyword_with_quoted()?);
        }
        for part in &name {
            self.reject_reserved_name_part(part)?;
        }
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
}

/// A role or grantee name as one identifier, its parts joined by dots like a GRANT securable.
fn securable_identifier(name: crate::expressions::names::models::TableRef) -> Identifier {
    match (name.catalog, name.schema) {
        (None, None) => name.name,
        (catalog, schema) => Identifier::new(
            [catalog, schema, Some(name.name)]
                .into_iter()
                .flatten()
                .map(|part| part.name)
                .collect::<Vec<_>>()
                .join("."),
        ),
    }
}
