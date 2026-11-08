//! Search conditions and scalar expressions by SQL Server's precedence, each checked for its kind.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::operators::models::{
    BinaryOp, CollationExpr, LikeOp, QuantifiedExpr, QuantifiedOp, UnaryOp,
};
use sqlquack_syntax::expressions::typed_functions::models::AtTimeZone;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_any_name};
use crate::parse::_helpers::tree::nodes::{between, binary, exists, in_list, null_test};
use crate::scan::models::{Token, TokenKind};

/// The engine's error for a value where a condition is expected (error 4145).
const NON_BOOLEAN: &str =
    "An expression of non-boolean type specified in a context where a condition is expected.";

/// Whether `expression` is a condition, not a value.
pub(crate) fn is_condition(expression: &Expression) -> bool {
    match expression {
        Expression::And(_)
        | Expression::Or(_)
        | Expression::Not(_)
        | Expression::Eq(_)
        | Expression::Neq(_)
        | Expression::Lt(_)
        | Expression::Lte(_)
        | Expression::Gt(_)
        | Expression::Gte(_)
        | Expression::Like(_)
        | Expression::In(_)
        | Expression::Between(_)
        | Expression::IsNull(_)
        | Expression::Exists(_)
        | Expression::Any(_)
        | Expression::All(_)
        | Expression::NullSafeEq(_)
        | Expression::NullSafeNeq(_) => true,
        Expression::Paren(paren) => is_condition(&paren.this),
        _ => false,
    }
}

fn unary(kind: fn(Box<UnaryOp>) -> Expression, this: Expression) -> Expression {
    kind(Box::new(UnaryOp::new(this)))
}

type Build = fn(Box<BinaryOp>) -> Expression;

/// The comparison at `token`, with its node and its quantified form.
fn comparison(token: &Token) -> Option<(Build, QuantifiedOp)> {
    Some(match (token.kind, token.value.as_str()) {
        (TokenKind::Char('='), _) => (Expression::Eq, QuantifiedOp::Eq),
        (TokenKind::Op, "<>" | "!=") => (Expression::Neq, QuantifiedOp::Neq),
        (TokenKind::Char('<'), _) => (Expression::Lt, QuantifiedOp::Lt),
        (TokenKind::Char('>'), _) => (Expression::Gt, QuantifiedOp::Gt),
        (TokenKind::Op, "<=" | "!>") => (Expression::Lte, QuantifiedOp::Lte),
        (TokenKind::Op, ">=" | "!<") => (Expression::Gte, QuantifiedOp::Gte),
        _ => return None,
    })
}

/// The additive operator at `token`: `+ - & ^ |` share one level.
fn additive(token: &Token) -> Option<Build> {
    Some(match token.kind {
        TokenKind::Char('+') => Expression::Add,
        TokenKind::Char('-') => Expression::Sub,
        TokenKind::Char('&') => Expression::BitwiseAnd,
        TokenKind::Char('^') => Expression::BitwiseXor,
        TokenKind::Char('|') => Expression::BitwiseOr,
        _ => return None,
    })
}

fn multiplicative(token: &Token) -> Option<Build> {
    Some(match token.kind {
        TokenKind::Char('*') => Expression::Mul,
        TokenKind::Char('/') => Expression::Div,
        TokenKind::Char('%') => Expression::Mod,
        _ => return None,
    })
}

impl<'sql> Parser<'sql> {
    /// A search condition: what `WHERE`, `ON`, `HAVING`, `WHEN` and `IF` take.
    pub(crate) fn condition(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        let this: Expression = self.deeper(|parser| parser.or_expression())?;
        match is_condition(&this) {
            true => Ok(this),
            false => Err(self.non_boolean(start)),
        }
    }

    /// A scalar expression: a value, never a condition.
    pub(crate) fn scalar(&mut self) -> Parsing<Expression> {
        let start: usize = self.at;
        let this: Expression = self.deeper(|parser| parser.or_expression())?;
        self.value(this, start)
    }

    /// `this` when it is a value; the syntax error at its first token when it is a condition.
    fn value(&self, this: Expression, start: usize) -> Parsing<Expression> {
        match is_condition(&this) {
            false => Ok(this),
            true => Err(self.error_at(start)),
        }
    }

