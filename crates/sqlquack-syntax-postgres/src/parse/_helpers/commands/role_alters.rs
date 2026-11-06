//! `AlterRoleStmt`, `AlterRoleSetStmt`, `AlterGroupStmt`, `AlterPolicyStmt`, `AlterSystemStmt` and `AlterUserMappingStmt` (gram.y), as `AlterObject`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::TableRef;

use crate::parse::_helpers::commands::alters::{alter, name_from};
use crate::parse::_helpers::commands::role_creates::role_option;
use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::commands::tables::clause;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id};
use crate::parse::_helpers::tree::nodes::table;
use crate::parse::errors::ParseError;

/// The kind of every role `ALTER`: `ROLE`, `USER` and `GROUP` make one `AlterRoleStmt` or `RenameStmt`.
const ROLE: &str = "ROLE";

/// The kind of `ALTER POLICY`.
const POLICY: &str = "POLICY";

/// The kind of `ALTER SYSTEM`.
const SYSTEM: &str = "SYSTEM";

/// The kind of `ALTER USER MAPPING`.
const USER_MAPPING: &str = "USER MAPPING";

/// The action of `RenameStmt`.
const RENAME: &str = "RENAME";

/// The action of `ALTER GROUP ... DROP USER` (`AlterRoleStmt.action` -1); options and `ADD USER` have none.
const DROP_MEMBERS: &str = "DROP";

/// The `RoleSpec` every role is, which `RoleId` refuses as reserved.
const PUBLIC: &str = "PUBLIC";

/// The role option `USER role_list` and `ALTER GROUP ... USER role_list` set.
const MEMBERS_OPTION: &str = "rolemembers";

/// The word after `ALTER` that makes `AlterGroupStmt`.
const GROUP_WORD: &str = "group";

/// The words after `ALTER` that alter a role.
const ROLE_KINDS: [&str; 3] = ["role", "user", GROUP_WORD];

/// The `ALTER` kinds this module alters, by the word after `ALTER`.
#[derive(Clone, Copy)]
pub(crate) enum RoleAlter {
    /// `ALTER ROLE`, `ALTER USER` or `ALTER GROUP`, with whether it is `GROUP`.
    Role(bool),
    /// `ALTER POLICY`.
    Policy,
    /// `ALTER SYSTEM`.
    System,
    /// `ALTER USER MAPPING`.
    UserMapping,
}

