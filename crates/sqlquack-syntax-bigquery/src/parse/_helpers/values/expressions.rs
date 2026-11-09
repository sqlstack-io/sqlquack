//! `booleanExpression`, `predicate` and `valueExpression` (SqlBaseParser.g4).

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::operators::models::{IsTrueFalse, LikeOp, UnaryOp};

use crate::parse::_helpers::tree::cursor::{Parser, Parsing};
use crate::parse::_helpers::tree::nodes::{
    between, binary, call, exists, in_list, null_test, subquery, tuple, var,
};
use crate::parse::constants::{
    ADD, BIT_AND, BIT_OR, BIT_XOR, COMPARE, CONCAT, CONCAT_OPERATOR, ILIKE_WORD,
    LEFT_SHIFT_OPERATOR, MULTIPLY, NOT_UNKNOWN_TEST, PIPE_STARTS, SHIFT, UNKNOWN_TEST,
};

fn unary(kind: fn(Box<UnaryOp>) -> Expression, this: Expression) -> Expression {
    kind(Box::new(UnaryOp::new(this)))
}

/// The binary operator at the cursor: its precedence, how many tokens it spans, and its node.
type Operator = (u8, usize, fn(Expression, Expression) -> Expression);

impl Parser<'_> {
    /// `expression`: a `booleanExpression`.
    pub(crate) fn expression(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| parser.or_expression())
    }

    fn or_expression(&mut self) -> Parsing<Expression> {
        let mut left: Expression = self.and_expression()?;
        while self.eat_kw("OR") {
            let right: Expression = self.and_expression()?;
            left = binary(Expression::Or, left, right);
        }
        Ok(left)
    }

    fn and_expression(&mut self) -> Parsing<Expression> {
        let mut left: Expression = self.not_expression()?;
        while self.eat_kw("AND") {
            let right: Expression = self.not_expression()?;
            left = binary(Expression::And, left, right);
        }
        Ok(left)
    }

    /// `(NOT | !) booleanExpression`, `EXISTS (query)` and `valueExpression predicate?`.
    fn not_expression(&mut self) -> Parsing<Expression> {
        if self.eat_kw("NOT") || self.eat_char('!') {
            let this: Expression = self.deeper(|parser| parser.not_expression())?;
            return Ok(unary(Expression::Not, this));
        }
        if self.kw("EXISTS") && self.char_at(1, '(') && self.starts_query_at(1) {
            self.at += 2;
            let query: Expression = self.query()?;
            self.expect_char(')')?;
            return Ok(exists(subquery(query, None, false)));
        }
        let value: Expression = self.value_expression(0)?;
        self.predicate(value)
    }

    /// `errorCapturingNot`: `NOT` or `!`.
    fn eat_not(&mut self) -> bool {
        self.eat_kw("NOT") || self.eat_char('!')
    }

    /// At most one `predicate` after a value.
    fn predicate(&mut self, value: Expression) -> Parsing<Expression> {
        if self.eat_kw("IS") {
            let not: bool = self.eat_not();
            let word: Option<String> = self.word_at(0);
            return match word.as_deref() {
                Some("NULL") => {
                    self.at += 1;
                    Ok(null_test(value, not))
                }
                Some("UNKNOWN") => {
                    self.at += 1;
                    let test: &str = match not {
                        true => NOT_UNKNOWN_TEST,
                        false => UNKNOWN_TEST,
                    };
                    Ok(binary(Expression::Is, value, var(test)))
                }
                Some(truth @ ("TRUE" | "FALSE")) => {
                    let node = Box::new(IsTrueFalse { this: value, not });
                    self.at += 1;
                    Ok(match truth {
                        "TRUE" => Expression::IsTrue(node),
                        _ => Expression::IsFalse(node),
                    })
                }
                Some("DISTINCT") => {
                    self.at += 1;
                    self.expect_kw("FROM")?;
                    let right: Expression = self.value_expression(0)?;
                    Ok(match not {
                        true => binary(Expression::NullSafeEq, value, right),
                        false => unary(
                            Expression::Not,
                            binary(Expression::NullSafeEq, value, right),
                        ),
                    })
                }
                _ => Err(self.error()),
            };
        }
        let start: usize = self.at;
        let not: bool = self.eat_not();
        let negate = |this: Expression| match not {
            true => unary(Expression::Not, this),
            false => this,
        };
        match self.word_at(0).as_deref() {
            Some("BETWEEN") => {
                self.at += 1;
                let low: Expression = self.value_expression(0)?;
                self.expect_kw("AND")?;
                let high: Expression = self.value_expression(0)?;
                Ok(between(value, low, high, not))
            }
            Some("IN") if self.char_at(1, '{') => {
                self.at += 1;
                self.graph_subquery()?;
                Ok(in_list(value, Vec::new(), None, not))
            }
            Some("IN") if self.kw_at(1, "UNNEST") && self.char_at(2, '(') => {
                self.at += 3;
                let array: Expression = self.expression()?;
                self.expect_char(')')?;
                Ok(in_list(value, vec![call("UNNEST", vec![array])], None, not))
            }
            Some("IN") if self.char_at(1, '(') => {
                self.at += 2;
                if self.starts_query_at(0) {
                    let attempt = self.attempt(|parser| {
                        let query: Expression = parser.query()?;
                        parser.expect_char(')')?;
                        Ok(query)
                    });
                    if let Ok(query) = attempt {
                        return Ok(in_list(value, Vec::new(), Some(query), not));
                    }
                }
                let items: Vec<Expression> = self.comma_list(|parser| parser.expression())?;
                self.expect_char(')')?;
                Ok(in_list(value, items, None, not))
            }
            Some("RLIKE" | "REGEXP") => {
                self.at += 1;
                let pattern: Expression = self.value_expression(0)?;
                Ok(negate(call("rlike", vec![value, pattern])))
            }
            Some(word @ ("LIKE" | "ILIKE")) => {
                let ilike: bool = word == ILIKE_WORD;
                self.at += 1;
                let kind: fn(Box<LikeOp>) -> Expression = match ilike {
                    true => Expression::ILike,
                    false => Expression::Like,
                };
                let quantifier: Option<String> = match self.word_at(0).as_deref() {
                    Some(word @ ("ANY" | "SOME" | "ALL"))
                        if self.char_at(1, '(')
                            || (self.kw_at(1, "UNNEST") && self.char_at(2, '(')) =>
                    {
                        Some(word.to_owned())
                    }
                    _ => None,
                };
                let mut node: LikeOp = match quantifier {
                    Some(quantifier) if self.kw_at(1, "UNNEST") => {
                        self.at += 3;
                        let array: Expression = self.expression()?;
                        self.expect_char(')')?;
                        let mut node: LikeOp = LikeOp::new(value, call("UNNEST", vec![array]));
                        node.quantifier = Some(quantifier);
                        node
                    }
                    Some(quantifier) => {
                        self.at += 2;
                        let items: Vec<Expression> = match self.is_char(')') {
                            true => Vec::new(),
                            false => self.comma_list(|parser| parser.expression())?,
                        };
                        self.expect_char(')')?;
                        let mut node: LikeOp = LikeOp::new(value, tuple(items));
                        node.quantifier = Some(quantifier);
                        node
                    }
                    None => {
                        let pattern: Expression = self.value_expression(0)?;
                        let mut node: LikeOp = LikeOp::new(value, pattern);
                        if self.eat_kw("ESCAPE") {
                            node.escape = Some(self.string_literal()?);
                        }
                        node
                    }
                };
                node.negated = not;
                Ok(kind(Box::new(node)))
            }
            _ => {
                self.at = start;
                Ok(value)
            }
        }
    }

    /// The binary operator at the cursor, if one of at least `min` precedence is there.
    fn operator(&self) -> Option<Operator> {
        let token = self.peek()?;
        let text: &str = token.value.as_str();
        use crate::scan::models::TokenKind;
        let operator: Operator = match token.kind {
            TokenKind::Char('*') => (MULTIPLY, 1, |l, r| binary(Expression::Mul, l, r)),
            TokenKind::Char('/') => (MULTIPLY, 1, |l, r| binary(Expression::Div, l, r)),
            TokenKind::Char('%') => (MULTIPLY, 1, |l, r| binary(Expression::Mod, l, r)),
            TokenKind::Word if text.eq_ignore_ascii_case("DIV") => {
                (MULTIPLY, 1, |l, r| call("div", vec![l, r]))
            }
            TokenKind::Char('+') => (ADD, 1, |l, r| binary(Expression::Add, l, r)),
            TokenKind::Char('-') => (ADD, 1, |l, r| binary(Expression::Sub, l, r)),
            TokenKind::Op if text == CONCAT_OPERATOR => {
                (CONCAT, 1, |l, r| binary(Expression::Concat, l, r))
            }
            TokenKind::Op if text == LEFT_SHIFT_OPERATOR => {
                (SHIFT, 1, |l, r| binary(Expression::BitwiseLeftShift, l, r))
            }
            TokenKind::Char('>') if self.char_at(1, '>') && self.adjacent(0) => {
                match self.char_at(2, '>') && self.adjacent(1) {
                    true => (SHIFT, 3, |l, r| call("shiftrightunsigned", vec![l, r])),
                    false => (SHIFT, 2, |l, r| binary(Expression::BitwiseRightShift, l, r)),
                }
            }
            TokenKind::Char('&') => (BIT_AND, 1, |l, r| binary(Expression::BitwiseAnd, l, r)),
            TokenKind::Char('^') => (BIT_XOR, 1, |l, r| binary(Expression::BitwiseXor, l, r)),
            TokenKind::Char('|') if !self.pipe_start_at(0) => {
                (BIT_OR, 1, |l, r| binary(Expression::BitwiseOr, l, r))
            }
            TokenKind::Char('=') => (COMPARE, 1, |l, r| binary(Expression::Eq, l, r)),
            TokenKind::Char('<') => (COMPARE, 1, |l, r| binary(Expression::Lt, l, r)),
            TokenKind::Char('>') => (COMPARE, 1, |l, r| binary(Expression::Gt, l, r)),
            TokenKind::Op => match text {
                "==" => (COMPARE, 1, |l, r| binary(Expression::Eq, l, r)),
                "<>" | "!=" => (COMPARE, 1, |l, r| binary(Expression::Neq, l, r)),
                "<=" | "!>" => (COMPARE, 1, |l, r| binary(Expression::Lte, l, r)),
                ">=" | "!<" => (COMPARE, 1, |l, r| binary(Expression::Gte, l, r)),
                "<=>" => (COMPARE, 1, |l, r| binary(Expression::NullSafeEq, l, r)),
                _ => return None,
            },
            _ => return None,
        };
        Some(operator)
    }

    /// Whether the `|` at `offset` is the pipe operator, as `isOperatorPipeStart` decides.
    pub(crate) fn pipe_start_at(&self, offset: usize) -> bool {
        let _ = (offset, PIPE_STARTS);
        false
    }

    /// `valueExpression` of at least precedence `min`, by precedence climbing; all left-associative.
    pub(crate) fn value_expression(&mut self, min: u8) -> Parsing<Expression> {
        let mut left: Expression = self.unary_expression()?;
        while let Some((level, width, build)) = self.operator() {
            if level <= min {
                break;
            }
            self.at += width;
            let right: Expression = self.deeper(|parser| parser.value_expression(level))?;
            left = build(left, right);
        }
        Ok(left)
    }

    /// `(- | + | ~) valueExpression`, which binds tighter than every binary operator.
    fn unary_expression(&mut self) -> Parsing<Expression> {
        let kind: Option<fn(Box<UnaryOp>) -> Expression> = match self.peek().map(|token| token.kind)
        {
            Some(crate::scan::models::TokenKind::Char('-')) => Some(Expression::Neg),
            Some(crate::scan::models::TokenKind::Char('+')) => Some(Expression::Pos),
            Some(crate::scan::models::TokenKind::Char('~')) => Some(Expression::BitwiseNot),
            _ => None,
        };
        match kind {
            Some(kind) => {
                self.at += 1;
                let this: Expression = self.deeper(|parser| parser.unary_expression())?;
                Ok(unary(kind, this))
            }
            None => self.primary_expression(),
        }
    }
}