    fn non_boolean(&self, start: usize) -> crate::parse::errors::ParseError {
        crate::parse::errors::ParseError {
            message: NON_BOOLEAN.to_owned(),
            position: start,
            syntax: true,
        }
    }

    /// The syntax error at the token `index`.
    pub(crate) fn error_at(&self, index: usize) -> crate::parse::errors::ParseError {
        let mut probe: Parser<'sql> = Parser::new(self.sql, Vec::new());
        probe.tokens = self.tokens[index.min(self.tokens.len())..].to_vec();
        probe.error()
    }

    /// Either kind: a parenthesised operand, which may be a condition or a value.
    pub(crate) fn any_expression(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| parser.or_expression())
    }

    fn logical(
        &mut self,
        word: &str,
        build: Build,
        operand: fn(&mut Self) -> Parsing<Expression>,
    ) -> Parsing<Expression> {
        let start: usize = self.offset();
        let mut left: Expression = operand(self)?;
        let depth: usize = self.depth();
        while self.kw(word) {
            if !is_condition(&left) {
                return Err(self.non_boolean(start));
            }
            self.link()?;
            self.at += 1;
            let right_start: usize = self.offset();
            let right: Expression = self.deeper(operand)?;
            if !is_condition(&right) {
                return Err(self.non_boolean(right_start));
            }
            left = binary(build, left, right);
        }
        self.restore_depth(depth);
        Ok(left)
    }

    fn or_expression(&mut self) -> Parsing<Expression> {
        self.logical("OR", Expression::Or, Self::and_expression)
    }

    fn and_expression(&mut self) -> Parsing<Expression> {
        self.logical("AND", Expression::And, Self::not_expression)
    }

    fn not_expression(&mut self) -> Parsing<Expression> {
        if !self.kw("NOT") {
            return self.predicate();
        }
        self.at += 1;
        let start: usize = self.offset();
        let this: Expression = self.deeper(|parser| parser.not_expression())?;
        match is_condition(&this) {
            true => Ok(unary(Expression::Not, this)),
            false => Err(self.non_boolean(start)),
        }
    }

    /// Whether `<<` or `>>` is next: two touching angle brackets, SQL Server 2022's shifts.
    fn shift_at(&self) -> bool {
        match (self.peek(), self.peek_at(1)) {
            (Some(first), Some(second)) => {
                first.end == second.start
                    && matches!(
                        (first.value.as_str(), second.value.as_str()),
                        ("<", "<") | (">", ">")
                    )
            }
            _ => false,
        }
    }

    /// A value, then at most one comparison or predicate on it.
    fn predicate(&mut self) -> Parsing<Expression> {
        if self.kw("EXISTS") {
            self.at += 1;
            self.expect_char('(')?;
            let query: Expression = self.query_expression()?;
            self.expect_char(')')?;
            return Ok(exists(query));
        }
        let start: usize = self.at;
        let left: Expression = self.additive_expression()?;
        if self.shift_at() {
            return Err(self.gap("shift_operator", self.offset()));
        }
        if let Some((build, quantified)) = self.peek().and_then(comparison) {
            let left: Expression = self.value(left, start)?;
            self.at += 1;
            if self.kw_in(0, &["ALL", "ANY", "SOME"]) && self.char_at(1, '(') {
                let all: bool = self.kw("ALL");
                self.at += 2;
                let subquery: Expression = self.query_expression()?;
                self.expect_char(')')?;
                let node = Box::new(QuantifiedExpr {
                    this: left,
                    subquery,
                    op: Some(quantified),
                });
                return Ok(match all {
                    true => Expression::All(node),
                    false => Expression::Any(node),
                });
            }
            let right: Expression = self.operand()?;
            return Ok(binary(build, left, right));
        }
        let not: bool = self.kw("NOT");
        let offset: usize = usize::from(not);
        if self.kw_at(offset, "BETWEEN") {
            let left: Expression = self.value(left, start)?;
            self.at += offset + 1;
            let low: Expression = self.operand()?;
            self.expect_kw("AND")?;
            let high: Expression = self.operand()?;
            return Ok(between(left, low, high, not));
        }
        if self.kw_at(offset, "IN") {
            let left: Expression = self.value(left, start)?;
            self.at += offset + 1;
            self.expect_char('(')?;
            if self.starts_query() {
                let query: Expression = self.query_expression()?;
                self.expect_char(')')?;
                return Ok(in_list(left, Vec::new(), Some(query), not));
            }
            let items: Vec<Expression> = self.comma_list(Self::scalar)?;
            self.expect_char(')')?;
            return Ok(in_list(left, items, None, not));
        }
        if self.kw_at(offset, "LIKE") {
            let left: Expression = self.value(left, start)?;
            self.at += offset + 1;
            let pattern: Expression = self.operand()?;
            let mut node: LikeOp = LikeOp::new(left, pattern);
            node.negated = not;
            if self.eat_kw("ESCAPE") {
                node.escape = Some(self.operand()?);
            }
            return Ok(Expression::Like(Box::new(node)));
        }
        if self.kw("IS") {
            let left: Expression = self.value(left, start)?;
            self.at += 1;
            let not: bool = self.eat_kw("NOT");
            if self.eat_kws(&["DISTINCT", "FROM"]) {
                let right: Expression = self.operand()?;
                return Ok(match not {
                    true => binary(Expression::NullSafeEq, left, right),
                    false => binary(Expression::NullSafeNeq, left, right),
                });
            }
            self.expect_kw("NULL")?;
            return Ok(null_test(left, not));
        }
        Ok(left)
    }

    /// A comparison or predicate operand: a value at the additive level.
    fn operand(&mut self) -> Parsing<Expression> {
        let start: usize = self.at;
        let this: Expression = self.deeper(|parser| parser.additive_expression())?;
        self.value(this, start)
    }

    fn chain(
        &mut self,
        operator: fn(&Token) -> Option<Build>,
        operand: fn(&mut Self) -> Parsing<Expression>,
    ) -> Parsing<Expression> {
        let start: usize = self.at;
        let mut left: Expression = operand(self)?;
        let depth: usize = self.depth();
        while let Some(build) = self.peek().and_then(operator) {
            left = self.value(left, start)?;
            self.link()?;
            self.at += 1;
            let right_start: usize = self.at;
            let right: Expression = self.deeper(operand)?;
            let right: Expression = self.value(right, right_start)?;
            left = binary(build, left, right);
        }
        self.restore_depth(depth);
        Ok(left)
    }

    fn additive_expression(&mut self) -> Parsing<Expression> {
        self.chain(additive, Self::multiplicative_expression)
    }

    fn multiplicative_expression(&mut self) -> Parsing<Expression> {
        self.chain(multiplicative, Self::unary_expression)
    }

    /// `-x`, `+x` and `~x`, then a primary with its `COLLATE` and `AT TIME ZONE` suffixes.
    fn unary_expression(&mut self) -> Parsing<Expression> {
        let kind: Option<fn(Box<UnaryOp>) -> Expression> = match self.peek().map(|token| token.kind)
        {
            Some(TokenKind::Char('-')) => Some(Expression::Neg),
            Some(TokenKind::Char('+')) => Some(Expression::Pos),
            Some(TokenKind::Char('~')) => Some(Expression::BitwiseNot),
            _ => None,
        };
        if let Some(kind) = kind {
            self.at += 1;
            let start: usize = self.at;
            let this: Expression = self.deeper(|parser| parser.unary_expression())?;
            let this: Expression = self.value(this, start)?;
            return Ok(unary(kind, this));
        }
        let start: usize = self.at;
        let mut this: Expression = self.primary()?;
        let depth: usize = self.depth();
        loop {
            if self.kw("COLLATE") {
                this = self.value(this, start)?;
                self.link()?;
                self.at += 1;
                let name: Token = self.expect_token(is_any_name)?;
                this = Expression::Collation(Box::new(CollationExpr {
                    this,
                    collation: name.value,
                    quoted: false,
                    double_quoted: false,
                }));
            } else if self.kw("AT") && self.kw_at(1, "TIME") && self.kw_at(2, "ZONE") {
                this = self.value(this, start)?;
                self.link()?;
                self.at += 3;
                let zone_start: usize = self.at;
                let zone: Expression = self.deeper(|parser| parser.primary())?;
                let zone: Expression = self.value(zone, zone_start)?;
                this = Expression::AtTimeZone(Box::new(AtTimeZone { this, zone }));
            } else {
                self.restore_depth(depth);
                return Ok(this);
            }
        }
    }
}