impl Parser<'_> {
    /// The role, policy, system or user-mapping `ALTER` here, if it is one.
    pub(crate) fn altered_role(&self) -> Option<RoleAlter> {
        if !self.kw("alter") {
            return None;
        }
        if self.kw_at(1, "user") && self.kw_at(2, "mapping") && self.kw_at(3, "for") {
            return Some(RoleAlter::UserMapping);
        }
        if self.kw_at(1, "policy") {
            return Some(RoleAlter::Policy);
        }
        if self.kw_at(1, "system") {
            return Some(RoleAlter::System);
        }
        ROLE_KINDS
            .iter()
            .any(|word| self.kw_at(1, word))
            .then(|| RoleAlter::Role(self.kw_at(1, GROUP_WORD)))
    }

    /// The `ALTER` statement `which` names.
    pub(crate) fn alter_role_object(&mut self, which: RoleAlter) -> Parsing<Expression> {
        match which {
            RoleAlter::Role(group) => {
                self.at += 2;
                self.alter_role(group)
            }
            RoleAlter::Policy => {
                self.at += 2;
                self.alter_policy()
            }
            RoleAlter::System => {
                self.at += 2;
                let setting: Expression = self.generic_set_reset()?;
                Ok(alter((SYSTEM, table(Vec::new()), false), "", vec![setting]))
            }
            RoleAlter::UserMapping => {
                self.at += 4;
                self.alter_user_mapping()
            }
        }
    }

    /// `ALTER {ROLE | USER | GROUP}`: options, members, a setting, or a rename.
    fn alter_role(&mut self, group: bool) -> Parsing<Expression> {
        let start: usize = self.offset();
        let every: bool = !group && self.eat_kw("all");
        let role: Expression = match every {
            true => var("ALL"),
            false => self.role_spec()?,
        };
        let (name, mut operands): (TableRef, Vec<Expression>) = match role {
            Expression::Identifier(name) => (table(vec![name]), Vec::new()),
            other => (table(Vec::new()), vec![other]),
        };
        if !every && self.kw("rename") {
            if !operands.is_empty() {
                return Err(role_name_refused(&operands[0], start));
            }
            self.at += 1;
            self.expect_kw("to")?;
            operands.push(name_from(self.role_id()?));
            return Ok(alter((ROLE, name, false), RENAME, operands));
        }
        if group {
            let dropped: bool = !self.kw("add");
            if dropped {
                self.expect_kw("drop")?;
            } else {
                self.at += 1;
            }
            self.expect_kw("user")?;
            operands.push(role_option(MEMBERS_OPTION, self.role_list()?));
            let action: &str = if dropped { DROP_MEMBERS } else { "" };
            return Ok(alter((ROLE, name, false), action, operands));
        }
        let setting: bool = self.kw("set") || self.kw("reset") || self.kw("in");
        if every || setting {
            if self.eat_kw("in") {
                self.expect_kw("database")?;
                operands.push(clause(
                    "IN DATABASE",
                    vec![name_from(self.name(is_col_id)?)],
                ));
            }
            operands.push(self.set_reset_clause()?);
            return Ok(alter((ROLE, name, false), "", operands));
        }
        self.eat_kw("with");
        operands.extend(self.role_options(false)?);
        Ok(alter((ROLE, name, false), "", operands))
    }

    /// `ALTER POLICY [IF EXISTS] name ON qualified_name` then a rename, or roles, `USING` and `WITH CHECK`.
    fn alter_policy(&mut self) -> Parsing<Expression> {
        let if_exists: bool = self.kw("if") && self.kw_at(1, "exists");
        if if_exists {
            self.at += 2;
        }
        let name: TableRef = table(vec![self.name(is_col_id)?]);
        self.expect_kw("on")?;
        let relation: Expression = Expression::Table(Box::new(table(self.table_name()?)));
        let mut operands: Vec<Expression> = vec![clause("ON", vec![relation])];
        if if_exists || self.kw("rename") {
            self.expect_kw("rename")?;
            self.expect_kw("to")?;
            operands.push(name_from(self.name(is_col_id)?));
            return Ok(alter((POLICY, name, if_exists), RENAME, operands));
        }
        if self.eat_kw("to") {
            operands.push(clause("TO", vec![self.role_list()?]));
        }
        if self.eat_kw("using") {
            operands.push(clause("USING", vec![self.parenthesized_condition()?]));
        }
        if self.kw("with") && self.kw_at(1, "check") {
            self.at += 2;
            operands.push(clause("WITH CHECK", vec![self.parenthesized_condition()?]));
        }
        Ok(alter((POLICY, name, false), "", operands))
    }

    /// `ALTER USER MAPPING FOR auth_ident SERVER name alter_generic_options`; `USER` is `CURRENT_USER`.
    fn alter_user_mapping(&mut self) -> Parsing<Expression> {
        let user: Expression = match self.eat_kw("user") {
            true => var("CURRENT_USER"),
            false => self.role_spec()?,
        };
        let mut operands: Vec<Expression> = vec![clause("FOR", vec![user])];
        self.expect_kw("server")?;
        operands.push(clause("SERVER", vec![name_from(self.name(is_col_id)?)]));
        operands.push(clause("OPTIONS", self.generic_options()?));
        Ok(alter(
            (USER_MAPPING, table(Vec::new()), false),
            "",
            operands,
        ))
    }
}

/// `RoleId`'s error (42939) for a `RoleSpec` that is not a role name.
fn role_name_refused(role: &Expression, position: usize) -> ParseError {
    let message: String = match role {
        Expression::Var(word) if word.this == PUBLIC => {
            format!("role name \"{}\" is reserved", PUBLIC.to_lowercase())
        }
        Expression::Var(word) => format!("{} cannot be used as a role name here", word.this),
        _ => String::new(),
    };
    ParseError {
        message,
        position,
        syntax: false,
    }
}
