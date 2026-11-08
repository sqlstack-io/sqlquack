//! `PragmaStatement`, `UseStatement` and `ResetStatement` (pragma.gram, use.gram, set.gram).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::statements::models::{Pragma, ResetStatement, Use};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label, is_identifier};

/// The transformer's refusals of the statements here.
const PRAGMA_ONE_VALUE: &str =
    "PRAGMA statement with assignment should contain exactly one parameter";
const PRAGMA_NAMED: &str = "Named parameter requires a column reference on the LHS";
/// `USE` names a database and at most a schema.
const USE_MAX_PARTS: usize = 2;
const USE_PARTS: &str = "Expected \"USE database\" or \"USE database.schema\"";
const RESET_LOCAL: &str = "RESET LOCAL is not implemented.";
const LOCAL_SCOPE: &str = "LOCAL";

/// Whether a pragma argument is a named parameter whose left side is no column reference.
fn misnamed(argument: &Expression) -> bool {
    match argument {
        Expression::Paren(paren) => misnamed(&paren.this),
        Expression::Eq(comparison) => !is_column(&comparison.left),
        _ => false,
    }
}

/// Whether `expression` is a column reference, qualified or not, in parentheses or not.
fn is_column(expression: &Expression) -> bool {
    match expression {
        Expression::Paren(paren) => is_column(&paren.this),
        Expression::Column(_) => true,
        Expression::Dot(dot) => is_column(&dot.this),
        _ => false,
    }
}

impl Parser<'_> {
    /// `PragmaStatement`: `PRAGMA name = value` or `PRAGMA name [(arguments)]`.
    pub(crate) fn pragma_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("PRAGMA")?;
        let name: Identifier = self.name(is_identifier)?;
        if self.eat_char('=') {
            let mut values: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
            if values.len() > 1 {
                self.refuse_later(PRAGMA_ONE_VALUE, start);
            }
            return Ok(pragma(name, Some(values.remove(0)), Vec::new()));
        }
        let args: Vec<Expression> = match self.is_char('(') {
            true => self.parenthesized(|parser| parser.expression())?,
            false => Vec::new(),
        };
        if args.iter().any(misnamed) {
            self.refuse_later(PRAGMA_NAMED, start);
        }
        Ok(pragma(name, None, args))
    }

    /// `UseStatement`: `USE catalog[.schema]`, or a string; more parts are refused.
    pub(crate) fn use_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("USE")?;
        if self.string_at(0) {
            return Ok(use_node(vec![self.col_id_or_string()?]));
        }
        let mut parts: Vec<Identifier> = vec![self.name(is_identifier)?];
        if self.is_char('.') && self.is_at(1, is_col_label) {
            self.at += 1;
            parts.push(self.name(is_col_label)?);
            while self.is_char('.') && self.is_at(1, is_identifier) {
                self.at += 1;
                parts.push(self.name(is_identifier)?);
            }
        }
        if parts.len() > USE_MAX_PARTS {
            self.refuse_later(USE_PARTS, start);
        }
        Ok(use_node(parts))
    }

    /// `ResetStatement`: `RESET VARIABLE name` or `RESET [scope] name`, `VARIABLE` as its scope.
    pub(crate) fn reset_statement(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.expect_kw("RESET")?;
        let (scope, name) = self.setting_target()?;
        if scope.as_deref() == Some(LOCAL_SCOPE) {
            self.refuse_later(RESET_LOCAL, start);
        }
        Ok(Expression::ResetStatement(Box::new(ResetStatement {
            scope,
            name: Some(Expression::Identifier(name)),
            all: false,
        })))
    }
}

/// The `PRAGMA` node: an assignment when it has a value, else a call.
fn pragma(name: Identifier, value: Option<Expression>, args: Vec<Expression>) -> Expression {
    let use_assignment_syntax: bool = value.is_some();
    Expression::Pragma(Box::new(Pragma {
        schema: None,
        name,
        value,
        args,
        use_assignment_syntax,
    }))
}

/// The `USE` node: the dotted name as `this`, and its parts when there are two.
fn use_node(parts: Vec<Identifier>) -> Expression {
    let mut this: Identifier = parts[0].clone();
    if parts.len() > 1 {
        let names: Vec<&str> = parts.iter().map(|part| part.name.as_str()).collect();
        this.name = names.join(".");
    }
    let parts: Vec<Identifier> = match parts.len() {
        1 => Vec::new(),
        _ => parts,
    };
    Expression::Use(Box::new(Use {
        kind: None,
        this,
        roles: Vec::new(),
        parts,
    }))
}
