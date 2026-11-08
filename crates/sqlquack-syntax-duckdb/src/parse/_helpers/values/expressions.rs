//! `Expression` (expression.gram): DuckDB 2.0.0's precedence levels, lowest first.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::{JSONExtract, Operator};
use sqlquack_syntax::expressions::names::models::{Identifier, Literal};
use sqlquack_syntax::expressions::operators::models::{
    CollationExpr, IsTrueFalse, LikeOp, QuantifiedExpr, QuantifiedOp, SimilarToExpr, UnaryOp,
};
use sqlquack_syntax::expressions::typed_functions::models::AtTimeZone;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_label};
use crate::parse::_helpers::tree::nodes::{
    between, binary, call, cast, in_list, lambda, null_test, subquery, var,
};
use crate::parse::constants::{GENERIC_OPERATOR_CHARACTERS, SPECIAL_PRECEDENCE_OPERATORS};
use crate::scan::models::{Token, TokenKind};

/// The transformer's refusal of `ANY` or `ALL` after an operator that is no comparison.
const QUANTIFIED_COMPARISON: &str =
    "ANY and ALL operators require one of =,<>,>,<,>=,<= comparisons!";
/// The regular-expression operators `ANY` and `ALL` take over a list.
const REGEX_ANY_OPERATORS: [&str; 4] = ["~", "!~", "~*", "!~*"];
/// The transformer's refusal of two comparisons in a row.
const CHAINED_COMPARISONS: &str =
    "Chained comparisons are not supported, use AND to combine comparisons";

/// `~` and `!~`: `regexp_matches`, a partial match, as the transformer names them by default.
const REGEX_MATCH: &str = "regexp_matches";
/// `~*` and `!~*`, kept as their operator.
const REGEX_INSENSITIVE: &str = "~*";
/// `BetweenLikeOp`'s refusal of `NOT` before `!~~`, `!~~*`, `!~` or `!~*`.
const NOT_NEGATED: &str =
    "NOT cannot precede an operator that is already negated (!~~, !~~*, !~ or !~*)";
/// `LikeClause`'s refusal of `ESCAPE` after `~*` or `!~*`.
const INSENSITIVE_ESCAPE: &str =
    "ESCAPE clause is not supported with case-insensitive regular expression match operators";

/// The kind of an `IsExpressionTail`, as far as the chained-comparison check reads it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tail {
    Distinct,
    Comparison,
    Other,
}

/// The transformer's refusal of an operator name with more than a schema in front of it.
const OPERATOR_QUALIFIERS: &str =
    "Too many identifiers found, expected schema.operator or operator";

/// A binary node from its two operands.
type Build = Box<dyn Fn(Expression, Expression) -> Expression>;

/// An operand parser of one precedence level.
type Level<'sql> = fn(&mut Parser<'sql>) -> Parsing<Expression>;

fn unary(kind: fn(Box<UnaryOp>) -> Expression, this: Expression) -> Expression {
    kind(Box::new(UnaryOp::new(this)))
}

fn negated(not: bool, this: Expression) -> Expression {
    match not {
        true => unary(Expression::Not, this),
        false => this,
    }
}

/// An operator the shared tree has no node for, kept with its text.
fn operator(this: Expression, text: &str, expression: Expression) -> Expression {
    Expression::Operator(Box::new(Operator {
        this: Box::new(this),
        operator: Some(Box::new(var(text))),
        expression: Box::new(expression),
        comments: Vec::new(),
    }))
}

fn builder(
    kind: fn(Box<sqlquack_syntax::expressions::operators::models::BinaryOp>) -> Expression,
) -> Build {
    Box::new(move |left, right| binary(kind, left, right))
}

fn text_builder(text: String) -> Build {
    Box::new(move |left, right| match text.as_str() {
        "||" => binary(Expression::Concat, left, right),
        _ => operator(left, &text, right),
    })
}

