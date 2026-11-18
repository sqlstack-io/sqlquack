//! Parser statements: USE.

use crate::parser::models::Parser;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::statements::models::Use;
use sqlquack_syntax::expressions::statements::models::UseKind;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    /// Parse USE statement: USE db, USE DATABASE x, USE SCHEMA x.y, USE ROLE x, etc.
    pub(crate) fn parse_use(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::Use)?;

        if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("SECONDARY") {
            self.skip();
            if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("ROLES") {
                self.skip();
                let snowflake = self.is_snowflake();
                let mut roles: Vec<String> = Vec::new();
                let mut listed: Vec<Identifier> = Vec::new();
                loop {
                    if snowflake && self.check(TokenType::QuotedIdentifier) {
                        listed.push(Identifier::quoted(self.peek_text()));
                        roles.push(self.parse_quoted_role()?);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    } else if self.check(TokenType::Var)
                        || self.check(TokenType::All)
                        || self.check(TokenType::Identifier)
                    {
                        let role = self.advance_text()?;
                        listed.push(Identifier::new(role.clone()));
                        roles.push(role);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    } else if snowflake && !roles.is_empty() {
                        return Err(self.parse_error("Expected a role after the comma"));
                    } else {
                        break;
                    }
                }
                if snowflake && roles.is_empty() {
                    return Err(self.parse_error("Expected ALL, NONE or a role after ROLES"));
                }
                let keyword = match roles.as_slice() {
                    [role] if role.eq_ignore_ascii_case("ALL") => Some(UseKind::SecondaryRolesAll),
                    [role] if role.eq_ignore_ascii_case("NONE") => {
                        Some(UseKind::SecondaryRolesNone)
                    }
                    _ => None,
                };
                let first = listed.first().cloned();
                let (kind, this, roles) = match (snowflake, keyword, first) {
                    (true, Some(UseKind::SecondaryRolesAll), _) => (
                        UseKind::SecondaryRolesAll,
                        Identifier::new("ALL"),
                        Vec::new(),
                    ),
                    (true, Some(_), _) => (
                        UseKind::SecondaryRolesNone,
                        Identifier::new("NONE"),
                        Vec::new(),
                    ),
                    (true, None, Some(first)) => (UseKind::SecondaryRoles, first, listed),
                    _ => {
                        let name = match roles.is_empty() {
                            true => "ALL".to_string(),
                            false => roles.join(", "),
                        };
                        (UseKind::SecondaryRoles, Identifier::new(name), Vec::new())
                    }
                };
                return Ok(Expression::Use(Box::new(Use {
                    name_written: None,
                    kind: Some(kind),
                    this,
                    roles,
                    parts: Vec::new(),
                    string_name: false,
                    keyword: None,
                })));
            }
        }

        let kind_is_name = self.applies_snowflake_context_rules()
            && matches!(
                self.tokens
                    .get(self.current + 1)
                    .map(|token| token.token_type),
                None | Some(TokenType::Semicolon | TokenType::Dot)
            );
        let kind = if kind_is_name {
            None
        } else if self.match_token(TokenType::Database) {
            Some(UseKind::Database)
        } else if self.match_token(TokenType::Schema) {
            Some(UseKind::Schema)
        } else if self.match_token(TokenType::Warehouse) {
            Some(UseKind::Warehouse)
        } else if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("ROLE") {
            self.skip();
            Some(UseKind::Role)
        } else if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("CATALOG") {
            self.skip();
            Some(UseKind::Catalog)
        } else {
            None
        };

        let mut ident = self.expect_identifier_or_keyword_with_quoted()?;
        self.reject_reserved_name_part(&ident)?;
        let mut parts: Vec<Identifier> = Vec::new();

        if self.match_token(TokenType::Dot) {
            let second_part = self.expect_identifier_or_keyword_with_quoted()?;
            self.reject_reserved_name_part(&second_part)?;
            if self.applies_snowflake_context_rules() {
                parts = vec![ident.clone(), second_part.clone()];
            }
            ident.name = format!("{}.{}", ident.name, second_part.name);
        }

        Ok(Expression::Use(Box::new(Use {
            name_written: None,
            kind,
            this: ident,
            roles: Vec::new(),
            parts,
            string_name: false,
            keyword: None,
        })))
    }
}
