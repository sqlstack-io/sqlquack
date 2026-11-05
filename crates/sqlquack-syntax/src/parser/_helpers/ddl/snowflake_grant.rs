//! Snowflake GRANT and REVOKE of privileges, roles and service roles, as Snowflake parses them.

use crate::expressions::names::models::Identifier;
use crate::expressions::statements::models::{Grant, GrantPrincipal, Privilege, Revoke};
use crate::expressions::tree::models::Expression;
use crate::parser::constants::{SNOWFLAKE_GRANT_KINDS, SNOWFLAKE_GRANT_PRIVILEGES};
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

/// The grantees of a privilege; a bare name is a role.
const PRIVILEGE_GRANTEES: &[&str] = &[
    "APPLICATION ROLE",
    "DATABASE ROLE",
    "APPLICATION",
    "DATABASE",
    "ROLE",
    "SHARE",
    "USER",
];

/// The roles a GRANT or REVOKE hands over, each with the grantees it takes; none takes a bare name.
const ROLE_GRANTS: &[(&str, &[&str])] = &[
    ("ROLE", &["ROLE", "USER"]),
    (
        "DATABASE ROLE",
        &["DATABASE ROLE", "APPLICATION", "ROLE", "SHARE", "USER"],
    ),
    (
        "APPLICATION ROLE",
        &["APPLICATION ROLE", "APPLICATION", "ROLE", "USER"],
    ),
    (
        "SERVICE ROLE",
        &["APPLICATION ROLE", "DATABASE ROLE", "ROLE"],
    ),
];

impl Parser {
    /// A Snowflake GRANT (`revoke` false) or REVOKE after its keyword.
    pub(crate) fn parse_snowflake_grant(&mut self, revoke: bool) -> Result<Expression> {
        let grant_option = revoke && self.check(TokenType::Grant);
        if grant_option && !self.match_text_seq(&["GRANT", "OPTION", "FOR"]) {
            return Err(self.parse_error("Expected GRANT OPTION FOR"));
        }
        let role = ROLE_GRANTS
            .iter()
            .find(|(kind, _)| self.match_text_seq(&kind.split(' ').collect::<Vec<_>>()));
        let (privileges, kind, securable, function_params, grantees, bare) = match role {
            Some((kind, grantees)) => {
                let securable = if matches!(*kind, "SERVICE ROLE") {
                    let service = self.parse_grant_securable()?;
                    self.expect_token(TokenType::Exclamation)?;
                    let role = self.parse_grant_name_part()?;
                    Identifier::new(format!("{}!{}", service.name, role.name))
                } else {
                    self.parse_grant_securable()?
                };
                (
                    Vec::new(),
                    Some(kind.to_string()),
                    securable,
                    Vec::new(),
                    *grantees,
                    false,
                )
            }
            None => {
                let mut privileges = vec![self.parse_snowflake_privilege()?];
                while self.match_token(TokenType::Comma) {
                    privileges.push(self.parse_snowflake_privilege()?);
                }
                self.expect_token(TokenType::On)?;
                let inherited = privileges
                    .iter()
                    .any(|privilege| privilege.name.contains("INHERITED"));
                let (kind, securable) = self.parse_grant_target(inherited)?;
                let function_params = if self.check(TokenType::LParen) {
                    self.parse_function_param_types()?
                } else {
                    Vec::new()
                };
                (
                    privileges,
                    kind,
                    securable,
                    function_params,
                    PRIVILEGE_GRANTEES,
                    true,
                )
            }
        };
        self.expect_token(if revoke {
            TokenType::From
        } else {
            TokenType::To
        })?;
        let principals = vec![self.parse_grant_grantee(grantees, bare)?];
        if revoke {
            let cascade = self.match_token(TokenType::Cascade);
            let restrict = !cascade && self.match_token(TokenType::Restrict);
            return Ok(Expression::Revoke(Box::new(Revoke {
                privileges,
                kind,
                securable,
                function_params,
                principals,
                grant_option,
                cascade,
                restrict,
            })));
        }
        let grant_option = self.check(TokenType::With);
        if grant_option && !self.match_text_seq(&["WITH", "GRANT", "OPTION"]) {
            return Err(self.parse_error("Expected WITH GRANT OPTION"));
        }
        let current_grants = if self.check(TokenType::Copy) || self.check(TokenType::Revoke) {
            let word = self.advance_text()?.to_ascii_uppercase();
            if !self.match_text_seq(&["CURRENT", "GRANTS"]) {
                return Err(self.parse_error("Expected CURRENT GRANTS"));
            }
            Some(word)
        } else {
            None
        };
        Ok(Expression::Grant(Box::new(Grant {
            privileges,
            kind,
            securable,
            function_params,
            principals,
            grant_option,
            as_principal: None,
            current_grants,
        })))
    }

