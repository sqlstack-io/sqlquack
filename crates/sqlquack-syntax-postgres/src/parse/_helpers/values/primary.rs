//! `c_expr` (gram.y): constants, column references, calls and the parenthesized forms.

use sqlquack_syntax::DataType;
use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::{
    ArraySlice, DotAccess, OverlapsExpr, Subscript,
};
use sqlquack_syntax::expressions::typed_functions::models::ArrayConstructor;
use sqlquack_syntax::tokens::models::Span;

use crate::parse::_helpers::tree::cursor::{
    Parser, Parsing, identifier, is_col_id, is_col_label, is_type_function_name,
};
use crate::parse::_helpers::tree::nodes::{
    boolean, case, cast, column, exists, function, literal, null, parameter, paren, star, subquery,
    tuple,
};
use crate::parse::_helpers::values::expressions::Flavor;
use crate::parse::constants::{
    JSON_AGGREGATES, JSON_CALLS, ROW_CONSTRUCTOR, VALUE_FUNCTIONS, XML_CALLS,
};
use crate::parse::constants::{MIN_IMPLICIT_ROW_ITEMS, TYPE_KEYWORDS};
use crate::scan::models::{Token, TokenKind};

/// A column reference from its dotted names: `a`, `t.a`, or a field of `s.t.a` as dot access.
fn column_from(parts: Vec<Identifier>) -> Expression {
    let mut parts = parts.into_iter();
    let first: Identifier = parts.next().unwrap_or_else(Identifier::empty);
    let Some(second) = parts.next() else {
        return column(None, first);
    };
    parts.fold(column(Some(first), second), |this, field| {
        Expression::Dot(Box::new(DotAccess {
            this,
            field,
            inferred_type: None,
        }))
    })
}

fn dot(this: Expression, field: Identifier) -> Expression {
    Expression::Dot(Box::new(DotAccess {
        this,
        field,
        inferred_type: None,
    }))
}

