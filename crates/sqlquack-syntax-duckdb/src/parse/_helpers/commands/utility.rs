//! ATTACH, DETACH, SET, transactions and COMMENT ON (attach, detach, set, transaction, comment).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::statements::models::{
    Attach, AttachOption, Comment, Commit, Detach, Rollback, SetItem, SetStatement, Transaction,
};

use crate::parse::_helpers::commands::options::CopyOption;
use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};
use crate::parse::_helpers::tree::nodes::{alias, column_from, literal, null, table, var};
use crate::parse::constants::REPLACE_AND_IF_NOT_EXISTS;

/// The transformer's refusals of the statements here.
const SET_LOCAL: &str = "SET LOCAL is not implemented.";
const SET_ONE_VALUE: &str = "SET can only contain a single value";
const TOO_MANY_PARTS: &str =
    "Too many qualifications found - expected [catalog.schema.name] or [schema.name]";
/// `CommentOnType`, longest first, and the kinds the transformer does not implement.
const COMMENT_KINDS: [&[&str]; 11] = [
    &["MACRO", "TABLE"],
    &["TABLE"],
    &["SEQUENCE"],
    &["FUNCTION"],
    &["MACRO"],
    &["VIEW"],
    &["DATABASE"],
    &["INDEX"],
    &["SCHEMA"],
    &["TYPE"],
    &["COLUMN"],
];
const COMMENT_COLUMN: &str = "COLUMN";
const COMMENT_UNIMPLEMENTED: [(&str, &str); 2] = [("DATABASE", "databases"), ("SCHEMA", "schemas")];
/// A commented object's name has at most three parts; a column's at most four.
const MAX_OBJECT_PARTS: usize = 3;
const MAX_COLUMN_PARTS: usize = 4;
/// `SettingScope`.
const SETTING_SCOPES: [&str; 3] = ["LOCAL", "SESSION", "GLOBAL"];