    /// One privilege: ALL [ INHERITED ] [ CALLER ] [ PRIVILEGES ], CREATE SNOWFLAKE.<class>, USE AI FUNCTION <name>, or a documented one after INHERITED and CALLER.
    fn parse_snowflake_privilege(&mut self) -> Result<Privilege> {
        let mut words: Vec<String> = Vec::new();
        if self.match_text_seq(&["ALL"]) {
            words.push("ALL".to_string());
            for word in ["INHERITED", "CALLER"] {
                if self.match_text_seq(&[word]) {
                    words.push(word.to_string());
                }
            }
            if self.match_text_seq(&["PRIVILEGES"]) {
                words.push("PRIVILEGES".to_string());
            } else if words.len() > 1 {
                return Err(self.parse_error("Expected PRIVILEGES"));
            }
            return Ok(privilege(words.join(" ")));
        }
        for word in ["INHERITED", "CALLER"] {
            if self.match_text_seq(&[word]) {
                words.push(word.to_string());
            }
        }
        if self.check_text_seq(&["CREATE", "SNOWFLAKE"]) && self.check_at(2, TokenType::Dot) {
            self.match_text_seq(&["CREATE", "SNOWFLAKE"]);
            self.expect_token(TokenType::Dot)?;
            let schema = if self.check(TokenType::Dot) {
                String::new()
            } else {
                self.parse_grant_name_part()?.name
            };
            self.expect_token(TokenType::Dot)?;
            let class = self.parse_grant_name_part()?.name;
            words.push(format!("CREATE SNOWFLAKE.{schema}.{class}"));
        } else if self.match_text_seq(&["USE", "AI", "FUNCTION"]) {
            let name = self.parse_grant_name_part()?;
            words.push(format!("USE AI FUNCTION {}", name.name));
        } else {
            let found = SNOWFLAKE_GRANT_PRIVILEGES
                .iter()
                .map(|name| name.split(' ').collect::<Vec<_>>())
                .filter(|parts| {
                    self.check_text_seq(parts)
                        && (self.check_at(parts.len(), TokenType::On)
                            || self.check_at(parts.len(), TokenType::Comma))
                })
                .max_by_key(Vec::len);
            let Some(parts) = found else {
                return Err(self.parse_error("Expected a privilege"));
            };
            self.match_text_seq(&parts);
            words.push(parts.join(" "));
        }
        Ok(privilege(words.join(" ")))
    }

    /// The target after ON: ACCOUNT, { ALL | FUTURE } <plural> IN <container>, or [ <kind> ] <name>; IN ACCOUNT takes an INHERITED privilege.
    fn parse_grant_target(&mut self, inherited: bool) -> Result<(Option<String>, Identifier)> {
        if self.check_identifier("ACCOUNT") && self.check_grant_end(1) {
            self.skip();
            return Ok((Some("ACCOUNT".to_string()), Identifier::new("")));
        }
        let future = self.check_identifier("FUTURE");
        if future || self.check_text_seq(&["ALL"]) {
            let mut words = vec![self.advance_text()?.to_ascii_uppercase()];
            while !self.check(TokenType::In) {
                words.push(self.parse_grant_name_part()?.name.to_ascii_uppercase());
            }
            if words.len() == 1 {
                return Err(self.parse_error("Expected the plural of an object kind"));
            }
            self.expect_token(TokenType::In)?;
            words.push("IN".to_string());
            if !future && inherited && self.check_identifier("ACCOUNT") {
                self.skip();
                words.push("ACCOUNT".to_string());
                return Ok((Some(words.join(" ")), Identifier::new("")));
            }
            let containers: &[&str] = if future {
                &["DATABASE", "SCHEMA"]
            } else {
                &["APPLICATION PACKAGE", "APPLICATION", "DATABASE", "SCHEMA"]
            };
            let Some(container) = self.match_grant_kind(containers) else {
                return Err(self.parse_error("Expected DATABASE or SCHEMA"));
            };
            words.push(container);
            return Ok((Some(words.join(" ")), self.parse_grant_securable()?));
        }
        let kind = self.match_grant_kind(SNOWFLAKE_GRANT_KINDS).or_else(|| {
            let generic = self.check_grant_name_start(0)
                && self.check_grant_name_start(1)
                && !self.check_at(2, TokenType::Dot)
                && !SNOWFLAKE_GRANT_KINDS.iter().any(|kind| {
                    kind.split(' ')
                        .next()
                        .is_some_and(|first| kind.contains(' ') && self.check_identifier(first))
                });
            generic.then(|| {
                let word = self.peek_text().to_ascii_uppercase();
                self.skip();
                word
            })
        });
        Ok((kind, self.parse_grant_securable()?))
    }

