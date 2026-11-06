//! `DropStmt` and the other `DROP` statements (gram.y), as `DropObject`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, TableRef};
use sqlquack_syntax::expressions::tables::models::DropObject;

use crate::parse::_helpers::commands::alters::name_from;
use crate::parse::_helpers::commands::tables::{clause, name_value};
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id};
use crate::parse::_helpers::tree::nodes::{table, tuple};

/// What a kind drops: names, types, signatures, or one object of a special shape.
#[derive(Clone, Copy, PartialEq)]
enum Objects {
    /// `any_name_list`.
    AnyNames,
    /// `name_list`.
    Names,
    /// `type_name_list`.
    Types,
    /// `function_with_argtypes_list`.
    Routines,
    /// `aggregate_with_argtypes_list`.
    Aggregates,
    /// `operator_with_argtypes_list`.
    Operators,
    /// `name ON any_name`.
    OnTable,
    /// `any_name USING name`.
    Using,
    /// `( Typename AS Typename )`.
    Cast,
    /// `FOR Typename LANGUAGE name`.
    Transform,
    /// `role_list`, without a drop behavior.
    Roles,
    /// `BY role_list`.
    Owned,
    /// `FOR auth_ident SERVER name`, without a drop behavior.
    UserMapping,
    /// `name [[WITH] ( FORCE, ... )]`, without a drop behavior.
    Database,
    /// One `name`, without a drop behavior.
    Tablespace,
    /// One `name`.
    Subscription,
}

/// The kinds `DROP` takes, longest spelling first, with the kind text and what each drops.
const DROP_KINDS: [(&[&str], &str, Objects); 44] = [
    (
        &["materialized", "view"],
        "MATERIALIZED VIEW",
        Objects::AnyNames,
    ),
    (
        &["foreign", "data", "wrapper"],
        "FOREIGN DATA WRAPPER",
        Objects::Names,
    ),
    (&["foreign", "table"], "FOREIGN TABLE", Objects::AnyNames),
    (
        &["text", "search", "parser"],
        "TEXT SEARCH PARSER",
        Objects::AnyNames,
    ),
    (
        &["text", "search", "dictionary"],
        "TEXT SEARCH DICTIONARY",
        Objects::AnyNames,
    ),
    (
        &["text", "search", "template"],
        "TEXT SEARCH TEMPLATE",
        Objects::AnyNames,
    ),
    (
        &["text", "search", "configuration"],
        "TEXT SEARCH CONFIGURATION",
        Objects::AnyNames,
    ),
    (&["access", "method"], "ACCESS METHOD", Objects::Names),
    (&["event", "trigger"], "EVENT TRIGGER", Objects::Names),
    (&["procedural", "language"], "LANGUAGE", Objects::Names),
    (&["operator", "class"], "OPERATOR CLASS", Objects::Using),
    (&["operator", "family"], "OPERATOR FAMILY", Objects::Using),
    (&["owned", "by"], "OWNED", Objects::Owned),
    (&["table"], "TABLE", Objects::AnyNames),
    (&["sequence"], "SEQUENCE", Objects::AnyNames),
    (&["view"], "VIEW", Objects::AnyNames),
    (&["index"], INDEX, Objects::AnyNames),
    (&["collation"], "COLLATION", Objects::AnyNames),
    (&["conversion"], "CONVERSION", Objects::AnyNames),
    (&["statistics"], "STATISTICS", Objects::AnyNames),
    (&["extension"], "EXTENSION", Objects::Names),
    (&["language"], "LANGUAGE", Objects::Names),
    (&["publication"], "PUBLICATION", Objects::Names),
    (&["schema"], "SCHEMA", Objects::Names),
    (&["server"], "SERVER", Objects::Names),
    (&["type"], "TYPE", Objects::Types),
    (&["domain"], "DOMAIN", Objects::Types),
    (&["function"], "FUNCTION", Objects::Routines),
    (&["procedure"], "PROCEDURE", Objects::Routines),
    (&["routine"], "ROUTINE", Objects::Routines),
    (&["aggregate"], "AGGREGATE", Objects::Aggregates),
    (&["operator"], "OPERATOR", Objects::Operators),
    (&["policy"], "POLICY", Objects::OnTable),
    (&["rule"], "RULE", Objects::OnTable),
    (&["trigger"], "TRIGGER", Objects::OnTable),
    (&["cast"], "CAST", Objects::Cast),
    (&["transform"], "TRANSFORM", Objects::Transform),
    (&["role"], ROLE, Objects::Roles),
    (&["group"], ROLE, Objects::Roles),
    (&["user", "mapping"], USER_MAPPING, Objects::UserMapping),
    (&["user"], ROLE, Objects::Roles),
    (&["database"], "DATABASE", Objects::Database),
    (&["tablespace"], "TABLESPACE", Objects::Tablespace),
    (&["subscription"], "SUBSCRIPTION", Objects::Subscription),
];