/// The names of a lambda's left side: one unqualified column, or a list of them.
fn lambda_parameters(left: &Expression) -> Option<Vec<Identifier>> {
    let name = |expression: &Expression| match expression {
        Expression::Column(column) if column.table.is_none() => Some(column.name.clone()),
        _ => None,
    };
    match left {
        Expression::Tuple(tuple) => tuple.expressions.iter().map(name).collect(),
        Expression::Paren(paren) => name(&paren.this).map(|name| vec![name]),
        other => name(other).map(|name| vec![name]),
    }
}

/// `left -> right`: a lambda over names, or else the JSON arrow, as the binder decides later.
fn arrow(left: Expression, right: Expression) -> Expression {
    match (lambda_parameters(&left), &right) {
        (Some(parameters), body) if !matches!(body, Expression::Literal(_)) => {
            lambda(parameters, right)
        }
        _ => Expression::JSONExtract(Box::new(JSONExtract {
            this: Box::new(left),
            expression: Box::new(right),
            only_json_types: None,
            expressions: Vec::new(),
            variant_extract: None,
            json_query: None,
            option: None,
            quote: None,
            on_condition: None,
            requires_json: None,
        })),
    }
}

/// Whether `token` is an operator `OperatorLiteral` matches; `all` takes the special ones too.
fn is_generic_operator(token: &Token, all: bool) -> bool {
    let text: &str = token.value.as_str();
    let operator: bool = match token.kind {
        TokenKind::Op | TokenKind::Char(_) => {
            !text.is_empty()
                && text
                    .bytes()
                    .all(|byte| GENERIC_OPERATOR_CHARACTERS.contains(&byte))
        }
        _ => false,
    };
    operator && (all || !SPECIAL_PRECEDENCE_OPERATORS.contains(&text))
}

/// The quantifier comparison a `ANY`/`ALL` operator names, when it is a comparison.
fn quantified_operator(text: &str) -> Option<QuantifiedOp> {
    match text {
        "=" | "==" => Some(QuantifiedOp::Eq),
        "<>" | "!=" => Some(QuantifiedOp::Neq),
        "<" => Some(QuantifiedOp::Lt),
        "<=" => Some(QuantifiedOp::Lte),
        ">" => Some(QuantifiedOp::Gt),
        ">=" => Some(QuantifiedOp::Gte),
        _ => None,
    }
}

