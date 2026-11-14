//! `grant_statement` and `revoke_statement` (googlesql.tm): privileges on an object for grantees.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::schema_objects::models::{
    Grant, GrantPrincipal, Privilege, Revoke,
};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_identifier};
use crate::parse::_helpers::tree::nodes::joined;
use crate::parse::constants::MAX_GRANT_PATHS;
use crate::scan::models::TokenKind;

/// What a `GRANT` or `REVOKE` names: privileges (none for `ALL`), kind, object and grantees.
struct Grants {
    privileges: Vec<Privilege>,
    kind: Option<String>,
    securable: Identifier,
    principals: Vec<GrantPrincipal>,
}

fn principal(name: Identifier, kind: Option<&str>) -> GrantPrincipal {
    GrantPrincipal {
        name,
        is_role: false,
        is_group: false,
        is_share: false,
        kind: kind.map(str::to_owned),
    }
}

impl Parser<'_> {
    /// `GRANT privileges ON [kind [kind]] path TO grantee_list`.
    pub(crate) fn grant_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("GRANT")?;
        let grants: Grants = self.grants("TO")?;
        Ok(Expression::Grant(Box::new(Grant {
            caller: None,
            bulk: None,
            container: None,
            objects: Vec::new(),
            granted_by: None,
            options: Vec::new(),
            privileges: grants.privileges,
            kind: grants.kind,
            securable: Some(grants.securable),
            function_params: Vec::new(),
            principals: grants.principals,
            grant_option: false,
            as_principal: None,
            current_grants: None,
        })))
    }

    /// `REVOKE privileges ON [kind [kind]] path FROM grantee_list`.
    pub(crate) fn revoke_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("REVOKE")?;
        let grants: Grants = self.grants("FROM")?;
        Ok(Expression::Revoke(Box::new(Revoke {
            caller: None,
            bulk: None,
            container: None,
            objects: Vec::new(),
            granted_by: None,
            options: Vec::new(),
            privileges: grants.privileges,
            kind: grants.kind,
            securable: Some(grants.securable),
            function_params: Vec::new(),
            principals: grants.principals,
            grant_option: false,
            cascade: false,
            restrict: false,
        })))
    }

    fn grants(&mut self, direction: &str) -> Parsing<Grants> {
        let privileges: Vec<Privilege> = match self.eat_kw("ALL") {
            true => {
                self.eat_kw("PRIVILEGES");
                Vec::new()
            }
            false => self.comma_list(|parser| parser.privilege())?,
        };
        self.expect_kw("ON")?;
        let mut paths: Vec<Vec<Identifier>> = vec![self.multipart_identifier()?];
        while !self.kw(direction) && paths.len() < MAX_GRANT_PATHS {
            if paths.last().is_some_and(|path| path.len() != 1) {
                return Err(self.error());
            }
            paths.push(self.multipart_identifier()?);
        }
        self.expect_kw(direction)?;
        let securable: Identifier = joined(paths.pop().unwrap_or_default());
        let kind: Option<String> = match paths.is_empty() {
            true => None,
            false => Some(
                paths
                    .iter()
                    .flatten()
                    .map(|word| word.name.to_ascii_uppercase())
                    .collect::<Vec<String>>()
                    .join(" "),
            ),
        };
        let principals: Vec<GrantPrincipal> = self.comma_list(|parser| parser.grantee())?;
        Ok(Grants {
            privileges,
            kind,
            securable,
            principals,
        })
    }

    /// `privilege_name [( path, ... )]`; `SELECT` is a privilege name too.
    fn privilege(&mut self) -> Parsing<Privilege> {
        let name: String = match self.kw("SELECT") {
            true => self.advance()?.value,
            false => self.name(is_identifier)?.name,
        };
        let mut columns: Vec<String> = Vec::new();
        if self.is_char('(') {
            columns =
                self.parenthesized(|parser| Ok(joined(parser.multipart_identifier()?).name))?;
        }
        Ok(Privilege { name, columns })
    }

    /// `string_literal_or_parameter`: a string, `@name`, `?` or `@@name`, the last three by kind.
    fn grantee(&mut self) -> Parsing<GrantPrincipal> {
        if self.string_at(0) {
            return match self.string_literal()? {
                Expression::Literal(literal) => match *literal {
                    Literal::String(text) | Literal::RawString(text) => {
                        Ok(principal(Identifier::new(text), None))
                    }
                    _ => Err(self.error()),
                },
                _ => Err(self.error()),
            };
        }
        if self.eat_char('?') {
            return Ok(principal(Identifier::new("?"), Some("PARAMETER")));
        }
        if self.is_char('@')
            && self.is_at(1, |token| {
                matches!(token.kind, TokenKind::Word | TokenKind::Quoted)
            })
        {
            self.at += 1;
            let name: Identifier = identifier(&self.advance()?);
            return Ok(principal(name, Some("PARAMETER")));
        }
        if self.is_op("@@") {
            self.at += 1;
            let word = |token: &crate::scan::models::Token| {
                matches!(token.kind, TokenKind::Word | TokenKind::Quoted)
            };
            let mut parts: Vec<Identifier> = vec![self.name(word)?];
            while self.is_char('.') && self.is_at(1, word) {
                self.at += 1;
                parts.push(self.name(word)?);
            }
            return Ok(principal(joined(parts), Some("SYSTEM_VARIABLE")));
        }
        Err(self.error())
    }
}