/// The kind `DROP ROLE`, `USER` and `GROUP` all give, as gram.y makes one `DropRoleStmt` of each.
const ROLE: &str = "ROLE";

/// The kind of `DROP USER MAPPING`, which `DROP USER` must not take for a role named `mapping`.
const USER_MAPPING: &str = "USER MAPPING";

/// The only kind that may be dropped `CONCURRENTLY`.
const INDEX: &str = "INDEX";

/// The only `drop_option`.
const FORCE: &str = "force";

impl Parser<'_> {
    /// The `DROP` kind here, with its spelling's length, or `None` when no kind follows `DROP`.
    fn dropped_kind(&self) -> Option<(usize, &'static str, Objects)> {
        DROP_KINDS
            .iter()
            .find(|(words, kind, _)| {
                self.words_from(1, words)
                    && (*kind != USER_MAPPING || self.kw_at(3, "for") || self.kw_at(3, "if"))
            })
            .map(|(words, kind, objects)| (words.len(), *kind, *objects))
    }

    /// A `DROP` statement: its kind, `IF EXISTS`, its objects and `CASCADE`; `RESTRICT` is the default.
    pub(crate) fn drop_statement(&mut self) -> Parsing<Expression> {
        let found = self.dropped_kind();
        self.expect_kw("drop")?;
        let Some((words, kind, objects)) = found else {
            return Err(self.error());
        };
        self.at += words;
        let mut node = DropObject {
            kind: kind.to_owned(),
            name: table(Vec::new()),
            if_exists: false,
            cascade: false,
            restrict: false,
            force: false,
            objects: Vec::new(),
            concurrently: false,
        };
        if kind == INDEX {
            node.concurrently = self.eat_kw("concurrently");
        }
        // DropOwnedStmt has no IF EXISTS: there `IF` is a role name, as in gram.y.
        node.if_exists = objects != Objects::Owned && self.kw("if") && self.kw_at(1, "exists");
        if node.if_exists {
            self.at += 2;
        }
        let (name, dropped) = self.dropped_objects(objects)?;
        node.name = name;
        node.objects = dropped;
        if objects == Objects::Database {
            node.force = self.drop_options()?;
        }
        if !matches!(
            objects,
            Objects::Roles | Objects::UserMapping | Objects::Database | Objects::Tablespace
        ) {
            node.cascade = self.eat_kw("cascade");
            if !node.cascade {
                self.eat_kw("restrict");
            }
        }
        Ok(Expression::DropObject(Box::new(node)))
    }

    /// The objects: one plain name goes in `name`; anything else goes in order in `objects`.
    fn dropped_objects(&mut self, objects: Objects) -> Parsing<(TableRef, Vec<Expression>)> {
        match objects {
            Objects::AnyNames => {
                let names: Vec<Vec<Identifier>> =
                    self.comma_list(|parser| parser.qualified_name())?;
                match names.len() {
                    1 => Ok((
                        table(names.into_iter().next().unwrap_or_default()),
                        Vec::new(),
                    )),
                    _ => Ok((
                        table(Vec::new()),
                        names.iter().map(|name| name_value(name)).collect(),
                    )),
                }
            }
            Objects::Names => {
                let names: Vec<Identifier> = self.comma_list(|parser| parser.name(is_col_id))?;
                match names.len() {
                    1 => Ok((table(names), Vec::new())),
                    _ => Ok((
                        table(Vec::new()),
                        names.into_iter().map(name_from).collect(),
                    )),
                }
            }
            Objects::Tablespace | Objects::Subscription | Objects::Database => {
                Ok((table(vec![self.name(is_col_id)?]), Vec::new()))
            }
            _ => Ok((table(Vec::new()), self.dropped_list(objects)?)),
        }
    }

    /// The objects of a kind that drops something other than plain names.
    fn dropped_list(&mut self, objects: Objects) -> Parsing<Vec<Expression>> {
        match objects {
            Objects::Types => {
                self.comma_list(|parser| Ok(Expression::DataType(parser.typename()?)))
            }
            Objects::Routines => self.comma_list(|parser| parser.function_with_argtypes()),
            Objects::Aggregates => self.comma_list(|parser| parser.aggregate_with_argtypes()),
            Objects::Operators => self.comma_list(|parser| parser.operator_with_argtypes()),
            Objects::Roles => self.comma_list(|parser| parser.role_spec()),
            Objects::Owned => self.comma_list(|parser| parser.role_spec()),
            Objects::OnTable => {
                let name: Expression = name_from(self.name(is_col_id)?);
                self.expect_kw("on")?;
                let relation: Expression =
                    Expression::Table(Box::new(table(self.qualified_name()?)));
                Ok(vec![tuple(vec![name, clause("ON", vec![relation])])])
            }
            Objects::Using => {
                let name: Expression = name_value(&self.qualified_name()?);
                self.expect_kw("using")?;
                let method: Expression = name_from(self.name(is_col_id)?);
                Ok(vec![tuple(vec![name, clause("USING", vec![method])])])
            }
            Objects::Cast => {
                self.expect_char('(')?;
                let source: Expression = Expression::DataType(self.typename()?);
                self.expect_kw("as")?;
                let target: Expression = Expression::DataType(self.typename()?);
                self.expect_char(')')?;
                Ok(vec![tuple(vec![source, target])])
            }
            Objects::Transform => {
                self.expect_kw("for")?;
                let data_type: Expression = Expression::DataType(self.typename()?);
                self.expect_kw("language")?;
                let language: Expression = name_from(self.name(is_col_id)?);
                Ok(vec![tuple(vec![
                    clause("FOR", vec![data_type]),
                    clause("LANGUAGE", vec![language]),
                ])])
            }
            _ => {
                self.expect_kw("for")?;
                let user: Expression = match self.eat_kw("user") {
                    true => crate::parse::_helpers::commands::routines::var("CURRENT_USER"),
                    false => self.role_spec()?,
                };
                self.expect_kw("server")?;
                let server: Expression = name_from(self.name(is_col_id)?);
                Ok(vec![
                    clause("FOR", vec![user]),
                    clause("SERVER", vec![server]),
                ])
            }
        }
    }

    /// `[[WITH] ( FORCE, ... )]` after `DROP DATABASE name`: whether `FORCE` was given.
    fn drop_options(&mut self) -> Parsing<bool> {
        let with: bool = self.kw("with") && self.char_at(1, '(');
        if with {
            self.at += 1;
        }
        if !self.is_char('(') {
            return Ok(false);
        }
        self.parenthesized(|parser| match parser.kw(FORCE) {
            true => {
                parser.at += 1;
                Ok(())
            }
            false => Err(parser.error()),
        })?;
        Ok(true)
    }
}
