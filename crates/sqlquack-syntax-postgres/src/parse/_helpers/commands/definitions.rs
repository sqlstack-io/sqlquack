//! `DefineStmt` (gram.y): `CREATE [OR REPLACE] AGGREGATE`, its definition kept as properties in order.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::schema_objects::models::{CreateObject, ObjectProperty};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label};
use crate::parse::_helpers::tree::nodes::table;
use crate::scan::models::TokenKind;

/// The object kind `CREATE AGGREGATE` gives.
const AGGREGATE_KIND: &str = "AGGREGATE";

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

/// One `def_elem`: its name and, unless written bare, its `def_arg`.
fn definition(name: Identifier, value: Option<Expression>) -> ObjectProperty {
    ObjectProperty {
        kind: String::new(),
        name: Some(name),
        value,
    }
}