impl<'sql> Parser<'sql> {
    /// `Expression`.
    pub(crate) fn expression(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| parser.lambda_arrow())
    }

    /// An operand of `BETWEEN`, `LIKE` and `IN`: `OtherOperatorExpression`.
    pub(crate) fn value_expression(&mut self, _min: u8) -> Parsing<Expression> {
        self.deeper(|parser| parser.other_operator_expression())
    }

    /// `left (operator right)*` at one level; a tail whose right operand fails is not taken.
    fn chain(
        &mut self,
        operand: Level<'sql>,
        operator: fn(&Self) -> Option<(usize, Build)>,
    ) -> Parsing<Expression> {
        let mut left: Expression = operand(self)?;
        let mut links: usize = 0;
        while let Some((width, build)) = operator(self) {
            let right = self.attempt(|parser| {
                parser.at += width;
                parser.deeper(operand)
            });
            match right {
                Ok(right) => left = build(left, right),
                Err(_) => break,
            }
            links = match left {
                Expression::And(_) | Expression::Or(_) => self.flat_link(links)?,
                _ => self.link(links)?,
            };
        }
        Ok(left)
    }

    /// `LambdaArrowExpression`: `a -> b -> c`, each arrow over `OR` operands.
    fn lambda_arrow(&mut self) -> Parsing<Expression> {
        self.chain(Self::or_expression, |parser| {
            parser.is_op("->").then(|| (1, Box::new(arrow) as Build))
        })
    }

    /// `ColumnDefaultExpr`: `OR` and `AND` over `IsDistinctFromExpression`, so `NOT NULL` follows.
    pub(crate) fn column_default(&mut self) -> Parsing<Expression> {
        self.deeper(|parser| {
            parser.chain(Self::column_default_and, |parser| {
                parser.kw("OR").then(|| (1, builder(Expression::Or)))
            })
        })
    }

    fn column_default_and(&mut self) -> Parsing<Expression> {
        self.chain(Self::distinct_from_expression, |parser| {
            parser.kw("AND").then(|| (1, builder(Expression::And)))
        })
    }

    fn or_expression(&mut self) -> Parsing<Expression> {
        self.chain(Self::and_expression, |parser| {
            parser.kw("OR").then(|| (1, builder(Expression::Or)))
        })
    }

    fn and_expression(&mut self) -> Parsing<Expression> {
        self.chain(Self::not_expression, |parser| {
            parser.kw("AND").then(|| (1, builder(Expression::And)))
        })
    }

    /// `LogicalNotExpression`: `NOT* IsExpression`.
    fn not_expression(&mut self) -> Parsing<Expression> {
        match self.eat_kw("NOT") {
            true => {
                let this: Expression = self.deeper(|parser| parser.not_expression())?;
                Ok(unary(Expression::Not, this))
            }
            false => self.test_expression(),
        }
    }

    /// The `IS` test at the cursor applied to `this`, and whether one was there.
    fn null_or_truth_test(&mut self, this: Expression) -> (Expression, bool) {
        if self.kw("IS") {
            let not: bool = self.kw_at(1, "NOT");
            let at: usize = 1 + usize::from(not);
            let node = match self.word_at(at).as_deref() {
                Some("NULL" | "UNKNOWN") => null_test(this, not),
                Some("TRUE") => Expression::IsTrue(Box::new(IsTrueFalse { this, not })),
                Some("FALSE") => Expression::IsFalse(Box::new(IsTrueFalse { this, not })),
                _ => return (this, false),
            };
            self.at += at + 1;
            return (node, true);
        }
        if self.kw("NOT") && self.kw_at(1, "NULL") {
            self.at += 2;
            return (null_test(this, true), true);
        }
        match self.word_at(0).as_deref() {
            Some("NOTNULL") => {
                self.at += 1;
                (null_test(this, true), true)
            }
            Some("ISNULL") => {
                self.at += 1;
                (null_test(this, false), true)
            }
            _ => (this, false),
        }
    }

    /// `IsExpression`: a test, then further tests, comparisons, operators or casts.
    fn test_expression(&mut self) -> Parsing<Expression> {
        let position: usize = self.offset();
        let this: Expression = self.distinct_from_expression()?;
        let (mut this, tested) = self.null_or_truth_test(this);
        if !tested {
            return Ok(this);
        }
        let mut previous: Tail = Tail::Other;
        loop {
            let start: usize = self.at;
            let tested: bool;
            (this, tested) = self.null_or_truth_test(this);
            if tested {
                previous = Tail::Other;
            }
            let tail: Tail = match (
                self.distinct_from_at(),
                self.comparison_operator().is_some(),
            ) {
                (true, _) => Tail::Distinct,
                (false, true) => Tail::Comparison,
                _ => Tail::Other,
            };
            if tail != Tail::Other && tail == previous {
                self.refuse_later(CHAINED_COMPARISONS, position);
            }
            previous = tail;
            if self.distinct_from_at() {
                this = self.distinct_from_tail(this)?;
            } else if let Some((width, build)) = self.comparison_operator() {
                self.at += width;
                let right: Expression = self.comparison_operand()?;
                this = build(this, right);
            } else if self
                .peek()
                .is_some_and(|token| is_generic_operator(token, false))
            {
                let text: String = self.advance()?.value;
                let right: Expression = self.bitwise_expression()?;
                this = text_builder(text)(this, right);
            } else if self.eat_op("::") {
                let to = self.data_type()?;
                this = Expression::Cast(Box::new(cast(this, to, true)));
            } else if self.at == start {
                return Ok(this);
            }
        }
    }

    /// Whether `IS [NOT] DISTINCT FROM` comes next.
    fn distinct_from_at(&self) -> bool {
        let not: usize = usize::from(self.kw_at(1, "NOT"));
        self.kw("IS") && self.kw_at(1 + not, "DISTINCT") && self.kw_at(2 + not, "FROM")
    }

    fn distinct_from_tail(&mut self, left: Expression) -> Parsing<Expression> {
        let not: bool = self.kw_at(1, "NOT");
        self.at += 3 + usize::from(not);
        let right: Expression = self.comparison_expression()?;
        Ok(match not {
            true => binary(Expression::NullSafeEq, left, right),
            false => binary(Expression::NullSafeNeq, left, right),
        })
    }

    /// `IsDistinctFromExpression`.
    fn distinct_from_expression(&mut self) -> Parsing<Expression> {
        let position: usize = self.offset();
        let mut left: Expression = self.comparison_expression()?;
        let mut tails: usize = 0;
        while self.distinct_from_at() {
            left = self.distinct_from_tail(left)?;
            tails += 1;
        }
        if tails > 1 {
            self.refuse_later(CHAINED_COMPARISONS, position);
        }
        Ok(left)
    }

    /// The comparison operator at the cursor.
    fn comparison_operator(&self) -> Option<(usize, Build)> {
        let token: &Token = self.peek()?;
        let kind = match (token.kind, token.value.as_str()) {
            (TokenKind::Char('='), _) | (TokenKind::Op, "==") => Expression::Eq,
            (TokenKind::Op, "!=" | "<>") => Expression::Neq,
            (TokenKind::Char('<'), _) => Expression::Lt,
            (TokenKind::Char('>'), _) => Expression::Gt,
            (TokenKind::Op, "<=") => Expression::Lte,
            (TokenKind::Op, ">=") => Expression::Gte,
            _ => return None,
        };
        let quantified: bool = self.kw_in(1, &["ANY", "SOME", "ALL"]);
        (!quantified).then(|| (1, builder(kind)))
    }

    /// `ComparisonOperand`: `NOT+ IsExpression`, or a `BetweenInLikeExpression`.
    fn comparison_operand(&mut self) -> Parsing<Expression> {
        match self.kw("NOT") {
            true => self.not_expression(),
            false => self.between_in_like_expression(),
        }
    }

    /// `ComparisonExpression`.
    fn comparison_expression(&mut self) -> Parsing<Expression> {
        let position: usize = self.offset();
        let mut left: Expression = self.between_in_like_expression()?;
        let mut tails: usize = 0;
        while let Some((width, build)) = self.comparison_operator() {
            let right = self.attempt(|parser| {
                parser.at += width;
                parser.comparison_operand()
            });
            match right {
                Ok(right) => left = build(left, right),
                Err(_) => break,
            }
            tails += 1;
        }
        if tails > 1 {
            self.refuse_later(CHAINED_COMPARISONS, position);
        }
        Ok(left)
    }

    /// The `LikeVariations` at `offset`: its width, its node kind, and whether it negates.
    fn like_at(&self, offset: usize) -> Option<(usize, &'static str, bool)> {
        let token: &Token = self.peek_at(offset)?;
        let found = match (token.kind, token.value.to_ascii_uppercase().as_str()) {
            (TokenKind::Word, "LIKE") | (TokenKind::Op, "~~") => (1, "like", false),
            (TokenKind::Word, "ILIKE") | (TokenKind::Op, "~~*") => (1, "ilike", false),
            (TokenKind::Word, "GLOB") | (TokenKind::Op, "~~~") => (1, "glob", false),
            (TokenKind::Word, "SIMILAR") if self.kw_at(offset + 1, "TO") => (2, "similar", false),
            (TokenKind::Char('~'), _) => (1, REGEX_MATCH, false),
            (TokenKind::Op, "~*") => (1, REGEX_INSENSITIVE, false),
            (TokenKind::Op, "!~~*") => (1, "ilike", true),
            (TokenKind::Op, "!~~") => (1, "like", true),
            (TokenKind::Op, "!~*") => (1, REGEX_INSENSITIVE, true),
            (TokenKind::Op, "!~") => (1, REGEX_MATCH, true),
            _ => return None,
        };
        Some(found)
    }

    /// `BetweenInLikeExpression`: an operand, `IN` predicates, then one `BETWEEN` or `LIKE`.
    fn between_in_like_expression(&mut self) -> Parsing<Expression> {
        let mut this: Expression = self.other_operator_expression()?;
        loop {
            let not: bool = self.kw("NOT");
            let offset: usize = usize::from(not);
            if !self.kw_at(offset, "IN") {
                break;
            }
            self.at += offset + 1;
            this = self.in_tail(this, not)?;
            while self.eat_op("::") {
                let to = self.data_type()?;
                this = Expression::Cast(Box::new(cast(this, to, true)));
            }
        }
        let not: bool = self.kw("NOT");
        let offset: usize = usize::from(not);
        if self.kw_at(offset, "BETWEEN") {
            let range = self.attempt(|parser| {
                parser.at += offset + 1;
                parser.eat_kw("ASYMMETRIC");
                let low: Expression = parser.other_operator_expression()?;
                parser.expect_kw("AND")?;
                Ok((low, parser.other_operator_expression()?))
            });
            return Ok(match range {
                Ok((low, high)) => between(this, low, high, not),
                Err(_) => this,
            });
        }
        let Some((width, kind, negates)) = self.like_at(offset) else {
            return Ok(this);
        };
        let position: usize = self.offset();
        self.at += offset + width;
        let pattern: Expression = self.other_operator_expression()?;
        let escape: Option<Expression> = match self.eat_kw("ESCAPE") {
            true => Some(self.other_operator_expression()?),
            false => None,
        };
        if kind == REGEX_INSENSITIVE && escape.is_some() {
            self.refuse_later(INSENSITIVE_ESCAPE, position);
        }
        if not && negates {
            self.refuse_later(NOT_NEGATED, position);
        }
        let not: bool = not != negates;
        Ok(match kind {
            "like" | "ilike" => {
                let mut node: LikeOp = LikeOp::new(this, pattern);
                node.escape = escape;
                node.negated = not;
                match kind {
                    "like" => Expression::Like(Box::new(node)),
                    _ => Expression::ILike(Box::new(node)),
                }
            }
            "similar" => Expression::SimilarTo(Box::new(SimilarToExpr {
                this,
                pattern,
                escape,
                not,
            })),
            "glob" => negated(not, binary(Expression::Glob, this, pattern)),
            REGEX_MATCH => {
                let mut args: Vec<Expression> = vec![this, pattern];
                args.extend(escape);
                negated(not, call(REGEX_MATCH, args))
            }
            other => negated(not, operator(this, other, pattern)),
        })
    }

    /// `InExpression` after `[NOT] IN`: a list, a subquery, or any operand.
    fn in_tail(&mut self, this: Expression, not: bool) -> Parsing<Expression> {
        if self.is_char('(') && self.starts_query_at(1) {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let query: Expression = parser.query()?;
                parser.expect_char(')')?;
                parser.subqueries += 1;
                Ok(query)
            });
            if let Ok(query) = attempt {
                return Ok(in_list(this, Vec::new(), Some(query), not));
            }
        }
        if self.is_char('(') {
            let attempt = self.attempt(|parser| {
                parser.at += 1;
                let items: Vec<Expression> = parser.expression_list(')')?;
                parser.expect_char(')')?;
                Ok(items)
            });
            if let Ok(items) = attempt {
                return Ok(in_list(this, items, None, not));
            }
        }
        let container: Expression = self.other_operator_expression()?;
        Ok(negated(not, call("contains", vec![container, this])))
    }

    /// `List(Expression)` with an optional trailing comma, up to `close`.
    pub(crate) fn expression_list(&mut self, close: char) -> Parsing<Vec<Expression>> {
        let mut items: Vec<Expression> = vec![self.expression()?];
        while self.eat_char(',') {
            if self.is_char(close) {
                break;
            }
            items.push(self.expression()?);
        }
        Ok(items)
    }

    /// `OtherOperatorExpression`: bitwise operands joined by other operators, or a prefix one.
    fn other_operator_expression(&mut self) -> Parsing<Expression> {
        let prefix: bool = self.peek().is_some_and(|token| {
            is_generic_operator(token, false)
                && !matches!(token.kind, TokenKind::Char('-' | '+' | '*'))
        });
        if prefix {
            let text: String = self.advance()?.value;
            let this: Expression = self.deeper(|parser| parser.other_operator_expression())?;
            return Ok(call(&text, vec![this]));
        }
        let mut left: Expression = self.bitwise_expression()?;
        let mut links: usize = 0;
        loop {
            let quantified: bool = self
                .peek()
                .is_some_and(|token| is_generic_operator(token, true))
                && self.kw_in(1, &["ANY", "SOME", "ALL"]);
            if quantified {
                let position: usize = self.offset();
                let text: String = self.advance()?.value;
                let all: bool = self.eat_kw("ALL");
                if !all {
                    self.at += 1;
                }
                let right: Expression = self.bitwise_expression()?;
                let subquery: Expression = match right {
                    Expression::Paren(paren) => paren.this,
                    other => other,
                };
                if quantified_operator(&text).is_none() {
                    match is_query(&subquery) {
                        true => self.refuse_later(QUANTIFIED_COMPARISON, position),
                        false if !REGEX_ANY_OPERATORS.contains(&text.as_str()) => {
                            let message: String =
                                format!("Unsupported comparison \"{text}\" for ANY/ALL subquery");
                            self.refuse_later(&message, position);
                        }
                        false => {}
                    }
                }
                let node = Box::new(QuantifiedExpr {
                    this: left,
                    subquery,
                    op: quantified_operator(&text),
                });
                left = match all {
                    true => Expression::All(node),
                    false => Expression::Any(node),
                };
                links = self.link(links)?;
                continue;
            }
            let start: usize = self.at;
            let text: String = if self.kw("OPERATOR") && self.char_at(1, '(') {
                let (text, qualifiers) = self.qualified_operator()?;
                if qualifiers > 1 {
                    self.refuse_later(OPERATOR_QUALIFIERS, start);
                }
                text
            } else if self
                .peek()
                .is_some_and(|token| is_generic_operator(token, false))
            {
                self.advance()?.value
            } else {
                return Ok(left);
            };
            let right = self.attempt(|parser| parser.bitwise_expression());
            match right {
                Ok(right) => left = text_builder(text)(left, right),
                Err(_) => {
                    self.at = start;
                    return Ok(left);
                }
            }
            links = self.link(links)?;
        }
    }

    /// `OPERATOR ( [schema.]op )`, consumed; its operator text and how many names qualify it.
    fn qualified_operator(&mut self) -> Parsing<(String, usize)> {
        let start: usize = self.at;
        self.at += 2;
        let mut qualifiers: usize = 0;
        while self.is(is_col_label) && self.char_at(1, '.') {
            self.at += 2;
            qualifiers += 1;
        }

        let token: Token = self.advance()?;
        if !is_generic_operator(&token, true) {
            self.at = start;
            return Err(self.error());
        }
        self.expect_char(')')?;
        Ok((token.value, qualifiers))
    }

    /// `BitwiseExpression`: `& | << >>` over `~` operands.
    fn bitwise_expression(&mut self) -> Parsing<Expression> {
        self.chain(Self::tilde_expression, |parser| {
            let token: &Token = parser.peek()?;
            let kind = match (token.kind, token.value.as_str()) {
                (TokenKind::Char('&'), _) => Expression::BitwiseAnd,
                (TokenKind::Char('|'), _) => Expression::BitwiseOr,
                (TokenKind::Op, "<<") => Expression::BitwiseLeftShift,
                (TokenKind::Op, ">>") => Expression::BitwiseRightShift,
                _ => return None,
            };
            Some((1, builder(kind)))
        })
    }

    /// `TildeExpression`: `~* AdditiveExpression`.
    fn tilde_expression(&mut self) -> Parsing<Expression> {
        match self.eat_char('~') {
            true => {
                let this: Expression = self.deeper(|parser| parser.tilde_expression())?;
                Ok(unary(Expression::BitwiseNot, this))
            }
            false => self.additive_expression(),
        }
    }

    fn additive_expression(&mut self) -> Parsing<Expression> {
        self.chain(Self::multiplicative_expression, |parser| {
            let kind = match parser.peek()?.kind {
                TokenKind::Char('+') => Expression::Add,
                TokenKind::Char('-') => Expression::Sub,
                _ => return None,
            };
            Some((1, builder(kind)))
        })
    }

    fn multiplicative_expression(&mut self) -> Parsing<Expression> {
        self.chain(Self::exponentiation_expression, |parser| {
            let token: &Token = parser.peek()?;
            let build: Build = match (token.kind, token.value.as_str()) {
                (TokenKind::Char('*'), _) => builder(Expression::Mul),
                (TokenKind::Char('/'), _) => builder(Expression::Div),
                (TokenKind::Char('%'), _) => builder(Expression::Mod),
                (TokenKind::Op, "//") => text_builder("//".to_owned()),
                _ => return None,
            };
            Some((1, build))
        })
    }

    fn exponentiation_expression(&mut self) -> Parsing<Expression> {
        self.chain(Self::collate_expression, |parser| {
            let token: &Token = parser.peek()?;
            match (token.kind, token.value.as_str()) {
                (TokenKind::Char('^'), _) | (TokenKind::Op, "**") => {
                    Some((1, text_builder(token.value.clone())))
                }
                _ => None,
            }
        })
    }

    fn collate_expression(&mut self) -> Parsing<Expression> {
        self.chain(Self::at_time_zone_expression, |parser| {
            parser.kw("COLLATE").then(|| {
                let build: Build = Box::new(|this, name| {
                    let (collation, quoted, double_quoted) = collation_name(&name);
                    Expression::Collation(Box::new(CollationExpr {
                        this,
                        collation,
                        quoted,
                        double_quoted,
                    }))
                });
                (1, build)
            })
        })
    }

    fn at_time_zone_expression(&mut self) -> Parsing<Expression> {
        self.chain(Self::prefix_expression, |parser| {
            parser.eat_kws_ahead(&["AT", "TIME", "ZONE"]).then(|| {
                let build: Build = Box::new(|this, zone| {
                    Expression::AtTimeZone(Box::new(AtTimeZone { this, zone }))
                });
                (3, build)
            })
        })
    }

    /// Whether the words `names` come next, in order.
    fn eat_kws_ahead(&self, names: &[&str]) -> bool {
        names
            .iter()
            .enumerate()
            .all(|(offset, name)| self.kw_at(offset, name))
    }

    /// `PrefixExpression`: `- + OPERATOR(...)` prefixes on a `BaseExpression`.
    fn prefix_expression(&mut self) -> Parsing<Expression> {
        let kind: Option<fn(Box<UnaryOp>) -> Expression> = match self.peek().map(|token| token.kind)
        {
            Some(TokenKind::Char('-')) => Some(Expression::Neg),
            Some(TokenKind::Char('+')) => Some(Expression::Pos),
            _ => None,
        };
        if let Some(kind) = kind {
            self.at += 1;
            let this: Expression = self.deeper(|parser| parser.prefix_expression())?;
            return Ok(unary(kind, this));
        }
        if self.kw("OPERATOR") && self.char_at(1, '(') {
            let (text, _) = self.qualified_operator()?;
            let this: Expression = self.deeper(|parser| parser.prefix_expression())?;
            return Ok(call(&text, vec![this]));
        }
        self.primary_expression()
    }

    /// A query in parentheses as an expression.
    pub(crate) fn parenthesized_query(&mut self) -> Parsing<Expression> {
        self.expect_char('(')?;
        let query: Expression = self.query()?;
        self.expect_char(')')?;
        self.subqueries += 1;
        Ok(subquery(query, None, false))
    }
}

