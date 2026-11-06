//! `a_expr` and `b_expr` (gram.y): operators by precedence climbing over gram.y's levels.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::columns::models::WithOperator;
use sqlquack_syntax::expressions::functions::models::{Function, Operator};
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::operators::models::{
    BinaryOp, CollationExpr, IsJson, IsTrueFalse, JsonUniqueKeys, LikeOp, QuantifiedExpr,
    QuantifiedOp, SimilarToExpr, UnaryOp,
};
use sqlquack_syntax::expressions::statements::models::Var;
use sqlquack_syntax::expressions::typed_functions::models::AtTimeZone;
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_col_id};
use crate::parse::_helpers::tree::nodes::{
    between, binary, cast, function, in_list, keyword_call, literal, name_text, null_test,
    subquery,
};
use crate::parse::constants::{
    ADD, AND, AT, BITWISE_NOT, CATALOGUE_SCHEMA, COLLATE, COMPARE, EXPONENT, ILIKE, IS, LIKE,
    MULTIPLY, NOT, NOT_UNKNOWN_TEST, OPERATOR, OR, TIMEZONE_FUNCTION, TYPECAST, UNARY,
    UNKNOWN_TEST,
};
use crate::scan::models::{Token, TokenKind};

/// `this`, or `NOT this` as gram.y's `makeNotExpr` makes it when `not` is set.
fn negated(not: bool, this: Expression) -> Expression {
    match not {
        true => unary(Expression::Not, this),
        false => this,
    }
}

/// `x IS [form] NORMALIZED`: the `is_normalized` call gram.y makes, with the form as a string.
fn normalized(this: Expression, form: Option<&str>) -> Expression {
    let mut args: Vec<Expression> = vec![this];
    args.extend(form.map(|form| literal(Literal::String(form.to_uppercase()))));
    Expression::Function(Box::new(function(vec![Identifier::new("is_normalized")], args)))
}

/// Which of gram.y's two expression grammars: `a_expr`, or the restricted `b_expr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flavor {
    A,
    B,
}

/// The text of an operator token: `<=`, `>=` and `<>` for their own tokens, `<>` for `!=`.
pub(crate) fn operator_text(token: &Token) -> String {
    match token.kind {
        TokenKind::LessEquals => "<=".to_owned(),
        TokenKind::GreaterEquals => ">=".to_owned(),
        TokenKind::NotEquals => "<>".to_owned(),
        TokenKind::Char(character) => character.to_string(),
        _ => token.value.clone(),
    }
}

/// Whether `token` is an operator `qual_all_Op` or `subquery_Op` may name: `Op` or `MathOp`.
pub(crate) fn is_operator_token(token: &Token) -> bool {
    match token.kind {
        TokenKind::Op | TokenKind::LessEquals | TokenKind::GreaterEquals | TokenKind::NotEquals => {
            true
        }
        TokenKind::Char(character) => "+-*/%^<>=".contains(character),
        _ => false,
    }
}

/// The node for `left op right`: SQL's arithmetic, comparison and `||`, or a generic operator.
pub(crate) fn binary_node(operator: &str, left: Expression, right: Expression) -> Expression {
    let kind: fn(Box<BinaryOp>) -> Expression = match operator {
        "+" => Expression::Add,
        "-" => Expression::Sub,
        "*" => Expression::Mul,
        "/" => Expression::Div,
        "%" => Expression::Mod,
        "<" => Expression::Lt,
        ">" => Expression::Gt,
        "=" => Expression::Eq,
        "<=" => Expression::Lte,
        ">=" => Expression::Gte,
        "<>" => Expression::Neq,
        "||" => Expression::Concat,
        _ => {
            return Expression::Operator(Box::new(Operator {
                this: Box::new(left),
                operator: Some(Box::new(Expression::Identifier(Identifier::new(operator)))),
                expression: Box::new(right),
                comments: Vec::new(),
            }));
        }
    };
    kind(Box::new(BinaryOp::new(left, right)))
}