impl Parser<'_> {
    pub(crate) fn c_expr(&mut self, flavor: Flavor) -> Parsing<Expression> {
        let Some(token) = self.peek().cloned() else {
            return Err(self.error());
        };
        match token.kind {
            TokenKind::Integer | TokenKind::Numeric => {
                self.at += 1;
                Ok(literal(Literal::Number(token.value)))
            }
            TokenKind::String | TokenKind::EscapeString | TokenKind::DollarString => {
                self.at += 1;
                Ok(literal(Literal::String(token.value)))
            }
            TokenKind::BitString => {
                self.at += 1;
                Ok(literal(Literal::BitString(token.value)))
            }
            TokenKind::HexString => {
                self.at += 1;
                Ok(literal(Literal::HexString(token.value)))
            }
            TokenKind::Param => {
                let Ok(index) = token.value.parse::<u32>() else {
                    return Err(self.error());
                };
                self.at += 1;
                self.indirection(parameter(index))
            }
            TokenKind::Char('(') => self.parenthesized_primary(flavor),
            TokenKind::Keyword(keyword) => self.keyword_expression(keyword.name, flavor),
            TokenKind::Ident | TokenKind::QuotedIdent => self.name_expression(),
            _ => Err(self.error()),
        }
    }

    /// A `Sconst`, as a string literal.
    pub(crate) fn sconst(&mut self) -> Parsing<Expression> {
        match self.sconst_at(0) {
            true => Ok(literal(Literal::String(self.advance()?.value))),
            false => Err(self.error()),
        }
    }

    /// A keyword-led primary, such as a constant, `CASE`, `CAST`, `ARRAY` or a special call.
    fn keyword_expression(&mut self, name: &'static str, flavor: Flavor) -> Parsing<Expression> {
        let start: usize = self.offset();
        let call: bool = self.char_at(1, '(');
        match name {
            "true" => {
                self.at += 1;
                Ok(boolean(true))
            }
            "false" => {
                self.at += 1;
                Ok(boolean(false))
            }
            "null" => {
                self.at += 1;
                Ok(null())
            }
            "case" => self.case_expression(),
            "cast" => {
                self.at += 1;
                self.expect_char('(')?;
                let this: Expression = self.a_expr()?;
                self.expect_kw("as")?;
                let to: DataType = self.typename()?;
                self.expect_char(')')?;
                Ok(cast(this, to, false))
            }
            "array" => {
                self.at += 1;
                if self.is_char('(') {
                    let query: Expression = self.select_with_parens()?;
                    return Ok(Expression::Function(Box::new(function(
                        vec![Identifier::new("ARRAY")],
                        vec![query],
                    ))));
                }
                self.array_brackets(false)
            }
            "exists" if call && self.starts_query_at(1) => {
                self.at += 1;
                Ok(exists(self.select_with_parens()?))
            }
            "row" if call => {
                self.at += 1;
                self.expect_char('(')?;
                let items: Vec<Expression> = match self.is_char(')') {
                    true => Vec::new(),
                    false => self.expr_list()?,
                };
                self.expect_char(')')?;
                self.row_or_overlaps(items, true, flavor)
            }
            "grouping" if call => {
                self.at += 1;
                let args: Vec<Expression> = self.parenthesized(|parser| parser.a_expr())?;
                Ok(Expression::Function(Box::new(function(
                    vec![Identifier::new("GROUPING")],
                    args,
                ))))
            }
            "interval" if self.sconst_at(1) => {
                self.at += 1;
                let value: Expression = self.sconst()?;
                let to: DataType = self.interval_fields()?;
                Ok(cast(value, to, false))
            }
            "interval"
                if call
                    && self
                        .peek_at(2)
                        .is_some_and(|token| token.kind == TokenKind::Integer)
                    && self.char_at(3, ')')
                    && self.sconst_at(4) =>
            {
                let to: DataType = self.simple_typename()?;
                let value: Expression = self.sconst()?;
                Ok(cast(value, to, false))
            }
            "collation" if self.kw_at(1, "for") => {
                self.at += 2;
                self.expect_char('(')?;
                let this: Expression = self.a_expr()?;
                self.expect_char(')')?;
                Ok(Expression::Function(Box::new(function(
                    vec![Identifier::new("pg_collation_for")],
                    vec![this],
                ))))
            }
            _ if VALUE_FUNCTIONS
                .iter()
                .any(|(value, precise)| *value == name && (*precise || !call)) =>
            {
                self.value_function(name)
            }
            _ if call && JSON_CALLS.contains(&name) => {
                self.at += 1;
                self.json_call(name)?;
                if JSON_AGGREGATES.contains(&name) {
                    self.filter_and_window()?;
                }
                self.gap(name, start);
                Ok(null())
            }
            _ if call && XML_CALLS.contains(&name) => {
                self.at += 1;
                self.xml_call(name)?;
                self.gap(name, start);
                Ok(null())
            }
            _ if call && self.is_special_call(name) => self.special_call(name),
            _ => {
                if TYPE_KEYWORDS.contains(&name) {
                    let typed = self.attempt(|parser| {
                        let to: DataType = parser.const_typename()?;
                        let value: Expression = parser.sconst()?;
                        Ok(cast(value, to, false))
                    });
                    if let Ok(typed) = typed {
                        return Ok(typed);
                    }
                }
                self.name_expression()
            }
        }
    }

    /// `CURRENT_DATE`, `CURRENT_TIMESTAMP [(p)]` and the other SQL-value functions.
    fn value_function(&mut self, name: &'static str) -> Parsing<Expression> {
        self.at += 1;
        let precise: bool = VALUE_FUNCTIONS
            .iter()
            .any(|(value, precision)| *value == name && *precision);
        let mut args: Vec<Expression> = Vec::new();
        if precise && self.eat_char('(') {
            let token: Token = self.advance()?;
            if token.kind != TokenKind::Integer {
                self.at -= 1;
                return Err(self.error());
            }
            args.push(literal(Literal::Number(token.value)));
            self.expect_char(')')?;
        }
        let mut call = function(vec![Identifier::new(name)], args);
        call.no_parens = call.args.is_empty();
        Ok(Expression::Function(Box::new(call)))
    }

    /// A name-led primary: a call, a typed literal (`func_name Sconst`), or a column reference.
    pub(crate) fn name_expression(&mut self) -> Parsing<Expression> {
        let start: usize = self.at;
        let first: Token = match self.peek() {
            Some(token) if is_col_id(token) || is_type_function_name(token) => token.clone(),
            _ => return Err(self.error()),
        };
        self.at += 1;
        let mut parts: Vec<Identifier> = vec![identifier(&first)];
        while self.is_char('.') && self.peek_at(1).is_some_and(is_col_label) {
            self.at += 1;
            parts.push(self.name(is_col_label)?);
        }
        let function_name: bool = match parts.len() {
            1 => is_type_function_name(&first),
            _ => is_col_id(&first),
        };
        if function_name && self.is_char('(') {
            let literal_follows: bool = self
                .closing(self.at)
                .is_some_and(|close| self.sconst_at(close + 1 - self.at));
            if literal_follows && is_type_function_name(&first) {
                self.at = start;
                let to: DataType = self.generic_type()?;
                let value: Expression = self.sconst()?;
                return Ok(cast(value, to, false));
            }
            return self.function_call(parts, false);
        }
        if function_name && self.sconst_at(0) && is_type_function_name(&first) {
            self.at = start;
            let to: DataType = self.generic_type()?;
            let value: Expression = self.sconst()?;
            return Ok(cast(value, to, false));
        }
        if !is_col_id(&first) {
            return Err(self.error());
        }
        if self.is_char('.') && self.char_at(1, '*') {
            self.at += 2;
            let qualifier: String = parts
                .iter()
                .map(|part| part.name.as_str())
                .collect::<Vec<&str>>()
                .join(".");
            let mut table: Identifier = Identifier::new(qualifier);
            table.quoted = parts.len() == 1 && parts[0].quoted;
            table.span = match (parts.first().and_then(|part| part.span), parts.last()) {
                (Some(first), Some(last)) => last.span.map(|last| Span {
                    start: first.start,
                    ..last
                }),
                _ => None,
            };
            return self.after_star(star(Some(table)));
        }
        self.indirection(column_from(parts))
    }

    /// `*` ends an indirection: anything after it is improper.
    fn after_star(&self, this: Expression) -> Parsing<Expression> {
        match self.is_char('.') || self.is_char('[') {
            true => Err(self.error_with("improper use of \"*\"", self.offset())),
            false => Ok(this),
        }
    }

    /// `opt_indirection`: `.field`, `.*`, `[i]` and `[i:j]` after a primary.
    pub(crate) fn indirection(&mut self, mut this: Expression) -> Parsing<Expression> {
        loop {
            if self.is_char('.') {
                if self.char_at(1, '*') {
                    self.at += 2;
                    return self.after_star(dot(this, Identifier::new("*")));
                }
                self.at += 1;
                let field: Identifier = self.name(is_col_label)?;
                this = dot(this, field);
            } else if self.eat_char('[') {
                let start: Option<Expression> = match self.is_char(':') {
                    true => None,
                    false => Some(self.a_expr()?),
                };
                if self.eat_char(':') {
                    let end: Option<Expression> = match self.is_char(']') {
                        true => None,
                        false => Some(self.a_expr()?),
                    };
                    self.expect_char(']')?;
                    this = Expression::ArraySlice(Box::new(ArraySlice { this, start, end }));
                } else {
                    self.expect_char(']')?;
                    let Some(index) = start else {
                        return Err(self.error());
                    };
                    this = Expression::Subscript(Box::new(Subscript { this, index }));
                }
            } else {
                return Ok(this);
            }
        }
    }

    /// `(` at the cursor: a subquery, a parenthesized expression or an implicit row.
    fn parenthesized_primary(&mut self, flavor: Flavor) -> Parsing<Expression> {
        if !self.starts_query_at(1) {
            return self.parenthesized_expression(flavor);
        }
        let reached: usize = match self.attempt(|parser| parser.select_with_parens()) {
            Ok(query) => {
                return self.indirection(subquery(query, None, false));
            }
            Err((error, _)) => error.position,
        };
        match self.attempt(|parser| parser.parenthesized_expression(flavor)) {
            Ok(expression) => Ok(expression),
            Err((error, _)) if error.position >= reached => Err(error),
            Err(_) => Err(self.error_at_position(reached)),
        }
    }

    /// `( a_expr ) opt_indirection`, or the implicit row `( a_expr, ... )`.
    fn parenthesized_expression(&mut self, flavor: Flavor) -> Parsing<Expression> {
        self.expect_char('(')?;
        let first: Expression = self.a_expr()?;
        if self.eat_char(',') {
            let mut items: Vec<Expression> = vec![first];
            items.extend(self.expr_list()?);
            self.expect_char(')')?;
            return self.row_or_overlaps(items, false, flavor);
        }
        self.expect_char(')')?;
        self.indirection(paren(first))
    }

    /// A row, or `row OVERLAPS row`, whose sides hold one or two items.
    fn row_or_overlaps(
        &mut self,
        items: Vec<Expression>,
        explicit: bool,
        flavor: Flavor,
    ) -> Parsing<Expression> {
        if flavor == Flavor::A && self.kw("overlaps") {
            let start: usize = self.offset();
            self.at += 1;
            let right: Vec<Expression> = self.overlaps_row()?;
            let (left_start, left_end) = self.overlaps_side(items, "left", start)?;
            let (right_start, right_end) = self.overlaps_side(right, "right", start)?;
            return Ok(Expression::Overlaps(Box::new(OverlapsExpr {
                this: None,
                expression: None,
                left_start: Some(left_start),
                left_end: Some(left_end),
                right_start: Some(right_start),
                right_end: Some(right_end),
            })));
        }
        Ok(match explicit {
            true => Expression::Function(Box::new(function(
                vec![Identifier::new(ROW_CONSTRUCTOR)],
                items,
            ))),
            false => tuple(items),
        })
    }

    /// The `row` after `OVERLAPS`: `ROW ( ... )` or `( a, b, ... )`.
    fn overlaps_row(&mut self) -> Parsing<Vec<Expression>> {
        if self.kw("row") && self.char_at(1, '(') {
            self.at += 2;
            let items: Vec<Expression> = match self.is_char(')') {
                true => Vec::new(),
                false => self.expr_list()?,
            };
            self.expect_char(')')?;
            return Ok(items);
        }
        self.expect_char('(')?;
        let items: Vec<Expression> = self.expr_list()?;
        if items.len() < MIN_IMPLICIT_ROW_ITEMS {
            return Err(self.error());
        }
        self.expect_char(')')?;
        Ok(items)
    }

    /// One side of `OVERLAPS`: two items, or one standing for both ends.
    fn overlaps_side(
        &self,
        items: Vec<Expression>,
        side: &str,
        position: usize,
    ) -> Parsing<(Expression, Expression)> {
        let mut items = items.into_iter();
        match (items.next(), items.next(), items.next()) {
            (Some(only), None, None) => Ok((only.clone(), only)),
            (Some(first), Some(second), None) => Ok((first, second)),
            _ => Err(self.error_with(
                &format!("wrong number of parameters on {side} side of OVERLAPS expression"),
                position,
            )),
        }
    }

    /// `CASE [a_expr] WHEN a_expr THEN a_expr ... [ELSE a_expr] END`.
    fn case_expression(&mut self) -> Parsing<Expression> {
        self.at += 1;
        let operand: Option<Expression> = match self.kw("when") {
            true => None,
            false => Some(self.a_expr()?),
        };
        let mut whens: Vec<(Expression, Expression)> = Vec::new();
        while self.eat_kw("when") {
            let condition: Expression = self.a_expr()?;
            self.expect_kw("then")?;
            whens.push((condition, self.a_expr()?));
        }
        if whens.is_empty() {
            return Err(self.error());
        }
        let otherwise: Option<Expression> = match self.eat_kw("else") {
            true => Some(self.a_expr()?),
            false => None,
        };
        self.expect_kw("end")?;
        Ok(case(operand, whens, otherwise))
    }

    /// `array_expr`: `[ expr_list ]`, `[ array_expr, ... ]` or `[ ]`.
    fn array_brackets(&mut self, nested: bool) -> Parsing<Expression> {
        self.expect_char('[')?;
        let expressions: Vec<Expression> = if self.is_char(']') {
            Vec::new()
        } else if self.is_char('[') {
            self.comma_list(|parser| parser.array_brackets(true))?
        } else {
            self.expr_list()?
        };
        self.expect_char(']')?;
        Ok(Expression::ArrayFunc(Box::new(ArrayConstructor {
            expressions,
            bracket_notation: nested,
            use_list_keyword: false,
            inferred_type: None,
        })))
    }
}