/// `TransformCollateExpression`'s name, whether it was a string, and whether a part was quoted.
fn collation_name(name: &Expression) -> (String, bool, bool) {
    match name {
        Expression::Paren(paren) => collation_name(&paren.this),
        Expression::Literal(literal) => match literal.as_ref() {
            Literal::Number(text) => (text.replace('_', ""), false, false),
            Literal::String(text) | Literal::EscapeString(text) | Literal::DollarString(text) => {
                (text.clone(), true, false)
            }
            Literal::HexString(hex) => (blob_text(hex), false, false),
            Literal::BitString(bits) => (bits.clone(), false, false),
            _ => (String::new(), false, false),
        },
        Expression::Neg(negation) => match &negation.this {
            Expression::Literal(literal) => match literal.as_ref() {
                Literal::Number(text) => (format!("-{}", text.replace('_', "")), false, false),
                _ => (String::new(), false, false),
            },
            _ => (String::new(), false, false),
        },
        Expression::Null(_) => ("NULL".to_owned(), false, false),
        Expression::Boolean(value) => (value.value.to_string(), false, false),
        _ => match column_path(name) {
            Some(parts) => {
                let quoted: bool = parts.iter().any(|part| part.quoted);
                let names: Vec<&str> = parts.iter().map(|part| part.name.as_str()).collect();
                (names.join("."), false, quoted)
            }
            None => (String::new(), false, false),
        },
    }
}

