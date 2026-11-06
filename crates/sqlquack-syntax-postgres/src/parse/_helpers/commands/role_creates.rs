//! `CreateRoleStmt` (`ROLE`, `USER`, `GROUP`), `CreatePolicyStmt` and `CreateUserMappingStmt` (gram.y), as `CreateObject`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::schema_objects::models::CreateObject;

use crate::parse::_helpers::commands::alters::{iconst, name_from};
use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::commands::tables::{clause, unsupported};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_col_id};
use crate::parse::_helpers::tree::nodes::{boolean, literal, null, table, tuple};
use crate::parse::errors::ParseError;
use crate::scan::models::TokenKind;

/// The role options gram.y reads from a plain identifier, with the option each sets and its value.
const IDENT_OPTIONS: [(&str, &str, bool); 13] = [
    ("superuser", "superuser", true),
    ("nosuperuser", "superuser", false),
    ("createrole", "createrole", true),
    ("nocreaterole", "createrole", false),
    ("replication", "isreplication", true),
    ("noreplication", "isreplication", false),
    ("createdb", "createdb", true),
    ("nocreatedb", "createdb", false),
    ("login", "canlogin", true),
    ("nologin", "canlogin", false),
    ("bypassrls", "bypassrls", true),
    ("nobypassrls", "bypassrls", false),
    ("noinherit", "inherit", false),
];

/// The commands a policy may be for, `ALL` being the default.
const POLICY_COMMANDS: [&str; 5] = ["all", "select", "insert", "update", "delete"];

/// gram.y's error for `UNENCRYPTED PASSWORD` (0A000).
const UNENCRYPTED: &str = "UNENCRYPTED PASSWORD is no longer supported";

/// gram.y's error for a role option it does not know (42601).
const UNKNOWN_OPTION: &str = "unrecognized role option";

/// gram.y's error for a policy kind other than `PERMISSIVE` or `RESTRICTIVE` (42601).
const UNKNOWN_POLICY: &str = "unrecognized row security option";

/// One role option, as the `DefElem` gram.y makes: its name and value.
fn role_option(name: &str, value: Expression) -> Expression {
    tuple(vec![Expression::Identifier(Identifier::new(name)), value])
}

