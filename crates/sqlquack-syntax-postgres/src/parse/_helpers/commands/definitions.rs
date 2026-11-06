//! `DefineStmt` and `CreatedbStmt` (gram.y): `CREATE AGGREGATE` and `CREATE DATABASE`, options kept in order.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{
    CreateDatabase, CreateObject, ObjectProperty,
};

use crate::parse::_helpers::commands::routines::var;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, identifier, is_col_id, is_col_label};
use crate::parse::_helpers::tree::nodes::{table, tuple};
use crate::scan::models::TokenKind;

/// The object kind `CREATE AGGREGATE` gives.
const AGGREGATE_KIND: &str = "AGGREGATE";

/// `createdb_opt_name` keywords other than `CONNECTION LIMIT`, which PostgreSQL names `connection_limit`.
const DATABASE_OPTION_KEYWORDS: [&str; 5] =
    ["encoding", "location", "owner", "tablespace", "template"];

/// The option name `CONNECTION LIMIT` gives.
const CONNECTION_LIMIT: &str = "connection_limit";

/// The source of the pre-8.2 form, `old_aggr_definition`, which takes no argument list.
const OLD_STYLE_SOURCE: &str = "OLD STYLE";

impl Parser<'_> {
    /// Whether `CREATE [OR REPLACE] AGGREGATE` comes next.
    pub(crate) fn creates_aggregate(&self) -> bool {
        let skip: usize = match self.kw_at(1, "or") && self.kw_at(2, "replace") {
            true => 3,
            false => 1,
        };
        self.kw("create") && self.kw_at(skip, "aggregate")
    }

    /// `CREATE opt_or_replace AGGREGATE func_name aggr_args definition`, or `old_aggr_definition`.
    pub(crate) fn create_aggregate(&mut self) -> Parsing<Expression> {
        self.expect_kw("create")?;
        let or_replace: bool = self.eat_kw("or");
        if or_replace {
            self.expect_kw("replace")?;
        }
        self.expect_kw("aggregate")?;
        let name: Vec<Identifier> = self.func_name()?;
        let old_style: bool = self.is_char('(')
            && self.peek_at(1).is_some_and(|token| {
                matches!(token.kind, TokenKind::Ident | TokenKind::QuotedIdent)
            })
            && self.char_at(2, '=');
        let (source, operands, properties) = match old_style {
            true => {
                let properties: Vec<ObjectProperty> = self.parenthesized(|parser| {
                    let token = parser.advance()?;
                    if !matches!(token.kind, TokenKind::Ident | TokenKind::QuotedIdent) {
                        parser.at -= 1;
                        return Err(parser.error());
                    }
                    let name: Identifier = crate::parse::_helpers::tree::cursor::identifier(&token);
                    parser.expect_char('=')?;
                    Ok(definition(name, Some(parser.definition_value()?)))
                })?;
                (OLD_STYLE_SOURCE, Vec::new(), properties)
            }
            false => {
                let arguments: Expression = self.aggregate_arguments(name.clone())?;
                let properties: Vec<ObjectProperty> = self.parenthesized(|parser| {
                    let name: Identifier = parser.name(is_col_label)?;
                    let value: Option<Expression> = match parser.eat_char('=') {
                        true => Some(parser.definition_value()?),
                        false => None,
                    };
                    Ok(definition(name, value))
                })?;
                ("", vec![arguments], properties)
            }
        };
        Ok(Expression::CreateObject(Box::new(CreateObject {
            kind: AGGREGATE_KIND.to_owned(),
            modifier: None,
            or_replace,
            or_alter: false,
            if_not_exists: false,
            name: table(name),
            source: source.to_owned(),
            operands,
            properties,
        })))
    }
}

impl Parser<'_> {
    /// `CREATE DATABASE name [WITH] createdb_opt_item...`, each option a `(name, value)` tuple.
    pub(crate) fn create_database(&mut self) -> Parsing<Expression> {
        self.expect_kw("create")?;
        self.expect_kw("database")?;
        let name: Identifier = self.name(is_col_id)?;
        let mut node: CreateDatabase = CreateDatabase::new(String::new());
        node.name = name;
        self.eat_kw("with");
        node.properties = self.database_options()?;
        Ok(Expression::CreateDatabase(Box::new(node)))
    }

    /// `createdb_opt_list`: options, possibly none, each a `(name, value)` tuple.
    pub(crate) fn database_options(&mut self) -> Parsing<Vec<Expression>> {
        let mut options: Vec<Expression> = Vec::new();
        while let Some(option) = self.database_option_name()? {
            self.eat_char('=');
            let value: Expression = match self.peek().map(|token| token.kind) {
                Some(TokenKind::Integer | TokenKind::Numeric | TokenKind::Char('+' | '-')) => {
                    self.numeric_only()?
                }
                _ if self.eat_kw("default") => var("DEFAULT"),
                _ => self.boolean_or_string()?,
            };
            options.push(tuple(vec![Expression::Identifier(option), value]));
        }
        Ok(options)
    }

    /// `createdb_opt_name`, or `None` when none comes next.
    fn database_option_name(&mut self) -> Parsing<Option<Identifier>> {
        let Some(token) = self.peek().cloned() else {
            return Ok(None);
        };
        match token.kind {
            TokenKind::Ident | TokenKind::QuotedIdent => {
                self.at += 1;
                Ok(Some(identifier(&token)))
            }
            TokenKind::Keyword(keyword) if DATABASE_OPTION_KEYWORDS.contains(&keyword.name) => {
                self.at += 1;
                Ok(Some(Identifier::new(keyword.name)))
            }
            TokenKind::Keyword(_) if self.kw("connection") => {
                self.at += 1;
                self.expect_kw("limit")?;
                Ok(Some(Identifier::new(CONNECTION_LIMIT)))
            }
            _ => Ok(None),
        }
    }
}

/// One `def_elem`: its name and, unless written bare, its `def_arg`.
fn definition(name: Identifier, value: Option<Expression>) -> ObjectProperty {
    ObjectProperty {
        kind: String::new(),
        name: Some(name),
        value,
    }
}