/// A blob's text as DuckDB writes it: printable bytes as themselves, and quotes, backslashes and other bytes as hex escapes.
fn blob_text(hex: &str) -> String {
    hex.as_bytes()
        .chunks(2)
        .filter_map(|pair| {
            let high: u32 = char::from(*pair.first()?).to_digit(16)?;
            let low: u32 = char::from(*pair.get(1)?).to_digit(16)?;
            Some(high * 16 + low)
        })
        .map(|byte| match (byte, char::from_u32(byte)) {
            (0x20..=0x7e, Some(printable)) if !matches!(printable, '"' | '\'' | '\\') => {
                printable.to_string()
            }
            _ => format!("\\x{byte:02x}"),
        })
        .collect()
}

/// The parts of a column reference, `a.b.c` read as a column and its fields.
fn column_path(name: &Expression) -> Option<Vec<&Identifier>> {
    match name {
        Expression::Column(column) => Some(column.table.iter().chain([&column.name]).collect()),
        Expression::Dot(dot) => {
            let mut parts: Vec<&Identifier> = column_path(&dot.this)?;
            parts.push(&dot.field);
            Some(parts)
        }
        _ => None,
    }
}

/// Whether `expression` is a subquery once parentheses fold away.
fn is_query(expression: &Expression) -> bool {
    match expression {
        Expression::Paren(paren) => is_query(&paren.this),
        Expression::Subquery(_) | Expression::Select(_) | Expression::Union(_) => true,
        _ => false,
    }
}