impl Parser<'_> {
    /// `CREATE {ROLE | USER | GROUP} RoleId [WITH] OptRoleList`, each option as gram.y names it.
    pub(crate) fn create_role(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.role_id()?]);
        self.eat_kw("with");
        node.operands = self.role_options(true)?;
        Ok(Expression::CreateObject(Box::new(node)))
    }

    /// `RoleId`: a `RoleSpec` that names a role, so not `PUBLIC` or a `CURRENT_*` word (42939).
    pub(crate) fn role_id(&mut self) -> Parsing<Identifier> {
        let start: usize = self.offset();
        match self.role_spec()? {
            Expression::Identifier(name) => Ok(name),
            Expression::Var(word) => Err(ParseError {
                message: format!("{} cannot be used as a role name here", word.this),
                position: start,
                syntax: false,
            }),
            _ => Err(self.error()),
        }
    }

    /// `OptRoleList` (`create`) or `AlterOptRoleList`: the options, in order.
    pub(crate) fn role_options(&mut self, create: bool) -> Parsing<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();
        while let Some(option) = self.role_option(create)? {
            options.push(option);
        }
        Ok(options)
    }

    /// One `CreateOptRoleElem` (`create`) or `AlterOptRoleElem`, or `None` when none comes next.
    fn role_option(&mut self, create: bool) -> Parsing<Option<Expression>> {
        let start: usize = self.offset();
        let Some(token) = self.peek().cloned() else {
            return Ok(None);
        };
        if matches!(token.kind, TokenKind::Ident | TokenKind::QuotedIdent) {
            let word: String = identifier(&token).name;
            let Some((_, name, value)) =
                IDENT_OPTIONS.iter().find(|(option, _, _)| *option == word)
            else {
                return Err(self.error_with(&format!("{UNKNOWN_OPTION} \"{word}\""), start));
            };
            self.at += 1;
            return Ok(Some(role_option(name, boolean(*value))));
        }
        let Some(word) = self.keyword_at(0) else {
            return Ok(None);
        };
        let option: Expression = match word {
            "password" => {
                self.at += 1;
                let value: Expression = match self.eat_kw("null") {
                    true => null(),
                    false => self.sconst()?,
                };
                role_option("password", value)
            }
            "encrypted" => {
                self.at += 1;
                self.expect_kw("password")?;
                role_option("password", self.sconst()?)
            }
            "unencrypted" => {
                self.at += 1;
                self.expect_kw("password")?;
                self.sconst()?;
                return Err(unsupported(UNENCRYPTED.to_owned(), start));
            }
            "inherit" => {
                self.at += 1;
                role_option("inherit", boolean(true))
            }
            "connection" => {
                self.at += 1;
                self.expect_kw("limit")?;
                role_option("connectionlimit", self.signed_iconst()?)
            }
            "valid" => {
                self.at += 1;
                self.expect_kw("until")?;
                role_option("validUntil", self.sconst()?)
            }
            "user" => {
                self.at += 1;
                role_option("rolemembers", self.role_list()?)
            }
            "sysid" if create => {
                self.at += 1;
                let token = self.advance()?;
                if token.kind != TokenKind::Integer {
                    self.at -= 1;
                    return Err(self.error());
                }
                role_option("sysid", integer(iconst(&token.value)))
            }
            "admin" if create => {
                self.at += 1;
                role_option("adminmembers", self.role_list()?)
            }
            "role" if create => {
                self.at += 1;
                role_option("rolemembers", self.role_list()?)
            }
            "in" if create && (self.kw_at(1, "role") || self.kw_at(1, "group")) => {
                self.at += 2;
                role_option("addroleto", self.role_list()?)
            }
            _ => return Ok(None),
        };
        Ok(Some(option))
    }

    /// `SignedIconst`: an integer with an optional sign, as its value.
    fn signed_iconst(&mut self) -> Parsing<Expression> {
        let negative: bool = self.is_char('-');
        if negative || self.is_char('+') {
            self.at += 1;
        }
        let token = self.advance()?;
        if token.kind != TokenKind::Integer {
            self.at -= 1;
            return Err(self.error());
        }
        let value: i64 = iconst(&token.value);
        Ok(integer(match negative {
            true => -value,
            false => value,
        }))
    }

    /// `role_list`: `RoleSpec, ...`, as a tuple.
    pub(crate) fn role_list(&mut self) -> Parsing<Expression> {
        Ok(tuple(self.comma_list(|parser| parser.role_spec())?))
    }

    /// `CreatePolicyStmt`, with gram.y's defaults written out: `AS PERMISSIVE`, `FOR ALL`, `TO PUBLIC`.
    pub(crate) fn create_policy(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.name = table(vec![self.name(is_col_id)?]);
        self.expect_kw("on")?;
        let relation: Expression = Expression::Table(Box::new(table(self.table_name()?)));
        node.operands.push(clause("ON", vec![relation]));
        let mut kind: &str = "PERMISSIVE";
        if self.eat_kw("as") {
            let start: usize = self.offset();
            let token = self.advance()?;
            let word: String = match token.kind {
                TokenKind::Ident | TokenKind::QuotedIdent => identifier(&token).name,
                _ => {
                    self.at -= 1;
                    return Err(self.error());
                }
            };
            kind = match word.as_str() {
                "permissive" => "PERMISSIVE",
                "restrictive" => "RESTRICTIVE",
                _ => return Err(self.error_with(&format!("{UNKNOWN_POLICY} \"{word}\""), start)),
            };
        }
        node.operands.push(clause("AS", vec![var(kind)]));
        let mut command: &str = "all";
        if self.eat_kw("for") {
            let Some(word) = POLICY_COMMANDS.iter().find(|word| self.kw(word)) else {
                return Err(self.error());
            };
            self.at += 1;
            command = word;
        }
        node.operands
            .push(clause("FOR", vec![var(&command.to_uppercase())]));
        let roles: Expression = match self.eat_kw("to") {
            true => self.role_list()?,
            false => tuple(vec![var("PUBLIC")]),
        };
        node.operands.push(clause("TO", vec![roles]));
        if self.eat_kw("using") {
            node.operands
                .push(clause("USING", vec![self.parenthesized_condition()?]));
        }
        if self.kw("with") && self.kw_at(1, "check") {
            self.at += 2;
            node.operands
                .push(clause("WITH CHECK", vec![self.parenthesized_condition()?]));
        }
        Ok(Expression::CreateObject(Box::new(node)))
    }

    /// `CreateUserMappingStmt`: `[IF NOT EXISTS] FOR auth_ident SERVER name [OPTIONS]`; `USER` is `CURRENT_USER`.
    pub(crate) fn create_user_mapping(&mut self, mut node: CreateObject) -> Parsing<Expression> {
        node.if_not_exists = self.if_not_exists()?;
        self.expect_kw("for")?;
        let user: Expression = match self.eat_kw("user") {
            true => var("CURRENT_USER"),
            false => self.role_spec()?,
        };
        node.operands.push(clause("FOR", vec![user]));
        self.expect_kw("server")?;
        node.operands
            .push(clause("SERVER", vec![name_from(self.name(is_col_id)?)]));
        node.operands.extend(self.create_generic_options()?);
        Ok(Expression::CreateObject(Box::new(node)))
    }
}

/// An integer value.
fn integer(value: i64) -> Expression {
    literal(Literal::Number(value.to_string()))
}
