//! `GrantStmt`, `RevokeStmt`, `GrantRoleStmt`, `RevokeRoleStmt` and `AlterDefaultPrivilegesStmt` (gram.y), as `Grant`, `Revoke` and `AlterDefaultPrivileges`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{
    AlterDefaultPrivileges, Grant, GrantPrincipal, Privilege, Revoke,
};

use crate::parse::_helpers::commands::alters::name_from;
use crate::parse::_helpers::commands::tables::{clause, name_value};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_col_id, is_col_label};
use crate::parse::_helpers::tree::nodes::{boolean, table, tuple};
use crate::scan::models::TokenKind;

/// What a `privilege_target` kind names: relations, names, `any_name`s, signatures, numbers or parameters.
#[derive(Clone, Copy, PartialEq)]
enum Targets {
    /// `qualified_name_list`.
    Relations,
    /// `name_list`.
    Names,
    /// `any_name_list`.
    AnyNames,
    /// `function_with_argtypes_list`.
    Routines,
    /// `NumericOnly_list`.
    Numbers,
    /// `parameter_name_list`.
    Parameters,
}

/// The `privilege_target` kinds, longest spelling first, with the kind text and what each names.
const TARGET_KINDS: [(&[&str], &str, Targets); 20] = [
    (
        &["all", "tables", "in", "schema"],
        "ALL TABLES IN SCHEMA",
        Targets::Names,
    ),
    (
        &["all", "sequences", "in", "schema"],
        "ALL SEQUENCES IN SCHEMA",
        Targets::Names,
    ),
    (
        &["all", "functions", "in", "schema"],
        "ALL FUNCTIONS IN SCHEMA",
        Targets::Names,
    ),
    (
        &["all", "procedures", "in", "schema"],
        "ALL PROCEDURES IN SCHEMA",
        Targets::Names,
    ),
    (
        &["all", "routines", "in", "schema"],
        "ALL ROUTINES IN SCHEMA",
        Targets::Names,
    ),
    (
        &["foreign", "data", "wrapper"],
        "FOREIGN DATA WRAPPER",
        Targets::Names,
    ),
    (&["foreign", "server"], "FOREIGN SERVER", Targets::Names),
    (&["large", "object"], "LARGE OBJECT", Targets::Numbers),
    (&["table"], TABLE, Targets::Relations),
    (&["sequence"], "SEQUENCE", Targets::Relations),
    (&["function"], "FUNCTION", Targets::Routines),
    (&["procedure"], "PROCEDURE", Targets::Routines),
    (&["routine"], "ROUTINE", Targets::Routines),
    (&["database"], "DATABASE", Targets::Names),
    (&["domain"], "DOMAIN", Targets::AnyNames),
    (&["language"], "LANGUAGE", Targets::Names),
    (&["parameter"], "PARAMETER", Targets::Parameters),
    (&["schema"], "SCHEMA", Targets::Names),
    (&["tablespace"], "TABLESPACE", Targets::Names),
    (&["type"], "TYPE", Targets::AnyNames),
];

/// The kind a `privilege_target` without a kind word names, as gram.y gives it.
const TABLE: &str = "TABLE";

/// `defacl_privilege_target` spellings with the kind each gives; `ROUTINES` is `OBJECT_FUNCTION`.
const DEFAULT_KINDS: [(&[&str], &str); 7] = [
    (&["tables"], "TABLES"),
    (&["functions"], FUNCTIONS),
    (&["routines"], FUNCTIONS),
    (&["sequences"], "SEQUENCES"),
    (&["types"], "TYPES"),
    (&["schemas"], "SCHEMAS"),
    (&["large", "objects"], "LARGE OBJECTS"),
];

/// The default-privilege kind of functions and routines.
const FUNCTIONS: &str = "FUNCTIONS";

/// The reserved words `privilege` takes, as their own names.
const KEYWORD_PRIVILEGES: [&str; 3] = ["select", "references", "create"];

/// The privilege `ALTER SYSTEM` names.
const ALTER_SYSTEM: &str = "alter system";

/// The `DefACLOption` `FOR ROLE` and `FOR USER` both give.
const FOR_ROLE: &str = "FOR ROLE";

/// The `DefACLOption` `IN SCHEMA` gives.
const IN_SCHEMA: &str = "IN SCHEMA";

