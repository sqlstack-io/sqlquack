//! `expr` (parse.y): SQLite's operator precedence, by precedence climbing.

use sqlquack_syntax::Expression;
use sqlquack_syntax::expressions::functions::models::{JSONExtract, JSONExtractScalar};
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::operators::models::{BinaryOp, CollationExpr, LikeOp, UnaryOp};
use sqlquack_syntax::expressions::typed_functions::models::RegexpFunc;
use sqlquack_syntax::tokens::models::SourceSpan;

use crate::parse::_helpers::tree::cursor::{Parser, Parsing, is_ids, is_nm};
use crate::parse::_helpers::tree::nodes::{between, binary, call, in_list, null_test, subquery};
use crate::scan::models::TokenKind;

/// `OR`.
const OR_LEVEL: u8 = 1;
/// `AND`.
const AND_LEVEL: u8 = 2;
/// The prefix `NOT`, and the operators spelled with a leading `NOT`.
const NOT_LEVEL: u8 = 3;
/// `IS`, `MATCH`, `LIKE_KW`, `BETWEEN`, `IN`, `ISNULL`, `NOTNULL`, `NE` and `EQ`.
const EQUALITY_LEVEL: u8 = 4;
/// `GT`, `LE`, `LT` and `GE`.
const COMPARISON_LEVEL: u8 = 5;
/// `ESCAPE`.
const ESCAPE_LEVEL: u8 = 6;
/// `BITAND`, `BITOR`, `LSHIFT` and `RSHIFT`.
const BITWISE_LEVEL: u8 = 7;
/// `PLUS` and `MINUS`.
const ADDITIVE_LEVEL: u8 = 8;
/// `STAR`, `SLASH` and `REM`.
const MULTIPLICATIVE_LEVEL: u8 = 9;
/// `CONCAT` and `PTR`.
const CONCAT_LEVEL: u8 = 10;
/// `COLLATE`.
const COLLATE_LEVEL: u8 = 11;
/// The prefix `~`, `+` and `-` (`BITNOT`).
const PREFIX_LEVEL: u8 = 12;

/// A binary operator's node, built from its two operands.
type Build = fn(Box<BinaryOp>) -> Expression;

/// A pattern operator as written: its word, a `NOT` before it, and its `ESCAPE`.
struct Like {
    word: String,
    not: bool,
    escape: Option<Expression>,
}

/// What one step of a chain did: extended the operand, or left it as it was.
enum Step {
    More(Expression),
    Done(Expression),
}

fn unary(kind: fn(Box<UnaryOp>) -> Expression, this: Expression) -> Expression {
    kind(Box::new(UnaryOp::new(this)))
}

fn negated(not: bool, this: Expression) -> Expression {
    match not {
        true => unary(Expression::Not, this),
        false => this,
    }
}

/// A binary operator token's level and node.
fn binary_operator(kind: TokenKind) -> Option<(u8, Build)> {
    Some(match kind {
        TokenKind::Op("EQ") => (EQUALITY_LEVEL, Expression::Eq),
        TokenKind::Op("NE") => (EQUALITY_LEVEL, Expression::Neq),
        TokenKind::Op("LT") => (COMPARISON_LEVEL, Expression::Lt),
        TokenKind::Op("LE") => (COMPARISON_LEVEL, Expression::Lte),
        TokenKind::Op("GT") => (COMPARISON_LEVEL, Expression::Gt),
        TokenKind::Op("GE") => (COMPARISON_LEVEL, Expression::Gte),
        TokenKind::Op("BITAND") => (BITWISE_LEVEL, Expression::BitwiseAnd),
        TokenKind::Op("BITOR") => (BITWISE_LEVEL, Expression::BitwiseOr),
        TokenKind::Op("LSHIFT") => (BITWISE_LEVEL, Expression::BitwiseLeftShift),
        TokenKind::Op("RSHIFT") => (BITWISE_LEVEL, Expression::BitwiseRightShift),
        TokenKind::Op("PLUS") => (ADDITIVE_LEVEL, Expression::Add),
        TokenKind::Op("MINUS") => (ADDITIVE_LEVEL, Expression::Sub),
        TokenKind::Op("STAR") => (MULTIPLICATIVE_LEVEL, Expression::Mul),
        TokenKind::Op("SLASH") => (MULTIPLICATIVE_LEVEL, Expression::Div),
        TokenKind::Op("REM") => (MULTIPLICATIVE_LEVEL, Expression::Mod),
        TokenKind::Op("CONCAT") => (CONCAT_LEVEL, Expression::Concat),
        TokenKind::Keyword("AND") => (AND_LEVEL, Expression::And),
        TokenKind::Keyword("OR") => (OR_LEVEL, Expression::Or),
        _ => return None,
    })
}