    /// The grantee after TO or FROM: one of `kinds` and its name, or a bare role name when `bare`.
    fn parse_grant_grantee(&mut self, kinds: &[&str], bare: bool) -> Result<GrantPrincipal> {
        let found = kinds
            .iter()
            .map(|kind| kind.split(' ').collect::<Vec<_>>())
            .find(|parts| self.check_text_seq(parts));
        let (kind, name) = match found {
            Some(parts)
                if self.check_grant_name_start(parts.len())
                    && !self.check_revoke_option_end(parts.len()) =>
            {
                self.match_text_seq(&parts);
                (Some(parts.join(" ")), self.parse_grant_name()?)
            }
            Some(parts) if parts.len() > 1 => {
                self.match_text_seq(&parts[..1]);
                (Some(parts[0].to_string()), self.parse_grant_name_part()?)
            }
            Some(_) | None if bare => (None, self.parse_grant_name()?),
            _ => return Err(self.parse_error("Expected a grantee kind and name")),
        };
        let is_role = matches!(kind.as_deref(), Some("ROLE"));
        let is_share = matches!(kind.as_deref(), Some("SHARE"));
        Ok(GrantPrincipal {
            name,
            is_role,
            is_group: false,
            is_share,
            kind: kind.filter(|_| !is_role && !is_share),
        })
    }

    /// The longest of `kinds` at the cursor that a name follows; a kind before TO or FROM is the name itself.
    fn match_grant_kind(&mut self, kinds: &[&str]) -> Option<String> {
        let parts = kinds
            .iter()
            .map(|kind| kind.split(' ').collect::<Vec<_>>())
            .filter(|parts| self.check_text_seq(parts) && self.check_grant_name_start(parts.len()))
            .max_by_key(Vec::len)?;
        self.match_text_seq(&parts);
        Some(parts.join(" "))
    }

    /// A securable name; like polyglot's securable it carries no source span.
    fn parse_grant_securable(&mut self) -> Result<Identifier> {
        Ok(Identifier {
            span: None,
            ..self.parse_grant_name()?
        })
    }

    /// A possibly dotted object name; each part is an identifier, not a string or reserved word.
    fn parse_grant_name(&mut self) -> Result<Identifier> {
        let mut parts = vec![self.parse_grant_name_part()?];
        while self.match_token(TokenType::Dot) {
            parts.push(self.parse_grant_name_part()?);
        }
        if parts.len() == 1 {
            return Ok(parts.remove(0));
        }
        Ok(Identifier::new(
            parts
                .iter()
                .map(|part| {
                    if part.quoted {
                        format!("\"{}\"", part.name)
                    } else {
                        part.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("."),
        ))
    }

    /// One name part.
    fn parse_grant_name_part(&mut self) -> Result<Identifier> {
        if !self.check_grant_name_start(0) {
            return Err(self.parse_error("Expected an object name"));
        }
        self.expect_identifier_or_keyword_with_quoted()
    }

    /// Whether the token `offset` ahead can start a name: an identifier or unreserved keyword.
    fn check_grant_name_start(&self, offset: usize) -> bool {
        let Some(token) = self.tokens.get(self.current + offset) else {
            return false;
        };
        match token.token_type {
            TokenType::QuotedIdentifier | TokenType::Identifier | TokenType::Var => {
                token.token_type == TokenType::QuotedIdentifier
                    || !crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS
                        .contains(&token.text.to_ascii_uppercase().as_str())
            }
            TokenType::String | TokenType::Semicolon | TokenType::Dot | TokenType::Comma => false,
            _ => {
                token
                    .text
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    && !crate::parser::constants::SNOWFLAKE_RESERVED_KEYWORDS
                        .contains(&token.text.to_ascii_uppercase().as_str())
            }
        }
    }

    /// Whether the token `offset` ahead is TO or FROM.
    fn check_grant_end(&self, offset: usize) -> bool {
        self.check_at(offset, TokenType::To) || self.check_at(offset, TokenType::From)
    }

    /// Whether the token `offset` ahead is a closing CASCADE or RESTRICT, which no name follows.
    fn check_revoke_option_end(&self, offset: usize) -> bool {
        (self.check_at(offset, TokenType::Cascade) || self.check_at(offset, TokenType::Restrict))
            && self
                .tokens
                .get(self.current + offset + 1)
                .is_none_or(|token| token.token_type == TokenType::Semicolon)
    }

    /// Whether the token `offset` ahead has the given type.
    fn check_at(&self, offset: usize, token_type: TokenType) -> bool {
        self.tokens
            .get(self.current + offset)
            .is_some_and(|token| token.token_type == token_type)
    }
}

/// A privilege without columns.
fn privilege(name: String) -> Privilege {
    Privilege {
        name,
        columns: Vec::new(),
    }
}
