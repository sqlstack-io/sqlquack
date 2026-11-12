//! `ParserExpressionImpl`'s operators, by precedence climbing over `operators_table`.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::{LikeOp, UnaryOp};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{
    alias, between, binary, call, in_list, lambda, null_test,
};
use crate::parse::constants::{
    PRIORITY_ACCESS, PRIORITY_ADD, PRIORITY_AND, PRIORITY_BETWEEN, PRIORITY_COMPARE,
    PRIORITY_CONCAT, PRIORITY_FINISH_BETWEEN, PRIORITY_IS, PRIORITY_LAMBDA, PRIORITY_MULTIPLY,
    PRIORITY_NEGATE, PRIORITY_NOT, PRIORITY_OR, PRIORITY_TERNARY, PRIORITY_TERNARY_ELSE,
};
use crate::scan::models::TokenKind;

/// What an operator of `operators_table` builds.
#[derive(Clone, Copy)]
enum Form {
    Lambda,
    Ternary,
    Binary(fn(Box<sqlquack_syntax::expressions::operators::models::BinaryOp>) -> Expression),
    Named(&'static str),
    NotDistinct(bool),
    IsNull(bool),
    Between(bool),
    Like(bool, bool),
    In(bool, bool),
    Dot,
    Subscript,
    Cast,
}

/// An operator at the cursor: its priority, how many tokens it spans and what it builds.
type Operator = (u8, usize, Form);

fn unary(kind: fn(Box<UnaryOp>) -> Expression, this: Expression) -> Expression {
    kind(Box::new(UnaryOp::new(this)))
}

fn like(this: Expression, pattern: Expression, ilike: bool, not: bool) -> Expression {
    let mut node: LikeOp = LikeOp::new(this, pattern);
    node.negated = not;
    match ilike {
        true => Expression::ILike(Box::new(node)),
        false => Expression::Like(Box::new(node)),
    }
}

/// The lambda parameters `left` names: one identifier, or a tuple of them.
fn lambda_parameters(left: &Expression) -> Option<Vec<Identifier>> {
    let name = |expression: &Expression| match expression {
        Expression::Column(column) if column.table.is_none() => Some(column.name.clone()),
        _ => None,
    };
    match left {
        Expression::Tuple(tuple) => tuple.expressions.iter().map(name).collect(),
        other => name(other).map(|name| vec![name]),
    }
}

impl Parser<'_> {
    /// `ParserExpression`: an expression with no alias at its top.
    pub(crate) fn expression(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| parser.climb(0))
    }

    /// A layer element: an expression and an alias written with `AS`.
    pub(crate) fn element(&mut self) -> Parsing<Expression> {
        let this: Expression = self.expression()?;
        self.optional_alias(this, false)
    }

    /// `ParserExpressionWithOptionalAlias`: an alias may also be a bare name that is no keyword.
    pub(crate) fn aliased(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let this: Expression = self.expression()?;
        let bare: bool = !matches!(&this, Expression::Column(column)
            if column.table.is_none() && !column.name.quoted && column.name.name.eq_ignore_ascii_case("FROM"));
        let node: Expression = self.optional_alias(this, bare)?;
        Ok(self.spanned(first, node))
    }

    /// `ParserAlias` after `this`, when one follows.
    pub(crate) fn optional_alias(&mut self, this: Expression, bare: bool) -> Parsing<Expression> {
        match self.alias_name(bare)? {
            Some((name, explicit)) => Ok(alias(this, name, explicit)),
            None => Ok(this),
        }
    }

    /// `ParserAlias`: `AS name`, or with `bare` a name that is quoted or no restricted keyword.
    pub(crate) fn alias_name(&mut self, bare: bool) -> Parsing<Option<(Identifier, bool)>> {
        if self.kw("AS") && self.is_at(1, crate::parse::_helpers::tree::cursor::is_identifier) {
            self.at += 1;
            return Ok(Some((self.ident()?, true)));
        }
        let allowed: bool = bare
            && match self.peek() {
                Some(token) if token.kind == TokenKind::QuotedIdentifier => {
                    crate::parse::_helpers::tree::cursor::is_identifier(token)
                }
                Some(token) if token.kind == TokenKind::BareWord => !self.restricted_at(0),
                _ => false,
            };
        match allowed {
            true => Ok(Some((self.ident()?, false))),
            false => Ok(None),
        }
    }

    /// The operator at the cursor, as `tryParseOperator` matches `operators_table` in order.
    fn operator(&self) -> Option<Operator> {
        let token = self.peek()?;
        let operator: Operator = match token.kind {
            TokenKind::Arrow => (PRIORITY_LAMBDA, 1, Form::Lambda),
            TokenKind::QuestionMark => (PRIORITY_TERNARY, 1, Form::Ternary),
            TokenKind::Equals => (PRIORITY_COMPARE, 1, Form::Binary(Expression::Eq)),
            TokenKind::NotEquals => (PRIORITY_COMPARE, 1, Form::Binary(Expression::Neq)),
            TokenKind::Spaceship => (PRIORITY_COMPARE, 1, Form::NotDistinct(true)),
            TokenKind::LessOrEquals => (PRIORITY_COMPARE, 1, Form::Binary(Expression::Lte)),
            TokenKind::GreaterOrEquals => (PRIORITY_COMPARE, 1, Form::Binary(Expression::Gte)),
            TokenKind::Less => (PRIORITY_COMPARE, 1, Form::Binary(Expression::Lt)),
            TokenKind::Greater => (PRIORITY_COMPARE, 1, Form::Binary(Expression::Gt)),
            TokenKind::Concatenation => (PRIORITY_CONCAT, 1, Form::Binary(Expression::Concat)),
            TokenKind::Plus => (PRIORITY_ADD, 1, Form::Binary(Expression::Add)),
            TokenKind::Minus => (PRIORITY_ADD, 1, Form::Binary(Expression::Sub)),
            TokenKind::Asterisk => (PRIORITY_MULTIPLY, 1, Form::Binary(Expression::Mul)),
            TokenKind::Slash => (PRIORITY_MULTIPLY, 1, Form::Binary(Expression::Div)),
            TokenKind::Percent => (PRIORITY_MULTIPLY, 1, Form::Binary(Expression::Mod)),
            TokenKind::Dot => (PRIORITY_ACCESS, 1, Form::Dot),
            TokenKind::OpeningSquareBracket => (PRIORITY_ACCESS, 1, Form::Subscript),
            TokenKind::DoubleColon => (PRIORITY_ACCESS, 1, Form::Cast),
            TokenKind::BareWord => return self.word_operator(),
            _ => return None,
        };
        Some(operator)
    }

    /// The keyword operators of `operators_table`.
    fn word_operator(&self) -> Option<Operator> {
        let words: [Option<String>; 4] = [
            self.word_at(0),
            self.word_at(1),
            self.word_at(2),
            self.word_at(3),
        ];
        let [first, second, third, fourth] = words.each_ref().map(|word| word.as_deref());
        let operator: Operator = match (first?, second, third, fourth) {
            ("OR", ..) => (PRIORITY_OR, 1, Form::Binary(Expression::Or)),
            ("AND", ..) => (PRIORITY_AND, 1, Form::Binary(Expression::And)),
            ("IS", Some("NOT"), Some("DISTINCT"), Some("FROM")) => {
                (PRIORITY_IS, 4, Form::NotDistinct(true))
            }
            ("IS", Some("DISTINCT"), Some("FROM"), _) => (PRIORITY_IS, 3, Form::NotDistinct(false)),
            ("IS", Some("NULL"), ..) => (PRIORITY_IS, 2, Form::IsNull(false)),
            ("IS", Some("NOT"), Some("NULL"), _) => (PRIORITY_IS, 3, Form::IsNull(true)),
            ("BETWEEN", ..) => (PRIORITY_BETWEEN, 1, Form::Between(false)),
            ("NOT", Some("BETWEEN"), ..) => (PRIORITY_BETWEEN, 2, Form::Between(true)),
            ("LIKE", ..) => (PRIORITY_COMPARE, 1, Form::Like(false, false)),
            ("ILIKE", ..) => (PRIORITY_COMPARE, 1, Form::Like(true, false)),
            ("NOT", Some("LIKE"), ..) => (PRIORITY_COMPARE, 2, Form::Like(false, true)),
            ("NOT", Some("ILIKE"), ..) => (PRIORITY_COMPARE, 2, Form::Like(true, true)),
            ("REGEXP", ..) => (PRIORITY_COMPARE, 1, Form::Named("match")),
            ("IN", Some("PARTITION"), ..) => return None,
            ("IN", ..) => (PRIORITY_COMPARE, 1, Form::In(false, false)),
            ("NOT", Some("IN"), ..) => (PRIORITY_COMPARE, 2, Form::In(true, false)),
            ("GLOBAL", Some("IN"), ..) => (PRIORITY_COMPARE, 2, Form::In(false, true)),
            ("GLOBAL", Some("NOT"), Some("IN"), _) => (PRIORITY_COMPARE, 3, Form::In(true, true)),
            ("MOD", ..) => (PRIORITY_MULTIPLY, 1, Form::Binary(Expression::Mod)),
            ("DIV", ..) => (PRIORITY_MULTIPLY, 1, Form::Named("intDiv")),
            _ => return None,
        };
        Some(operator)
    }

    /// Operators of priority above `min` applied to operands, left to right.
    fn climb(&mut self, min: u8) -> Parsing<Expression> {
        let mut left: Expression = self.prefixed()?;
        let mut links: usize = 0;
        let mut after_ternary: bool = false;
        while let Some((priority, width, form)) = self.operator() {
            if priority <= min {
                break;
            }
            if after_ternary && priority <= PRIORITY_TERNARY_ELSE {
                return Err(self.error());
            }
            after_ternary = matches!(form, Form::Ternary);
            if !matches!(form, Form::Binary(_) | Form::NotDistinct(_)) {
                links += 1;
                self.within_depth(links)?;
            }
            self.at += width;
            left = self.apply(left, priority, form)?;
        }
        Ok(left)
    }

    /// `left` under the operator just consumed.
    fn apply(&mut self, left: Expression, priority: u8, form: Form) -> Parsing<Expression> {
        match form {
            Form::Lambda => {
                let parameters: Vec<Identifier> =
                    lambda_parameters(&left).ok_or_else(|| self.error())?;
                let body: Expression = self.deeper(|parser| parser.climb(PRIORITY_LAMBDA))?;
                Ok(lambda(parameters, body))
            }
            Form::Ternary => {
                let then: Expression = self.deeper(|parser| parser.climb(PRIORITY_TERNARY))?;
                self.expect_kind(TokenKind::Colon)?;
                let otherwise: Expression =
                    self.deeper(|parser| parser.climb(PRIORITY_TERNARY_ELSE))?;
                Ok(call("if", vec![left, then, otherwise]))
            }
            Form::IsNull(not) => Ok(null_test(left, not)),
            Form::Between(not) => {
                let low: Expression = self.deeper(|parser| parser.climb(PRIORITY_BETWEEN))?;
                self.expect_kw("AND")?;
                let high: Expression =
                    self.deeper(|parser| parser.climb(PRIORITY_FINISH_BETWEEN))?;
                Ok(between(left, low, high, not))
            }
            Form::Dot => self.tuple_element(left),
            Form::Subscript => {
                let index: Expression = self.expression()?;
                self.expect_kind(TokenKind::ClosingSquareBracket)?;
                Ok(Expression::Subscript(Box::new(
                    sqlquack_syntax::expressions::operators::models::Subscript {
                        this: left,
                        index,
                    },
                )))
            }
            Form::Cast => {
                let to = self.data_type()?;
                Ok(Expression::Cast(Box::new(
                    crate::parse::_helpers::tree::nodes::cast(left, to, true),
                )))
            }
            _ => {
                let right: Expression = self.deeper(|parser| parser.climb(priority))?;
                Ok(Self::combine(left, right, form))
            }
        }
    }

    /// A binary operator's node.
    fn combine(left: Expression, right: Expression, form: Form) -> Expression {
        match form {
            Form::Binary(kind) => binary(kind, left, right),
            Form::Named(name) => call(name, vec![left, right]),
            Form::NotDistinct(true) => binary(Expression::NullSafeEq, left, right),
            Form::NotDistinct(false) => binary(Expression::NullSafeNeq, left, right),
            Form::Like(ilike, not) => like(left, right, ilike, not),
            Form::In(not, global) => {
                let (items, query) = match right {
                    Expression::Tuple(tuple) => (tuple.expressions, None),
                    Expression::Subquery(subquery) => (Vec::new(), Some(subquery.this)),
                    other => (vec![other], None),
                };
                let mut node: Expression = in_list(left, items, query, not);
                if let Expression::In(node) = &mut node {
                    node.global = global;
                }
                node
            }
            _ => left,
        }
    }

    /// After `.`: `*`, a field name, or any operand, as `tupleElement`.
    fn tuple_element(&mut self, left: Expression) -> Parsing<Expression> {
        if self.is(TokenKind::Asterisk) {
            self.at += 1;
            let mut star = crate::parse::_helpers::tree::nodes::star(None, None);
            self.star_transformers(&mut star)?;
            return Ok(call(
                "tupleElement",
                vec![left, Expression::Star(Box::new(star))],
            ));
        }
        if self.is_at(0, crate::parse::_helpers::tree::cursor::is_identifier) {
            let field: Identifier = self.ident()?;
            return Ok(crate::parse::_helpers::tree::nodes::dot(left, field));
        }
        let index: Expression = self.deeper(|parser| parser.climb(PRIORITY_ACCESS))?;
        Ok(call("tupleElement", vec![left, index]))
    }

    /// The unary operators: leading `+` ignored, `NOT` (5) and `-` (13); `NOT (` is a call.
    fn prefixed(&mut self) -> Parsing<Expression> {
        while self.is(TokenKind::Plus) {
            self.at += 1;
        }
        if self.kw("NOT") {
            if self.kind_at(1, TokenKind::OpeningRoundBracket) {
                return self.operand();
            }
            self.at += 1;
            let this: Expression = self.deeper(|parser| parser.climb(PRIORITY_NOT))?;
            return Ok(unary(Expression::Not, this));
        }
        if self.is(TokenKind::Minus) {
            self.at += 1;
            let this: Expression = self.deeper(|parser| parser.climb(PRIORITY_NEGATE))?;
            return Ok(unary(Expression::Neg, this));
        }
        self.operand()
    }
}