impl Parser<'_> {
    /// `AttachStatement`: `ATTACH [OR REPLACE] [IF NOT EXISTS] [DATABASE] path [AS name] [(options)]`.
    pub(crate) fn attach_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("ATTACH")?;
        let or_replace: bool = self.eat_kws(&["OR", "REPLACE"]);
        let exists: bool = self.eat_kws(&["IF", "NOT", "EXISTS"]);
        if or_replace && exists {
            self.refuse_later(REPLACE_AND_IF_NOT_EXISTS, start);
        }
        self.eat_kw("DATABASE");
        let path: Expression = self.database_path()?;
        let this: Expression = match self.eat_kw("AS") {
            true => alias(path, self.name(is_identifier)?, true),
            false => path,
        };
        let options: Vec<CopyOption> = match self.is_char('(') {
            true => self.generic_copy_options()?,
            false => Vec::new(),
        };
        if or_replace {
            self.gap("attach_or_replace", start);
        }
        let expressions: Vec<Expression> = options
            .into_iter()
            .map(|option| {
                Expression::AttachOption(Box::new(AttachOption {
                    this: Box::new(Expression::Identifier(option.name)),
                    expression: option.expression.map(Box::new),
                }))
            })
            .collect();
        Ok(Expression::Attach(Box::new(Attach {
            this: Box::new(this),
            exists,
            expressions,
        })))
    }

    /// `DatabasePath`: an expression, or a bare name when what follows are options, not arguments.
    fn database_path(&mut self) -> Parsing<Expression> {
        let path = self.attempt(|parser| {
            let path: Expression = parser.expression()?;
            match parser.terminal_at(0) || parser.kw("AS") || parser.is_char('(') {
                true => Ok(path),
                false => Err(parser.error()),
            }
        });
        match path {
            Ok(path) => Ok(path),
            Err((error, _)) if !self.is(is_identifier) => Err(error),
            Err((error, _)) => {
                let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
                while self.is_char('.') && self.is_at(1, is_col_label) {
                    self.at += 1;
                    parts.push(self.name(is_col_label)?);
                }
                match self.is_char('(') {
                    true => Ok(column_from(parts)),
                    false => Err(error),
                }
            }
        }
    }

    /// `DetachStatement`: `DETACH [DATABASE] [IF EXISTS] name`.
    pub(crate) fn detach_statement(&mut self) -> Parsing<Expression> {
        self.expect_kw("DETACH")?;
        self.eat_kw("DATABASE");
        let exists: bool = self.eat_kws(&["IF", "EXISTS"]);
        let name: Identifier = self.name(is_identifier)?;
        Ok(Expression::Detach(Box::new(Detach {
            this: Box::new(Expression::Identifier(name)),
            exists,
        })))
    }

    /// `SetStatement`: `SET SCHEMA 'name'`, a `StandardAssignment`, or `SET TIME ZONE value`.
    pub(crate) fn set_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("SET")?;
        if self.kw("SCHEMA") && self.string_at(1) {
            self.at += 1;
            let value: Expression = self.string_value()?;
            return Ok(set_item("SCHEMA", value));
        }
        if let Ok(item) = self.attempt(|parser| parser.standard_assignment(start)) {
            return Ok(Expression::SetStatement(Box::new(SetStatement {
                items: vec![item],
            })));
        }
        self.expect_kw("TIME")?;
        self.expect_kw("ZONE")?;
        let value: Expression = self.zone_value(start)?;
        Ok(set_item("TIME ZONE", value))
    }

    /// `SetVariableOrSetting`: `VARIABLE name` or `[scope] name`; a scope word is always a scope.
    pub(crate) fn setting_target(&mut self) -> Parsing<(Option<String>, Identifier)> {
        let variable: bool = self.kw("VARIABLE") && self.is_at(1, is_identifier);
        let kind: Option<String> = match variable {
            true => {
                self.at += 1;
                Some("VARIABLE".to_owned())
            }
            false => {
                let scope: Option<String> = self
                    .word_at(0)
                    .filter(|word| SETTING_SCOPES.contains(&word.as_str()));
                self.at += usize::from(scope.is_some());
                scope
            }
        };
        Ok((kind, self.name(is_identifier)?))
    }

    /// `StandardAssignment`: `[VARIABLE name | [scope] name] {= | TO} value`.
    fn standard_assignment(&mut self, start: usize) -> Parsing<SetItem> {
        let (kind, name) = self.setting_target()?;
        if !(self.eat_char('=') || self.eat_kw("TO")) {
            return Err(self.error());
        }
        let mut values: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
        if kind.as_deref() == Some("LOCAL") {
            self.refuse_later(SET_LOCAL, start);
        }
        if values.len() > 1 {
            self.refuse_later(SET_ONE_VALUE, start);
        }
        Ok(SetItem {
            name: Expression::Identifier(name),
            value: values.remove(0),
            kind,
            no_equals: false,
        })
    }

    /// `ZoneValue`: `LOCAL`, `DEFAULT`, a string, a name or a number; an interval is a gap.
    fn zone_value(&mut self, start: usize) -> Parsing<Expression> {
        if self.kw("INTERVAL") {
            self.skip_statement_rest()?;
            self.gap("set_time_zone_interval", start);
            return Ok(null());
        }
        if self.kw_in(0, &["LOCAL", "DEFAULT"]) {
            return Ok(var(&self.advance()?.value.to_ascii_uppercase()));
        }
        if self.string_at(0) {
            return self.string_value();
        }
        if self.is(is_identifier) {
            return Ok(Expression::Identifier(self.name(is_identifier)?));
        }
        match self.peek().map(|token| token.kind) {
            Some(crate::scan::models::TokenKind::Number) => self.primary_expression(),
            _ => Err(self.error()),
        }
    }

    /// `TransactionStatement`: a start (with `READ ONLY|WRITE`), a commit or a rollback.
    pub(crate) fn transaction_statement(&mut self) -> Parsing<Expression> {
        let word: String = self.word_at(0).unwrap_or_default();
        self.at += 1;
        if !self.eat_kw("WORK") {
            self.eat_kw("TRANSACTION");
        }
        Ok(match word.as_str() {
            "COMMIT" | "END" => Expression::Commit(Box::new(Commit {
                chain: None,
                this: None,
                durability: None,
            })),
            "ROLLBACK" | "ABORT" => Expression::Rollback(Box::new(Rollback {
                savepoint: None,
                this: None,
            })),
            _ => {
                let modes: Option<Box<Expression>> = if self.eat_kws(&["READ", "ONLY"]) {
                    Some(Box::new(var("READ ONLY")))
                } else if self.eat_kws(&["READ", "WRITE"]) {
                    Some(Box::new(var("READ WRITE")))
                } else {
                    None
                };
                Expression::Transaction(Box::new(Transaction {
                    this: None,
                    modes,
                    mark: Some(Box::new(var(&word))),
                }))
            }
        })
    }

    /// `CommentStatement`: `COMMENT ON kind name IS 'text' | NULL`, with the transformer's checks.
    pub(crate) fn comment_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("COMMENT")?;
        self.expect_kw("ON")?;
        let Some(words) = COMMENT_KINDS.iter().find(|words| self.eat_kws(words)) else {
            return Err(self.error());
        };
        let kind: String = words.join(" ");
        let parts: Vec<Identifier> = match self.string_at(0) {
            true => vec![self.col_id_or_string()?],
            false => {
                let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
                while self.is_char('.') && self.is_at(1, is_col_label) {
                    self.at += 1;
                    parts.push(self.name(is_col_label)?);
                }
                parts
            }
        };
        self.expect_kw("IS")?;
        let expression: Expression = match self.eat_kw("NULL") {
            true => null(),
            false => self.string_value()?,
        };
        let column: bool = kind == COMMENT_COLUMN;
        if let Some((_, plural)) = COMMENT_UNIMPLEMENTED.iter().find(|(name, _)| *name == kind) {
            let message: String = format!("Adding comments to {plural} is not implemented");
            self.refuse_later(&message, start);
        } else if column && parts.len() == 1 {
            let message: String = format!("Invalid column reference: {}", parts[0].name);
            self.refuse_later(&message, start);
        } else if parts.len()
            > if column {
                MAX_COLUMN_PARTS
            } else {
                MAX_OBJECT_PARTS
            }
        {
            self.refuse_later(TOO_MANY_PARTS, start);
        }
        let this: Expression = match column {
            true => column_from(parts),
            false => Expression::Table(Box::new(table(parts))),
        };
        Ok(Expression::Comment(Box::new(Comment {
            this,
            kind,
            expression,
            exists: false,
            materialized: false,
        })))
    }

    /// A string literal's text, as a literal.
    fn string_value(&mut self) -> Parsing<Expression> {
        let text: String = self.expect_string()?;
        Ok(literal(Literal::String(text)))
    }
}

/// `SET name = value` as a one-item statement.
fn set_item(name: &str, value: Expression) -> Expression {
    Expression::SetStatement(Box::new(SetStatement {
        items: vec![SetItem {
            name: Expression::Identifier(Identifier::new(name.to_owned())),
            value,
            kind: None,
            no_equals: false,
        }],
    }))
}