/// `left -> right` or `left ->> right`.
fn arrow(left: Expression, right: Expression, scalar: bool) -> Expression {
    match scalar {
        true => Expression::JSONExtractScalar(Box::new(JSONExtractScalar {
            this: Box::new(left),
            expression: Box::new(right),
            only_json_types: None,
            expressions: Vec::new(),
            json_type: None,
            scalar_only: None,
        })),
        false => Expression::JSONExtract(Box::new(JSONExtract {
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

/// `left [NOT] LIKE|GLOB|REGEXP|MATCH right [ESCAPE escape]`, as SQLite reads it.
fn like(operator: Like, left: Expression, right: Expression) -> Expression {
    let Like { word, not, escape } = operator;
    let word: String = word.to_ascii_uppercase();
    match (word.as_str(), escape) {
        ("LIKE", escape) => Expression::Like(Box::new(LikeOp {
            negated: not,
            left,
            right,
            escape,
            quantifier: None,
            inferred_type: None,
            span: SourceSpan::default(),
        })),
        ("GLOB", None) => negated(not, binary(Expression::Glob, left, right)),
        ("MATCH", None) => negated(not, binary(Expression::Match, left, right)),
        ("REGEXP", None) => negated(
            not,
            Expression::RegexpLike(Box::new(RegexpFunc {
                options: None,
                this: left,
                pattern: right,
                flags: None,
            })),
        ),
        (name, Some(escape)) => negated(
            not,
            call(&name.to_ascii_lowercase(), vec![right, left, escape]),
        ),
        (name, None) => negated(not, call(&name.to_ascii_lowercase(), vec![right, left])),
    }
}

impl Parser<'_> {
    /// `expr`: an expression at the lowest level.
    pub(crate) fn expression(&mut self) -> Parsing<Expression> {
        self.climb(OR_LEVEL)
    }

    /// `BETWEEN`'s lower bound: everything above `AND`, which ends it.
    pub(crate) fn above_and(&mut self) -> Parsing<Expression> {
        self.climb(NOT_LEVEL)
    }

    /// An expression whose operators bind at `level` or above; each link counts as depth.
    fn climb(&mut self, level: u8) -> Parsing<Expression> {
        let first: usize = self.at;
        let mut left: Expression = self.operand()?;
        let mut links: usize = 0;
        let result: Parsing<Expression> = loop {
            match self.infix(level, left) {
                Ok(Step::More(next)) => {
                    left = self.respanned(first, next);
                    if let Err(error) = self.lengthen() {
                        break Err(error);
                    }
                    links += 1;
                }
                Ok(Step::Done(done)) => break Ok(done),
                Err(error) => break Err(error),
            }
        };
        self.release(links);
        result
    }

    /// The operand an operator chain starts from: a prefix operator and its operand, or a term.
    fn operand(&mut self) -> Parsing<Expression> {
        let first: usize = self.at;
        let (kind, level): (fn(Box<UnaryOp>) -> Expression, u8) = match self.kind_at(0) {
            Some(TokenKind::Keyword("NOT")) => (Expression::Not, NOT_LEVEL),
            Some(TokenKind::Op("BITNOT")) => (Expression::BitwiseNot, PREFIX_LEVEL),
            Some(TokenKind::Op("PLUS")) => (Expression::Pos, PREFIX_LEVEL),
            Some(TokenKind::Op("MINUS")) => (Expression::Neg, PREFIX_LEVEL),
            _ => return self.deeper(|parser| parser.primary()),
        };
        self.at += 1;
        let this: Expression = self.deeper(|parser| parser.climb(level))?;
        Ok(self.respanned(first, unary(kind, this)))
    }

    /// The operator after `left`, when it binds at `level` or above, applied to it.
    fn infix(&mut self, level: u8, left: Expression) -> Parsing<Step> {
        let Some(kind) = self.kind_at(0) else {
            return Ok(Step::Done(left));
        };
        if let Some((found, node)) = binary_operator(kind) {
            if found < level {
                return Ok(Step::Done(left));
            }
            self.at += 1;
            let right: Expression = self.deeper(|parser| parser.climb(found + 1))?;
            return Ok(Step::More(binary(node, left, right)));
        }
        let (found, not): (u8, bool) = match kind {
            TokenKind::Op("PTR") => (CONCAT_LEVEL, false),
            TokenKind::Keyword("COLLATE") => (COLLATE_LEVEL, false),
            TokenKind::Keyword("NOT") => match self.kind_at(1) {
                Some(TokenKind::Keyword("NULL" | "LIKE_KW" | "MATCH" | "BETWEEN" | "IN")) => {
                    (NOT_LEVEL, true)
                }
                _ => {
                    self.at += 1;
                    return Err(self.error());
                }
            },
            TokenKind::Keyword(
                "IS" | "ISNULL" | "NOTNULL" | "LIKE_KW" | "MATCH" | "BETWEEN" | "IN",
            ) => (EQUALITY_LEVEL, false),
            _ => return Ok(Step::Done(left)),
        };
        if found < level {
            return Ok(Step::Done(left));
        }
        let operator = self.advance()?;
        if not {
            self.at += 1;
        }
        let keyword: &str = match (not, self.tokens[self.at - 1].kind) {
            (_, TokenKind::Keyword(keyword)) => keyword,
            _ => "PTR",
        };
        let right_level: u8 = EQUALITY_LEVEL + 1;
        Ok(Step::More(match keyword {
            "PTR" => {
                let right: Expression = self.deeper(|parser| parser.climb(CONCAT_LEVEL + 1))?;
                arrow(left, right, matches!(operator.value.as_str(), "->>"))
            }
            "COLLATE" => {
                let name = self.expect_token(is_ids)?;
                Expression::Collation(Box::new(CollationExpr {
                    this: left,
                    collation: name.value.clone(),
                    quoted: name.kind == TokenKind::String,
                    double_quoted: matches!(name.kind, TokenKind::Quoted(_)),
                }))
            }
            "NULL" => null_test(left, true),
            "ISNULL" => null_test(left, false),
            "NOTNULL" => null_test(left, true),
            "IS" => self.after_is(left)?,
            "LIKE_KW" | "MATCH" => {
                let word: String = self.tokens[self.at - 1].value.clone();
                let right: Expression = self.deeper(|parser| parser.climb(right_level))?;
                let escape: Option<Expression> = match self.eat_kw("ESCAPE") {
                    true => Some(self.deeper(|parser| parser.climb(ESCAPE_LEVEL + 1))?),
                    false => None,
                };
                like(Like { word, not, escape }, left, right)
            }
            "BETWEEN" => {
                let low: Expression = self.deeper(|parser| parser.above_and())?;
                self.expect_kw("AND")?;
                let high: Expression = self.deeper(|parser| parser.climb(right_level))?;
                between(left, low, high, not)
            }
            _ => self.in_tail(left, not)?,
        }))
    }

    /// After `IS`: `[NOT] [DISTINCT FROM] expr`; `IS [NOT] NULL` is a null test.
    fn after_is(&mut self, left: Expression) -> Parsing<Expression> {
        let not: bool = self.eat_kw("NOT");
        if self.kw("DISTINCT") {
            self.at += 1;
            self.expect_kw("FROM")?;
            let right: Expression = self.deeper(|parser| parser.climb(EQUALITY_LEVEL + 1))?;
            return Ok(match not {
                true => binary(Expression::NullSafeEq, left, right),
                false => binary(Expression::NullSafeNeq, left, right),
            });
        }
        let right: Expression = self.deeper(|parser| parser.climb(EQUALITY_LEVEL + 1))?;
        Ok(match right {
            Expression::Null(_) => null_test(left, not),
            right => negated(not, binary(Expression::Is, left, right)),
        })
    }

    /// After `IN`: `( select )`, `( exprlist )`, or a table or table function, kept as a gap.
    fn in_tail(&mut self, left: Expression, not: bool) -> Parsing<Expression> {
        if self.op("LP") && self.starts_select_at(1) {
            self.at += 1;
            let query: Expression = self.select()?;
            self.expect_op("RP")?;
            return Ok(in_list(left, Vec::new(), Some(query), not));
        }
        if self.eat_op("LP") {
            let items: Vec<Expression> = self.expression_list("RP")?;
            self.expect_op("RP")?;
            return Ok(in_list(left, items, None, not));
        }
        let start: usize = self.offset();
        let mut name: Vec<Identifier> = vec![self.name(is_nm)?];
        if self.eat_op("DOT") {
            name.push(self.name(is_nm)?);
        }
        if self.eat_op("LP") {
            self.expression_list("RP")?;
            self.expect_op("RP")?;
        }
        self.gap("in_table", start);
        let table: Expression = crate::parse::_helpers::tree::nodes::column_from(name);
        Ok(in_list(left, vec![table], None, not))
    }

    /// `exprlist`: expressions separated by commas, none when `close` comes first.
    pub(crate) fn expression_list(&mut self, close: &str) -> Parsing<Vec<Expression>> {
        match self.op(close) {
            true => Ok(Vec::new()),
            false => self.comma_list(|parser| parser.expression()),
        }
    }

    /// `( select )` as a subquery expression.
    pub(crate) fn subquery_expression(&mut self) -> Parsing<Expression> {
        self.expect_op("LP")?;
        let query: Expression = self.select()?;
        self.expect_op("RP")?;
        Ok(subquery(query, None, false))
    }
}