fn unary(kind: fn(Box<UnaryOp>) -> Expression, this: Expression) -> Expression {
    kind(Box::new(UnaryOp::new(this)))
}

/// `left [NOT] LIKE|ILIKE right` before its `ESCAPE` or quantifier.
struct LikeTest {
    ilike: bool,
    negated: bool,
    left: Expression,
    right: Expression,
}

fn like(test: LikeTest, escape: Option<Expression>, quantifier: Option<String>) -> Expression {
    let LikeTest {
        ilike,
        negated,
        left,
        right,
    } = test;
    let node = Box::new(LikeOp {
        negated,
        left,
        right,
        escape,
        quantifier,
        inferred_type: None,
        span: SourceSpan(None),
    });
    match ilike {
        true => Expression::ILike(node),
        false => Expression::Like(node),
    }
}

impl Parser<'_> {
    pub(crate) fn a_expr(&mut self) -> Parsing<Expression> {
        self.climb(0, Flavor::A)
    }

    pub(crate) fn b_expr(&mut self) -> Parsing<Expression> {
        self.climb(0, Flavor::B)
    }

    /// `expr_list`: `a_expr, ...`.
    pub(crate) fn expr_list(&mut self) -> Parsing<Vec<Expression>> {
        self.comma_list(|parser| parser.a_expr())
    }

    /// Operators binding at `min` or tighter; a non-associative level repeated is a syntax error.
    pub(crate) fn climb(&mut self, min: u8, flavor: Flavor) -> Parsing<Expression> {
        self.deeper(|parser| {
            let first: usize = parser.at;
            let operand: Expression = parser.prefix(flavor)?;
            let mut left: Expression = parser.spanned(first, operand);
            let mut pending: Option<u8> = None;
            while let Some(level) = parser.infix_level(flavor) {
                if level < min {
                    break;
                }
                if pending == Some(level) {
                    return Err(parser.error());
                }
                let (node, nonassociative) = parser.infix(left, level, flavor)?;
                left = parser.spanned(first, node);
                pending = nonassociative.then_some(level);
            }
            Ok(left)
        })
    }

    /// A prefix operator and its operand, `DEFAULT`, or a primary expression.
    fn prefix(&mut self, flavor: Flavor) -> Parsing<Expression> {
        match self.peek().map(|token| token.kind) {
            Some(TokenKind::Char('-')) => {
                self.at += 1;
                Ok(unary(Expression::Neg, self.climb(UNARY, flavor)?))
            }
            Some(TokenKind::Char('+')) => {
                self.at += 1;
                Ok(unary(Expression::Pos, self.climb(UNARY, flavor)?))
            }
            Some(TokenKind::Op) => {
                let operator: String = self.advance()?.value;
                self.prefix_operator(operator, flavor)
            }
            Some(TokenKind::Keyword(keyword)) => match keyword.name {
                "not" if flavor == Flavor::A => {
                    self.at += 1;
                    Ok(unary(Expression::Not, self.climb(NOT, flavor)?))
                }
                "operator" if self.char_at(1, '(') => {
                    self.at += 1;
                    let operator: String = self.operator_name()?;
                    self.prefix_operator(operator, flavor)
                }
                "default" if flavor == Flavor::A => {
                    self.at += 1;
                    Ok(Expression::Var(Box::new(Var {
                        this: "DEFAULT".to_owned(),
                    })))
                }
                "unique" if flavor == Flavor::A => self.unique_predicate(),
                _ => self.c_expr(flavor),
            },
            _ => self.c_expr(flavor),
        }
    }

    /// `qual_Op a_expr`: `~` is bitwise not; any other prefix operator is an unquoted call of it.
    fn prefix_operator(&mut self, operator: String, flavor: Flavor) -> Parsing<Expression> {
        let operand: Expression = self.climb(OPERATOR + 1, flavor)?;
        if operator == BITWISE_NOT {
            return Ok(unary(Expression::BitwiseNot, operand));
        }
        Ok(Expression::Function(Box::new(function(
            vec![Identifier::new(operator)],
            vec![operand],
        ))))
    }

    /// `UNIQUE [NULLS [NOT] DISTINCT] (query)`, which PostgreSQL's grammar refuses as unsupported.
    fn unique_predicate(&mut self) -> Parsing<Expression> {
        let start: usize = self.offset();
        self.at += 1;
        if self.eat_kw("nulls") {
            self.eat_kw("not");
            self.expect_kw("distinct")?;
        }
        self.select_with_parens()?;
        Err(crate::parse::errors::ParseError {
            message: "UNIQUE predicate is not yet implemented".to_owned(),
            position: start,
            syntax: false,
        })
    }

    /// `OPERATOR ( [ColId .]... op )`, after the keyword: `schema.op`, a dotted schema quoted.
    pub(crate) fn operator_name(&mut self) -> Parsing<String> {
        self.expect_char('(')?;
        let mut parts: Vec<Identifier> = Vec::new();
        while self.is(is_col_id) && self.char_at(1, '.') {
            parts.push(self.name(is_col_id)?);
            self.at += 1;
        }
        let operator: Token = self.expect_token(is_operator_token)?;
        parts.push(Identifier::new(operator_text(&operator)));
        self.expect_char(')')?;
        Ok(name_text(&parts))
    }

    /// The level of the operator at the cursor, if one continues the expression here.
    fn infix_level(&self, flavor: Flavor) -> Option<u8> {
        let token: &Token = self.peek()?;
        match token.kind {
            TokenKind::Char('<' | '>' | '=')
            | TokenKind::LessEquals
            | TokenKind::GreaterEquals
            | TokenKind::NotEquals => Some(COMPARE),
            TokenKind::Char('+' | '-') => Some(ADD),
            TokenKind::Char('*' | '/' | '%') => Some(MULTIPLY),
            TokenKind::Char('^') => Some(EXPONENT),
            TokenKind::Op => Some(OPERATOR),
            TokenKind::Typecast => Some(TYPECAST),
            TokenKind::Keyword(keyword) => self.keyword_level(keyword.name, flavor),
            _ => None,
        }
    }

    /// The level of keyword operator `name`, when what follows can complete it.
    fn keyword_level(&self, name: &str, flavor: Flavor) -> Option<u8> {
        let a: bool = flavor == Flavor::A;
        let quantified =
            || matches!(self.keyword_at(1), Some("any" | "some" | "all")) && self.char_at(2, '(');
        match name {
            "operator" if self.char_at(1, '(') => Some(OPERATOR),
            "is" if self.is_continues(flavor) => Some(IS),
            "isnull" | "notnull" if a => Some(IS),
            "or" if a && self.starts_expression_at(1) => Some(OR),
            "and" if a && self.starts_expression_at(1) => Some(AND),
            "not"
                if a && matches!(
                    self.keyword_at(1),
                    Some("between" | "in" | "like" | "ilike" | "similar")
                ) =>
            {
                Some(LIKE)
            }
            "between"
                if a && (self.starts_expression_at(1)
                    || matches!(self.keyword_at(1), Some("symmetric" | "asymmetric"))) =>
            {
                Some(LIKE)
            }
            "in" if a && self.char_at(1, '(') => Some(LIKE),
            "like" | "ilike" if a && (self.starts_expression_at(1) || quantified()) => Some(LIKE),
            "similar" if a && self.kw_at(1, "to") => Some(LIKE),
            "at" if a
                && ((self.kw_at(1, "time") && self.kw_at(2, "zone")) || self.kw_at(1, "local")) =>
            {
                Some(AT)
            }
            "collate" if a && self.peek_at(1).is_some_and(is_col_id) => Some(COLLATE),
            _ => None,
        }
    }

    /// Whether `IS` at the cursor begins a test this flavor has.
    fn is_continues(&self, flavor: Flavor) -> bool {
        let offset: usize = 1 + usize::from(self.kw_at(1, "not"));
        let word: Option<&str> = self.keyword_at(offset);
        match flavor {
            Flavor::B => matches!(word, Some("distinct" | "document")),
            Flavor::A => matches!(
                word,
                Some(
                    "null"
                        | "true"
                        | "false"
                        | "unknown"
                        | "distinct"
                        | "document"
                        | "normalized"
                        | "nfc"
                        | "nfd"
                        | "nfkc"
                        | "nfkd"
                        | "json"
                )
            ),
        }
    }

    /// The operator at the cursor applied to `left`, and whether it is non-associative.
    fn infix(
        &mut self,
        left: Expression,
        level: u8,
        flavor: Flavor,
    ) -> Parsing<(Expression, bool)> {
        let start: usize = self.offset();
        let token: Token = self.advance()?;
        let TokenKind::Keyword(keyword) = token.kind else {
            if token.kind == TokenKind::Typecast {
                let to = self.typename()?;
                return Ok((cast(left, to, true), false));
            }
            return self.binary_or_quantified(left, operator_text(&token), (level, flavor));
        };
        match keyword.name {
            "or" => Ok((
                binary(Expression::Or, left, self.climb(OR + 1, flavor)?),
                false,
            )),
            "and" => Ok((
                binary(Expression::And, left, self.climb(AND + 1, flavor)?),
                false,
            )),
            "is" => self.after_is(left, flavor),
            "isnull" => Ok((null_test(left, false, true), false)),
            "notnull" => Ok((null_test(left, true, true), false)),
            "not" => self.like_family(left, true, start),
            "between" | "in" | "like" | "ilike" | "similar" => {
                self.at -= 1;
                self.like_family(left, false, start)
            }
            "at" => {
                if self.eat_kw("local") {
                    let name: Vec<Identifier> = vec![
                        Identifier::new(CATALOGUE_SCHEMA),
                        Identifier::new(TIMEZONE_FUNCTION),
                    ];
                    let call: Function = function(name, vec![left]);
                    return Ok((Expression::Function(Box::new(call)), false));
                }
                self.expect_kw("time")?;
                self.expect_kw("zone")?;
                let zone: Expression = self.climb(AT + 1, flavor)?;
                Ok((
                    Expression::AtTimeZone(Box::new(AtTimeZone { this: left, zone })),
                    false,
                ))
            }
            "collate" => {
                let name: Vec<Identifier> = self.qualified_name()?;
                let quoted: bool = name.iter().any(|part| part.quoted);
                let collation: String = name_text(&name);
                Ok((
                    Expression::Collation(Box::new(CollationExpr {
                        this: left,
                        collation,
                        quoted: false,
                        double_quoted: quoted,
                    })),
                    false,
                ))
            }
            _ => {
                let operator: String = self.operator_name()?;
                self.binary_or_quantified(left, operator, (level, flavor))
            }
        }
    }

    /// `left op right`, or `left op ANY|SOME|ALL (...)`, at `precedence` (level, flavor).
    fn binary_or_quantified(
        &mut self,
        left: Expression,
        operator: String,
        precedence: (u8, Flavor),
    ) -> Parsing<(Expression, bool)> {
        let (level, flavor) = precedence;
        if flavor == Flavor::A
            && matches!(self.keyword_at(0), Some("any" | "some" | "all"))
            && self.char_at(1, '(')
        {
            let quantifier: String = self.advance()?.value;
            let operand: Expression = self.sublink_operand()?;
            let op: Option<QuantifiedOp> = match operator.as_str() {
                "=" => Some(QuantifiedOp::Eq),
                "<>" => Some(QuantifiedOp::Neq),
                "<" => Some(QuantifiedOp::Lt),
                "<=" => Some(QuantifiedOp::Lte),
                ">" => Some(QuantifiedOp::Gt),
                ">=" => Some(QuantifiedOp::Gte),
                _ => None,
            };
            let this: Expression = match op {
                Some(_) => left,
                None => Expression::WithOperator(Box::new(WithOperator {
                    this: Box::new(left),
                    op: operator,
                })),
            };
            let node = Box::new(QuantifiedExpr {
                this,
                subquery: operand,
                op,
            });
            let quantified: Expression = match quantifier.as_str() {
                "all" => Expression::All(node),
                _ => Expression::Any(node),
            };
            return Ok((quantified, false));
        }
        let right: Expression = self.climb(level + 1, flavor)?;
        Ok((binary_node(&operator, left, right), level == COMPARE))
    }

    /// The parenthesized operand of `IN` or `op ANY`: a subquery, or `( a_expr )`.
    fn sublink_operand(&mut self) -> Parsing<Expression> {
        if self.starts_query_at(1) {
            let reached: usize = match self.attempt(|parser| parser.select_with_parens()) {
                Ok(query) => return Ok(subquery(query, None, false)),
                Err((error, _)) => error.position,
            };
            return self
                .attempt(|parser| {
                    parser.expect_char('(')?;
                    let operand: Expression = parser.a_expr()?;
                    parser.expect_char(')')?;
                    Ok(operand)
                })
                .map_err(|(error, _)| match error.position >= reached {
                    true => error,
                    false => self.error_at_position(reached),
                });
        }
        self.expect_char('(')?;
        let operand: Expression = self.a_expr()?;
        self.expect_char(')')?;
        Ok(operand)
    }

    /// `[NOT] BETWEEN`, `IN`, `LIKE`, `ILIKE` or `SIMILAR TO`, the keyword at the cursor.
    fn like_family(
        &mut self,
        left: Expression,
        negated: bool,
        start: usize,
    ) -> Parsing<(Expression, bool)> {
        let word: &str = self.keyword_at(0).unwrap_or_default();
        self.at += 1;
        match word {
            "between" => {
                let symmetric: Option<bool> =
                    match (self.eat_kw("symmetric"), self.eat_kw("asymmetric")) {
                        (true, _) => Some(true),
                        (false, true) => Some(false),
                        _ => None,
                    };
                let low: Expression = self.b_expr()?;
                self.expect_kw("and")?;
                let high: Expression = self.climb(LIKE + 1, Flavor::A)?;
                Ok((between(left, (low, high), negated, symmetric), true))
            }
            "in" => {
                if self.starts_query_at(1)
                    && let Ok(query) = self.attempt(|parser| parser.select_with_parens())
                {
                    return Ok((in_list(left, Vec::new(), Some(query), negated), false));
                }
                self.expect_char('(')?;
                let list: Vec<Expression> = self.expr_list()?;
                self.expect_char(')')?;
                Ok((in_list(left, list, None, negated), false))
            }
            "like" | "ilike" => {
                let ilike: bool = word == ILIKE;
                if matches!(self.keyword_at(0), Some("any" | "some" | "all"))
                    && self.char_at(1, '(')
                {
                    let quantifier: String = match self.advance()?.value.as_str() {
                        "all" => "ALL".to_owned(),
                        _ => "ANY".to_owned(),
                    };
                    let operand: Expression = self.sublink_operand()?;
                    return Ok((
                        like(
                            LikeTest {
                                ilike,
                                negated,
                                left,
                                right: operand,
                            },
                            None,
                            Some(quantifier),
                        ),
                        false,
                    ));
                }
                let right: Expression = self.climb(LIKE + 1, Flavor::A)?;
                let escape: Option<Expression> = self.escape()?;
                let test: LikeTest = LikeTest {
                    ilike,
                    negated,
                    left,
                    right,
                };
                Ok((like(test, escape, None), true))
            }
            "similar" => {
                self.expect_kw("to")?;
                let pattern: Expression = self.climb(LIKE + 1, Flavor::A)?;
                let escape: Option<Expression> = self.escape()?;
                Ok((
                    Expression::SimilarTo(Box::new(SimilarToExpr {
                        this: left,
                        pattern,
                        escape,
                        not: negated,
                    })),
                    true,
                ))
            }
            _ => {
                self.at = self.at.saturating_sub(1);
                let _ = start;
                Err(self.error())
            }
        }
    }

    /// `ESCAPE a_expr`, when present.
    fn escape(&mut self) -> Parsing<Option<Expression>> {
        if self.kw("escape") && self.starts_expression_at(1) {
            self.at += 1;
            return Ok(Some(self.climb(LIKE + 1, Flavor::A)?));
        }
        Ok(None)
    }

    /// The test after `IS`: `[NOT] NULL|TRUE|FALSE|UNKNOWN|DOCUMENT|JSON ...|[form] NORMALIZED`
    /// or `[NOT] DISTINCT FROM`.
    fn after_is(&mut self, left: Expression, flavor: Flavor) -> Parsing<(Expression, bool)> {
        let not: bool = self.eat_kw("not");
        let word: &str = self.keyword_at(0).unwrap_or_default();
        self.at += 1;
        let truth = |value: bool, this: Expression| {
            let node = Box::new(IsTrueFalse { this, not });
            match value {
                true => Expression::IsTrue(node),
                false => Expression::IsFalse(node),
            }
        };
        match word {
            "null" => Ok((null_test(left, not, false), false)),
            "unknown" => {
                let test: &str = match not {
                    true => NOT_UNKNOWN_TEST,
                    false => UNKNOWN_TEST,
                };
                let right: Expression = Expression::Var(Box::new(Var {
                    this: test.to_owned(),
                }));
                Ok((binary(Expression::Is, left, right), false))
            }
            "true" => Ok((truth(true, left), false)),
            "false" => Ok((truth(false, left), false)),
            "distinct" => {
                self.expect_kw("from")?;
                let right: Expression = self.climb(IS + 1, flavor)?;
                let kind: fn(Box<BinaryOp>) -> Expression = match not {
                    true => Expression::NullSafeEq,
                    false => Expression::NullSafeNeq,
                };
                Ok((binary(kind, left, right), true))
            }
            "document" => Ok((negated(not, keyword_call("IS DOCUMENT", vec![left], Vec::new())), false)),
            "normalized" => Ok((negated(not, normalized(left, None)), false)),
            form @ ("nfc" | "nfd" | "nfkc" | "nfkd") => {
                self.expect_kw("normalized")?;
                Ok((negated(not, normalized(left, Some(form))), false))
            }
            _ => {
                self.at -= 1;
                self.expect_kw("json")?;
                let json_type: Option<String> = self.json_predicate_type();
                let unique_keys: Option<JsonUniqueKeys> =
                    self.json_key_uniqueness().then_some(JsonUniqueKeys::With);
                let node: Expression = Expression::IsJson(Box::new(IsJson {
                    this: left,
                    json_type,
                    unique_keys,
                    negated: false,
                }));
                Ok((negated(not, node), false))
            }
        }
    }

    /// `json_predicate_type_constraint` after `JSON`: `VALUE` is `JSON` alone (`JS_TYPE_ANY`).
    fn json_predicate_type(&mut self) -> Option<String> {
        let kind: &str = match self.keyword_at(0) {
            Some("value") => {
                self.at += 1;
                return None;
            }
            Some(kind @ ("array" | "object" | "scalar")) => kind,
            _ => return None,
        };
        self.at += 1;
        Some(kind.to_uppercase())
    }

    /// A syntax error at a byte position some failed alternative reached.
    pub(crate) fn error_at_position(&self, position: usize) -> crate::parse::errors::ParseError {
        match self.tokens.iter().find(|token| token.start == position) {
            Some(token) => crate::parse::errors::ParseError {
                message: format!(
                    "syntax error at or near \"{}\"",
                    &self.sql[token.start..token.end]
                ),
                position,
                syntax: true,
            },
            None => crate::parse::errors::ParseError {
                message: "syntax error at end of input".to_owned(),
                position,
                syntax: true,
            },
        }
    }
}