/// `privileges`: the privileges, and whether they are `ALL`, which gram.y keeps as no privileges.
struct Privileges {
    list: Vec<Privilege>,
    all: bool,
}

/// What follows the privileges of a `GRANT` or `REVOKE`: an object target, or roles.
struct Target {
    kind: Option<String>,
    objects: Vec<Expression>,
}

impl Parser<'_> {
    /// `GrantStmt` or `GrantRoleStmt`; `objects_only` refuses a role grant, as `schema_stmt` does.
    pub(crate) fn grant_statement(&mut self, objects_only: bool) -> Parsing<Expression> {
        self.expect_kw("grant")?;
        let privileges: Privileges = self.privileges()?;
        let on: bool = self.kw("on") || privileges.all || objects_only;
        let target: Target = match on {
            true => self.privilege_target(false)?,
            false => Target {
                kind: None,
                objects: Vec::new(),
            },
        };
        self.expect_kw("to")?;
        let principals: Vec<GrantPrincipal> = match on {
            true => self.grantees()?,
            false => self.role_principals()?,
        };
        let mut grant_option: bool = false;
        let mut options: Vec<Expression> = Vec::new();
        if self.kw("with") {
            self.at += 1;
            if on {
                self.expect_kw("grant")?;
                self.expect_kw("option")?;
                grant_option = true;
            } else {
                options = self.comma_list(|parser| parser.grant_role_option())?;
            }
        }
        let granted_by: Option<Expression> = self.granted_by()?;
        Ok(Expression::Grant(Box::new(Grant {
            privileges: privileges.list,
            kind: target.kind,
            securable: Identifier::new(""),
            function_params: Vec::new(),
            principals,
            grant_option,
            as_principal: None,
            current_grants: None,
            objects: target.objects,
            granted_by,
            options,
        })))
    }

    /// `RevokeStmt` or `RevokeRoleStmt`.
    pub(crate) fn revoke_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("revoke")?;
        let grant_option: bool = self.kw("grant") && self.kw_at(1, "option");
        let role_option: bool = !grant_option
            && self.peek().is_some_and(is_col_id)
            && self.kw_at(1, "option")
            && self.kw_at(2, "for");
        let mut options: Vec<Expression> = Vec::new();
        if grant_option {
            self.at += 2;
            self.expect_kw("for")?;
        } else if role_option {
            let name: Identifier = self.name(is_col_id)?;
            self.at += 2;
            options.push(tuple(vec![name_from(name), boolean(false)]));
        }
        let privileges: Privileges = self.privileges()?;
        let on: bool = !role_option && (self.kw("on") || privileges.all || grant_option);
        let target: Target = match on {
            true => self.privilege_target(false)?,
            false => Target {
                kind: None,
                objects: Vec::new(),
            },
        };
        self.expect_kw("from")?;
        let principals: Vec<GrantPrincipal> = match on {
            true => self.grantees()?,
            false => self.role_principals()?,
        };
        let granted_by: Option<Expression> = self.granted_by()?;
        let cascade: bool = self.eat_kw("cascade");
        if !cascade {
            self.eat_kw("restrict");
        }
        Ok(revoke(
            (privileges.list, target),
            principals,
            (grant_option, cascade),
            (granted_by, options),
        ))
    }

    /// `AlterDefaultPrivilegesStmt`: `FOR ROLE` and `IN SCHEMA` options, then a `GRANT` or `REVOKE` on a kind of object.
    pub(crate) fn alter_default_privileges(&mut self) -> Parsing<Expression> {
        self.at += 3;
        let mut options: Vec<Expression> = Vec::new();
        loop {
            if self.kw("in") {
                self.at += 1;
                self.expect_kw("schema")?;
                let schemas: Vec<Expression> =
                    self.comma_list(|parser| Ok(name_from(parser.name(is_col_id)?)))?;
                options.push(clause(IN_SCHEMA, vec![tuple(schemas)]));
                continue;
            }
            if self.kw("for") && (self.kw_at(1, "role") || self.kw_at(1, "user")) {
                self.at += 2;
                options.push(clause(FOR_ROLE, vec![self.role_list()?]));
                continue;
            }
            break;
        }
        let action: Expression = match self.kw("grant") {
            true => self.default_grant()?,
            false => self.default_revoke()?,
        };
        Ok(Expression::AlterDefaultPrivileges(Box::new(
            AlterDefaultPrivileges { options, action },
        )))
    }

    /// `DefACLAction`'s `GRANT`, which takes no `GRANTED BY`.
    fn default_grant(&mut self) -> Parsing<Expression> {
        self.expect_kw("grant")?;
        let privileges: Privileges = self.privileges()?;
        let target: Target = self.privilege_target(true)?;
        self.expect_kw("to")?;
        let principals: Vec<GrantPrincipal> = self.grantees()?;
        let grant_option: bool = self.kw("with");
        if grant_option {
            self.at += 1;
            self.expect_kw("grant")?;
            self.expect_kw("option")?;
        }
        Ok(Expression::Grant(Box::new(Grant {
            privileges: privileges.list,
            kind: target.kind,
            securable: Identifier::new(""),
            function_params: Vec::new(),
            principals,
            grant_option,
            as_principal: None,
            current_grants: None,
            objects: Vec::new(),
            granted_by: None,
            options: Vec::new(),
        })))
    }

    /// `DefACLAction`'s `REVOKE [GRANT OPTION FOR]`, which takes no `GRANTED BY`.
    fn default_revoke(&mut self) -> Parsing<Expression> {
        self.expect_kw("revoke")?;
        let grant_option: bool = self.kw("grant") && self.kw_at(1, "option");
        if grant_option {
            self.at += 2;
            self.expect_kw("for")?;
        }
        let privileges: Privileges = self.privileges()?;
        let target: Target = self.privilege_target(true)?;
        self.expect_kw("from")?;
        let principals: Vec<GrantPrincipal> = self.grantees()?;
        let cascade: bool = self.eat_kw("cascade");
        if !cascade {
            self.eat_kw("restrict");
        }
        Ok(revoke(
            (privileges.list, target),
            principals,
            (grant_option, cascade),
            (None, Vec::new()),
        ))
    }

    /// `privileges`: `ALL [PRIVILEGES] [( columns )]` or a `privilege_list`.
    fn privileges(&mut self) -> Parsing<Privileges> {
        if !self.eat_kw("all") {
            return Ok(Privileges {
                list: self.comma_list(|parser| parser.privilege())?,
                all: false,
            });
        }
        self.eat_kw("privileges");
        let list: Vec<Privilege> = match self.is_char('(') {
            true => vec![Privilege {
                name: String::new(),
                columns: self.privilege_columns()?,
            }],
            false => Vec::new(),
        };
        Ok(Privileges { list, all: true })
    }

    /// One `privilege`: `SELECT`, `REFERENCES`, `CREATE`, `ALTER SYSTEM` or a `ColId`, with its columns.
    fn privilege(&mut self) -> Parsing<Privilege> {
        if self.kw("alter") && self.kw_at(1, "system") {
            self.at += 2;
            return Ok(Privilege {
                name: ALTER_SYSTEM.to_owned(),
                columns: Vec::new(),
            });
        }
        let name: String = match KEYWORD_PRIVILEGES.iter().find(|word| self.kw(word)) {
            Some(word) => {
                self.at += 1;
                (*word).to_owned()
            }
            None => self.name(is_col_id)?.name,
        };
        let columns: Vec<String> = match self.is_char('(') {
            true => self.privilege_columns()?,
            false => Vec::new(),
        };
        Ok(Privilege { name, columns })
    }

    /// `'(' columnList ')'`: the column names.
    fn privilege_columns(&mut self) -> Parsing<Vec<String>> {
        self.parenthesized(|parser| Ok(parser.name(is_col_id)?.name))
    }

    /// `ON privilege_target`, or `ON defacl_privilege_target` when `default` is set.
    fn privilege_target(&mut self, default: bool) -> Parsing<Target> {
        self.expect_kw("on")?;
        if default {
            let Some((words, kind)) = DEFAULT_KINDS.iter().find(|(words, _)| self.words_at(words))
            else {
                return Err(self.error());
            };
            self.at += words.len();
            return Ok(Target {
                kind: Some((*kind).to_owned()),
                objects: Vec::new(),
            });
        }
        let found = TARGET_KINDS
            .iter()
            .find(|(words, _, _)| self.words_at(words) && !self.ends_target(words.len()));
        let (kind, targets): (&str, Targets) = match found {
            Some((words, kind, targets)) => {
                self.at += words.len();
                (kind, *targets)
            }
            None => (TABLE, Targets::Relations),
        };
        let objects: Vec<Expression> = self.comma_list(|parser| parser.target_object(targets))?;
        Ok(Target {
            kind: Some(kind.to_owned()),
            objects,
        })
    }

    /// Whether a kind word is instead the first relation name, as in `ON sequence TO r` or `ON sequence.t`.
    fn ends_target(&self, words: usize) -> bool {
        self.kw_at(words, "to")
            || self.kw_at(words, "from")
            || self.char_at(words, ',')
            || self.char_at(words, '.')
    }

    /// One object of a `privilege_target`.
    fn target_object(&mut self, targets: Targets) -> Parsing<Expression> {
        match targets {
            Targets::Relations => Ok(Expression::Table(Box::new(table(self.table_name()?)))),
            Targets::Names => Ok(name_from(self.name(is_col_id)?)),
            Targets::AnyNames => Ok(name_value(&self.qualified_name()?)),
            Targets::Routines => self.function_with_argtypes(),
            Targets::Numbers => self.numeric_only(),
            Targets::Parameters => {
                let mut parts: Vec<Identifier> = vec![self.name(is_col_id)?];
                while self.eat_char('.') {
                    parts.push(self.name(is_col_id)?);
                }
                Ok(name_value(&parts))
            }
        }
    }

    /// `grantee_list`: `[GROUP] RoleSpec, ...`; `GROUP` changes nothing.
    fn grantees(&mut self) -> Parsing<Vec<GrantPrincipal>> {
        self.comma_list(|parser| {
            parser.eat_kw("group");
            parser.principal()
        })
    }

    /// `role_list` as principals.
    fn role_principals(&mut self) -> Parsing<Vec<GrantPrincipal>> {
        self.comma_list(|parser| parser.principal())
    }

    /// One `RoleSpec` as a principal: a role's name, or the upper-case keyword of `PUBLIC` and the `CURRENT_*` words.
    fn principal(&mut self) -> Parsing<GrantPrincipal> {
        let name: Identifier = match self.role_spec()? {
            Expression::Identifier(name) => name,
            Expression::Var(word) => Identifier::new(word.this),
            _ => return Err(self.error()),
        };
        Ok(GrantPrincipal {
            name,
            is_role: false,
            is_group: false,
            is_share: false,
            kind: None,
        })
    }

    /// `grant_role_opt`: `ColLabel OPTION | TRUE | FALSE`, as `(name, bool)`.
    fn grant_role_option(&mut self) -> Parsing<Expression> {
        let token = self.expect_token(is_col_label)?;
        let name: Identifier = match token.kind {
            TokenKind::Ident | TokenKind::QuotedIdent => identifier(&token),
            _ => Identifier::new(token.value.to_lowercase()),
        };
        let value: bool = match self.keyword_at(0) {
            Some("option" | "true") => true,
            Some("false") => false,
            _ => return Err(self.error()),
        };
        self.at += 1;
        Ok(tuple(vec![name_from(name), boolean(value)]))
    }

    /// `opt_granted_by`: `GRANTED BY RoleSpec`, if written.
    fn granted_by(&mut self) -> Parsing<Option<Expression>> {
        if !self.eat_kw("granted") {
            return Ok(None);
        }
        self.expect_kw("by")?;
        Ok(Some(self.role_spec()?))
    }
}

/// A `Revoke` of `privileges` on `target` from `principals`.
fn revoke(
    (privileges, target): (Vec<Privilege>, Target),
    principals: Vec<GrantPrincipal>,
    (grant_option, cascade): (bool, bool),
    (granted_by, options): (Option<Expression>, Vec<Expression>),
) -> Expression {
    Expression::Revoke(Box::new(Revoke {
        privileges,
        kind: target.kind,
        securable: Identifier::new(""),
        function_params: Vec::new(),
        principals,
        grant_option,
        cascade,
        restrict: false,
        objects: target.objects,
        granted_by,
        options,
    }))
}
