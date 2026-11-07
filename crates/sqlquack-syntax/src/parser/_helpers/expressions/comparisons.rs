//! Parser expressions: comparisons.

use crate::expressions::functions::models::EuclideanDistance;
use crate::expressions::functions::models::Function;
use crate::expressions::functions::models::Operator;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::operators::models::Between;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::Cast;
use crate::expressions::operators::models::In;
use crate::expressions::operators::models::IsJson;
use crate::expressions::operators::models::IsNull;
use crate::expressions::operators::models::IsTrueFalse;
use crate::expressions::operators::models::JsonUniqueKeys;
use crate::expressions::operators::models::LikeOp;
use crate::expressions::operators::models::OverlapsExpr;
use crate::expressions::operators::models::QuantifiedExpr;
use crate::expressions::operators::models::QuantifiedOp;
use crate::expressions::operators::models::SimilarToExpr;
use crate::expressions::operators::models::UnaryOp;
use crate::expressions::statements::models::Annotated;
use crate::expressions::statements::models::Paren;
use crate::expressions::statements::models::Raw;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::main::new_unary_func::new_unary_func;
use crate::expressions::typed_functions::models::AtTimeZone;
use crate::expressions::typed_functions::models::BinaryFunc;
use crate::expressions::typed_functions::models::JsonExtractFunc;
use crate::expressions::typed_functions::models::ModelAttribute;
use crate::expressions::typed_functions::models::RegexpFunc;
use crate::expressions::typed_functions::models::RegexpILike;
use crate::expressions::typed_functions::models::UnaryFunc;
use crate::expressions::types::models::DataType;
use crate::parser::constants::INTEGER;
use crate::parser::constants::SQLITE_RANGE_FOLLOWERS;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    #[inline(never)]
    pub(crate) fn parse_comparison_tail(
        &mut self,
        mut left: Expression,
        pre_left_comments: Vec<String>,
    ) -> Result<Expression> {
        let has_comparison_op = !self.is_at_end()
            && matches!(
                self.peek().token_type,
                TokenType::Eq
                    | TokenType::Neq
                    | TokenType::Lt
                    | TokenType::Gt
                    | TokenType::Lte
                    | TokenType::Gte
                    | TokenType::Is
                    | TokenType::In
                    | TokenType::Not
                    | TokenType::Between
                    | TokenType::Like
                    | TokenType::ILike
                    | TokenType::RLike
                    | TokenType::SimilarTo
            );

        if !pre_left_comments.is_empty() {
            if has_comparison_op {
                match &mut left {
                    Expression::Column(col) => {
                        col.trailing_comments.extend(pre_left_comments);
                    }
                    Expression::Identifier(id) => {
                        id.trailing_comments.extend(pre_left_comments);
                    }
                    _ => {
                        left = Expression::Annotated(Box::new(Annotated {
                            this: left,
                            trailing_comments: pre_left_comments,
                            type_assertion: None,
                            type_like: None,
                        }));
                    }
                }
            } else {
                self.pending_leading_comments = pre_left_comments;
            }
        }

        loop {
            let global_in = false;

            let expr = {
                if self.match_token(TokenType::Eq) {
                    self.parse_quantified_comparison(left, QuantifiedOp::Eq, Expression::Eq)?
                } else if self.match_token(TokenType::Neq) {
                    self.parse_quantified_comparison(left, QuantifiedOp::Neq, Expression::Neq)?
                } else if self.match_token(TokenType::Lt) {
                    self.parse_quantified_comparison(left, QuantifiedOp::Lt, Expression::Lt)?
                } else if self.match_token(TokenType::Lte) {
                    self.parse_quantified_comparison(left, QuantifiedOp::Lte, Expression::Lte)?
                } else if self.match_token(TokenType::Gt) {
                    self.parse_quantified_comparison(left, QuantifiedOp::Gt, Expression::Gt)?
                } else if self.match_token(TokenType::Gte) {
                    self.parse_quantified_comparison(left, QuantifiedOp::Gte, Expression::Gte)?
                } else if self.match_token(TokenType::NullsafeEq) {
                    let right = self.parse_bitwise_or()?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Expression::NullSafeEq(Box::new(BinaryOp {
                        left,
                        right,
                        left_comments: Vec::new(),
                        operator_comments: Vec::new(),
                        trailing_comments,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }))
                } else if self.check_identifier("SOUNDS") && self.check_next(TokenType::Like) {
                    self.parse_sounds_like_comparison(left)?
                } else if self.match_token(TokenType::Like) {
                    self.parse_like_comparison(left)?
                } else if self.match_token(TokenType::ILike) {
                    self.parse_ilike_comparison(left)?
                } else if self.check_identifier("SIMILAR") && self.check_next(TokenType::To) {
                    self.parse_similar_to_comparison(left)?
                } else if self.match_token(TokenType::Glob) {
                    let right = self.parse_bitwise_or()?;
                    Expression::Glob(Box::new(BinaryOp::new(left, right)))
                } else if self.match_token(TokenType::Match) {
                    let right = self.parse_bitwise_or()?;
                    Expression::Match(Box::new(BinaryOp::new(left, right)))
                } else if self.match_token(TokenType::RLike) || self.match_token(TokenType::Tilde) {
                    let right = self.parse_bitwise_or()?;
                    Expression::RegexpLike(Box::new(RegexpFunc {
                        options: None,

                        this: left,
                        pattern: right,
                        flags: None,
                    }))
                } else {
                    let (expr, matched) = self.parse_comparison_operator(left, global_in)?;
                    if !matched {
                        return Ok(expr);
                    }
                    expr
                }
            };

            left = match self.config.dialect {
                Some(crate::tokens::models::DialectType::SQLite) => self.sqlite_negated_in(expr),
                _ => expr,
            };
        }
    }

    /// Rewrites SQLite's `x NOT IN (...)` as `NOT (x IN (...))`, parenthesized before a comparison.
    fn sqlite_negated_in(&self, expr: Expression) -> Expression {
        let expr: Expression = match expr {
            Expression::In(mut in_expr) if in_expr.not => {
                in_expr.not = false;
                Expression::Not(Box::new(UnaryOp::new(Expression::In(in_expr))))
            }
            other => other,
        };
        let negated_in: bool =
            matches!(&expr, Expression::Not(op) if matches!(&op.this, Expression::In(_)));
        if negated_in && SQLITE_RANGE_FOLLOWERS.contains(&self.peek().token_type) {
            return Expression::Paren(Box::new(Paren {
                this: expr,
                trailing_comments: Vec::new(),
                span: SourceSpan::default(),
            }));
        }
        expr
    }

    fn parse_comparison_operator(
        &mut self,
        left: Expression,
        global_in: bool,
    ) -> Result<(Expression, bool)> {
        let expr = if self.match_token(TokenType::IRLike) {
            let right = self.parse_bitwise_or()?;
            Expression::RegexpILike(Box::new(RegexpILike {
                this: Box::new(left),
                expression: Box::new(right),
                flag: None,
            }))
        } else if self.match_token(TokenType::NotLike) {
            self.parse_not_like_comparison(left)?
        } else if self.match_token(TokenType::NotILike) {
            self.parse_not_ilike_comparison(left)?
        } else if self.match_token(TokenType::NotRLike) {
            let right = self.parse_bitwise_or()?;
            let regexp_expr = Expression::RegexpLike(Box::new(RegexpFunc {
                options: None,

                this: left,
                pattern: right,
                flags: None,
            }));
            Expression::Not(Box::new(UnaryOp::new(regexp_expr)))
        } else if self.match_token(TokenType::NotIRLike) {
            let right = self.parse_bitwise_or()?;
            let regexp_expr = Expression::RegexpILike(Box::new(RegexpILike {
                this: Box::new(left),
                expression: Box::new(right),
                flag: None,
            }));
            Expression::Not(Box::new(UnaryOp::new(regexp_expr)))
        } else if self.check(TokenType::Is)
            && !self.is_last_expression_token(TokenType::Is)
            && self.match_token(TokenType::Is)
        {
            self.parse_is_comparison(left)?
        } else if self.match_token(TokenType::Not) {
            if self.match_token(TokenType::In) {
                self.parse_in_after_not(global_in, left)?
            } else if self.match_token(TokenType::Between) {
                self.parse_between_after_not(left)?
            } else if self.check_identifier("SOUNDS") && self.check_next(TokenType::Like) {
                self.parse_sounds_like_after_not(left)?
            } else if self.match_token(TokenType::Like) {
                self.parse_like_after_not(left)?
            } else if self.match_token(TokenType::ILike) {
                self.parse_ilike_after_not(left)?
            } else if self.check_identifier("SIMILAR") && self.check_next(TokenType::To) {
                self.parse_similar_to_after_not(left)?
            } else if self.match_token(TokenType::RLike) {
                let right = self.parse_bitwise_or()?;
                let regexp_expr = Expression::RegexpLike(Box::new(RegexpFunc {
                    options: None,

                    this: left,
                    pattern: right,
                    flags: None,
                }));
                Expression::Not(Box::new(UnaryOp::new(regexp_expr)))
            } else if self.match_token(TokenType::Null) {
                let is_null = Expression::Is(Box::new(BinaryOp::new(left, Expression::Null(Null))));
                Expression::Not(Box::new(UnaryOp::new(is_null)))
            } else {
                // A NOT that starts no negated comparison is left for the caller, not dropped.
                self.current -= 1;
                return Ok((left, false));
            }
        } else if self.match_token(TokenType::In) {
            self.parse_in_comparison(global_in, left)?
        } else if self.match_token(TokenType::Between) {
            self.parse_between_comparison(left)?
        } else if self.match_token(TokenType::Adjacent) {
            let right = self.parse_bitwise_or()?;
            Expression::Adjacent(Box::new(BinaryOp::new(left, right)))
        } else if self.check(TokenType::Overlaps)
            && self.current + 1 < self.tokens.len()
            && !matches!(
                self.tokens[self.current + 1].token_type,
                TokenType::Semicolon
                    | TokenType::Comma
                    | TokenType::From
                    | TokenType::Where
                    | TokenType::RParen
                    | TokenType::As
                    | TokenType::Join
                    | TokenType::On
                    | TokenType::OrderBy
                    | TokenType::GroupBy
                    | TokenType::Having
                    | TokenType::Limit
                    | TokenType::Union
                    | TokenType::Except
                    | TokenType::Intersect
                    | TokenType::Eof
            )
        {
            self.skip();
            let right = self.parse_bitwise_or()?;
            Expression::Overlaps(Box::new(OverlapsExpr {
                this: Some(left),
                expression: Some(right),
                left_start: None,
                left_end: None,
                right_start: None,
                right_end: None,
            }))
        } else if self.match_token(TokenType::IsNull) {
            Expression::IsNull(Box::new(IsNull {
                this: left,
                not: false,
                postfix_form: true,
                span: SourceSpan::default(),
            }))
        } else if self.match_token(TokenType::NotNull) {
            Expression::IsNull(Box::new(IsNull {
                this: left,
                not: true,
                postfix_form: true,
                span: SourceSpan::default(),
            }))
        } else if self.match_token(TokenType::AtAt) {
            let right = self.parse_bitwise_or()?;
            Expression::TsMatch(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::AtGt) {
            let right = self.parse_bitwise_or()?;
            Expression::ArrayContainsAll(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::LtAt) {
            let right = self.parse_bitwise_or()?;
            Expression::ArrayContainedBy(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::DAmp) {
            let right = self.parse_bitwise_or()?;
            Expression::ArrayOverlaps(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::QMarkAmp) {
            let right = self.parse_bitwise_or()?;
            Expression::JSONBContainsAllTopKeys(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::QMarkPipe) {
            let right = self.parse_bitwise_or()?;
            Expression::JSONBContainsAnyTopKeys(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::AtQMark) {
            let right = self.parse_bitwise_or()?;
            Expression::JSONBContains(Box::new(BinaryFunc {
                original_name: Some("@?".to_string()),
                this: left,
                expression: right,
                inferred_type: None,
            }))
        } else if self.match_token(TokenType::Parameter) {
            let right = self.parse_bitwise_or()?;
            Expression::JSONBContains(Box::new(BinaryFunc {
                original_name: Some("?".to_string()),
                this: left,
                expression: right,
                inferred_type: None,
            }))
        } else if self.match_token(TokenType::HashDash) {
            let right = self.parse_bitwise_or()?;
            Expression::JSONBDeleteAtPath(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::AmpLt) {
            let right = self.parse_bitwise_or()?;
            Expression::ExtendsLeft(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::AmpGt) {
            let right = self.parse_bitwise_or()?;
            Expression::ExtendsRight(Box::new(BinaryOp::new(left, right)))
        } else if self.match_identifier("MEMBER") {
            self.expect_token(TokenType::Of)?;
            self.expect_token(TokenType::LParen)?;
            let right = self.with_parser_depth(|parser| parser.parse_expression())?;
            self.expect_token(TokenType::RParen)?;
            Expression::MemberOf(Box::new(BinaryOp::new(left, right)))
        } else if self.match_token(TokenType::CaretAt) {
            let right = self.parse_bitwise_or()?;
            Expression::StartsWith(Box::new(BinaryFunc {
                original_name: Some("^@".to_string()),
                this: left,
                expression: right,
                inferred_type: None,
            }))
        } else if self.match_token(TokenType::LrArrow) {
            let right = self.parse_bitwise_or()?;
            Expression::EuclideanDistance(Box::new(EuclideanDistance {
                this: Box::new(left),
                expression: Box::new(right),
            }))
        } else if self.match_token(TokenType::Operator) {
            self.parse_operator_comparison(left)?
        } else {
            return Ok((left, false));
        };
        Ok((expr, true))
    }

    fn parse_sounds_like_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            self.skip();
            self.skip();
            let right = self.parse_bitwise_or()?;
            let soundex_left =
                Expression::Function(Box::new(Function::new("SOUNDEX".to_string(), vec![left])));
            let soundex_right =
                Expression::Function(Box::new(Function::new("SOUNDEX".to_string(), vec![right])));
            Expression::Eq(Box::new(BinaryOp::new(soundex_left, soundex_right)))
        })
    }

    fn parse_like_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let quantifier = if self.match_token(TokenType::Any) {
                Some("ANY".to_string())
            } else if self.match_token(TokenType::All) {
                Some("ALL".to_string())
            } else if self.match_token(TokenType::Some) {
                Some("SOME".to_string())
            } else {
                None
            };
            let right = self.parse_bitwise_or()?;
            let escape = if self.match_like_escape() {
                Some(self.parse_primary()?)
            } else {
                None
            };
            Expression::Like(Box::new(LikeOp {
                negated: false,
                left,
                right,
                escape,
                quantifier,
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        })
    }

    fn parse_ilike_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let quantifier = if self.match_token(TokenType::Any) {
                Some("ANY".to_string())
            } else if self.match_token(TokenType::All) {
                Some("ALL".to_string())
            } else if self.match_token(TokenType::Some) {
                Some("SOME".to_string())
            } else {
                None
            };
            let right = self.parse_bitwise_or()?;
            let escape = if self.match_like_escape() {
                Some(self.parse_primary()?)
            } else {
                None
            };
            Expression::ILike(Box::new(LikeOp {
                negated: false,
                left,
                right,
                escape,
                quantifier,
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        })
    }

    fn parse_similar_to_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            self.skip();
            self.skip();
            let pattern = self.parse_bitwise_or()?;
            let escape = if self.match_like_escape() {
                Some(self.parse_primary()?)
            } else {
                None
            };
            Expression::SimilarTo(Box::new(SimilarToExpr {
                this: left,
                pattern,
                escape,
                not: false,
            }))
        })
    }

    fn parse_sounds_like_after_not(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            self.skip();
            self.skip();
            let right = self.parse_bitwise_or()?;
            let soundex_left =
                Expression::Function(Box::new(Function::new("SOUNDEX".to_string(), vec![left])));
            let soundex_right =
                Expression::Function(Box::new(Function::new("SOUNDEX".to_string(), vec![right])));
            let eq_expr = Expression::Eq(Box::new(BinaryOp::new(soundex_left, soundex_right)));
            Expression::Not(Box::new(UnaryOp::new(eq_expr)))
        })
    }

    fn parse_similar_to_after_not(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            self.skip();
            self.skip();
            let pattern = self.parse_bitwise_or()?;
            let escape = if self.match_like_escape() {
                Some(self.parse_primary()?)
            } else {
                None
            };
            Expression::SimilarTo(Box::new(SimilarToExpr {
                this: left,
                pattern,
                escape,
                not: true,
            }))
        })
    }

    fn parse_not_like_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let right = self.parse_bitwise_or()?;
            let escape = if self.match_like_escape() {
                Some(self.parse_primary()?)
            } else {
                None
            };

            Expression::Like(Box::new(LikeOp {
                negated: true,
                left,
                right,
                escape,
                quantifier: None,
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        })
    }

    fn parse_not_ilike_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let right = self.parse_bitwise_or()?;
            let escape = if self.match_like_escape() {
                Some(self.parse_primary()?)
            } else {
                None
            };

            Expression::ILike(Box::new(LikeOp {
                negated: true,
                left,
                right,
                escape,
                quantifier: None,
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        })
    }

    fn parse_is_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let not = self.match_token(TokenType::Not);
            if self.match_token(TokenType::Null) {
                let expr = Expression::IsNull(Box::new(IsNull {
                    this: left,
                    not,
                    postfix_form: false,
                    span: SourceSpan::default(),
                }));
                {
                    expr
                }
            } else if self.match_token(TokenType::True) {
                Expression::IsTrue(Box::new(IsTrueFalse { this: left, not }))
            } else if self.match_token(TokenType::False) {
                Expression::IsFalse(Box::new(IsTrueFalse { this: left, not }))
            } else if self.match_token(TokenType::Distinct) {
                self.expect_token(TokenType::From)?;
                let right = self.parse_bitwise_or()?;
                if not {
                    Expression::NullSafeEq(Box::new(BinaryOp::new(left, right)))
                } else {
                    Expression::NullSafeNeq(Box::new(BinaryOp::new(left, right)))
                }
            } else if self.match_identifier("UNKNOWN") {
                Expression::IsNull(Box::new(IsNull {
                    this: left,
                    not,
                    postfix_form: false,
                    span: SourceSpan::default(),
                }))
            } else if self.match_texts(&["JSON"]) {
                let json_type = if self.match_texts(&["VALUE"]) {
                    Some("VALUE".to_string())
                } else if self.match_texts(&["SCALAR"]) {
                    Some("SCALAR".to_string())
                } else if self.match_texts(&["OBJECT"]) {
                    Some("OBJECT".to_string())
                } else if self.match_texts(&["ARRAY"]) {
                    Some("ARRAY".to_string())
                } else {
                    None
                };

                let unique_keys = if self.match_text_seq(&["WITH", "UNIQUE", "KEYS"]) {
                    Some(JsonUniqueKeys::With)
                } else if self.match_text_seq(&["WITHOUT", "UNIQUE", "KEYS"]) {
                    Some(JsonUniqueKeys::Without)
                } else if self.match_text_seq(&["UNIQUE", "KEYS"]) {
                    Some(JsonUniqueKeys::Shorthand)
                } else {
                    None
                };

                Expression::IsJson(Box::new(IsJson {
                    this: left,
                    json_type,
                    unique_keys,
                    negated: not,
                }))
            } else {
                let right = self.parse_primary()?;
                let is_expr = Expression::Is(Box::new(BinaryOp::new(left, right)));
                if not {
                    Expression::Not(Box::new(UnaryOp::new(is_expr)))
                } else {
                    is_expr
                }
            }
        })
    }

    fn parse_in_after_not(&mut self, global_in: bool, left: Expression) -> Result<Expression> {
        Ok({
            if self.check_identifier("UNNEST") {
                self.skip();
                self.expect_token(TokenType::LParen)?;
                let unnest_expr = self.with_parser_depth(|parser| parser.parse_expression())?;
                self.expect_token(TokenType::RParen)?;
                Expression::In(Box::new(In {
                    this: left,
                    expressions: Vec::new(),
                    query: None,
                    not: true,
                    global: global_in,
                    unnest: Some(Box::new(unnest_expr)),
                    is_field: false,
                    span: SourceSpan::default(),
                }))
            } else if self.match_token(TokenType::LParen) {
                if self.check(TokenType::Select) || self.check(TokenType::With) {
                    let subquery = self.parse_statement()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::In(Box::new(In {
                        this: left,
                        expressions: Vec::new(),
                        query: Some(subquery),
                        not: true,
                        global: global_in,
                        unnest: None,
                        is_field: false,
                        span: SourceSpan::default(),
                    }))
                } else if self.check(TokenType::RParen) {
                    if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake) {
                        return Err(self.parse_error("Expected a value list after IN"));
                    }
                    self.skip();
                    Expression::In(Box::new(In {
                        this: left,
                        expressions: Vec::new(),
                        query: None,
                        not: true,
                        global: global_in,
                        unnest: None,
                        is_field: false,
                        span: SourceSpan::default(),
                    }))
                } else {
                    let expressions =
                        self.with_parser_depth(|parser| parser.parse_expression_list())?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::In(Box::new(In {
                        this: left,
                        expressions,
                        query: None,
                        not: true,
                        global: global_in,
                        unnest: None,
                        is_field: false,
                        span: SourceSpan::default(),
                    }))
                }
            } else {
                let table_expr = self.parse_primary()?;
                Expression::In(Box::new(In {
                    this: left,
                    expressions: vec![table_expr],
                    query: None,
                    not: true,
                    global: global_in,
                    unnest: None,
                    is_field: true,
                    span: SourceSpan::default(),
                }))
            }
        })
    }

    fn parse_between_after_not(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let symmetric = if self.match_texts(&["SYMMETRIC"]) {
                Some(true)
            } else if self.match_texts(&["ASYMMETRIC"]) {
                Some(false)
            } else {
                None
            };
            let low = self.parse_bitwise_or()?;
            self.expect_token(TokenType::And)?;
            let high = self.parse_bitwise_or()?;
            Expression::Between(Box::new(Between {
                this: left,
                low,
                high,
                not: true,
                symmetric,
                span: SourceSpan::default(),
            }))
        })
    }

    fn parse_like_after_not(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let quantifier = if self.match_token(TokenType::Any) {
                Some("ANY".to_string())
            } else if self.match_token(TokenType::All) {
                Some("ALL".to_string())
            } else if self.match_token(TokenType::Some) {
                Some("SOME".to_string())
            } else {
                None
            };
            let right = self.parse_bitwise_or()?;
            let escape = if self.match_like_escape() {
                Some(self.parse_primary()?)
            } else {
                None
            };

            Expression::Like(Box::new(LikeOp {
                negated: true,
                left,
                right,
                escape,
                quantifier,
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        })
    }

    fn parse_ilike_after_not(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let quantifier = if self.match_token(TokenType::Any) {
                Some("ANY".to_string())
            } else if self.match_token(TokenType::All) {
                Some("ALL".to_string())
            } else if self.match_token(TokenType::Some) {
                Some("SOME".to_string())
            } else {
                None
            };
            let right = self.parse_bitwise_or()?;
            let escape = if self.match_like_escape() {
                Some(self.parse_primary()?)
            } else {
                None
            };

            Expression::ILike(Box::new(LikeOp {
                negated: true,
                left,
                right,
                escape,
                quantifier,
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        })
    }

    fn parse_in_comparison(&mut self, global_in: bool, left: Expression) -> Result<Expression> {
        Ok({
            if self.check_identifier("UNNEST") {
                self.skip();
                self.expect_token(TokenType::LParen)?;
                let unnest_expr = self.with_parser_depth(|parser| parser.parse_expression())?;
                self.expect_token(TokenType::RParen)?;
                Expression::In(Box::new(In {
                    this: left,
                    expressions: Vec::new(),
                    query: None,
                    not: false,
                    global: global_in,
                    unnest: Some(Box::new(unnest_expr)),
                    is_field: false,
                    span: SourceSpan::default(),
                }))
            } else if self.match_token(TokenType::LParen) {
                if self.check(TokenType::Select) || self.check(TokenType::With) {
                    let subquery = self.parse_statement()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::In(Box::new(In {
                        this: left,
                        expressions: Vec::new(),
                        query: Some(subquery),
                        not: false,
                        global: global_in,
                        unnest: None,
                        is_field: false,
                        span: SourceSpan::default(),
                    }))
                } else if self.check(TokenType::RParen) {
                    if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake) {
                        return Err(self.parse_error("Expected a value list after IN"));
                    }
                    self.skip();
                    Expression::In(Box::new(In {
                        this: left,
                        expressions: Vec::new(),
                        query: None,
                        not: false,
                        global: global_in,
                        unnest: None,
                        is_field: false,
                        span: SourceSpan::default(),
                    }))
                } else {
                    let expressions =
                        self.with_parser_depth(|parser| parser.parse_expression_list())?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::In(Box::new(In {
                        this: left,
                        expressions,
                        query: None,
                        not: false,
                        global: global_in,
                        unnest: None,
                        is_field: false,
                        span: SourceSpan::default(),
                    }))
                }
            } else {
                let expr = self.parse_bitwise_or()?;
                Expression::In(Box::new(In {
                    this: left,
                    expressions: vec![expr],
                    query: None,
                    not: false,
                    global: global_in,
                    unnest: None,
                    is_field: true,
                    span: SourceSpan::default(),
                }))
            }
        })
    }

    fn parse_between_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            let symmetric = if self.match_texts(&["SYMMETRIC"]) {
                Some(true)
            } else if self.match_texts(&["ASYMMETRIC"]) {
                Some(false)
            } else {
                None
            };
            let low = self.parse_bitwise_or()?;
            self.expect_token(TokenType::And)?;
            let high = self.parse_bitwise_or()?;
            Expression::Between(Box::new(Between {
                this: left,
                low,
                high,
                not: false,
                symmetric,
                span: SourceSpan::default(),
            }))
        })
    }

    fn parse_operator_comparison(&mut self, left: Expression) -> Result<Expression> {
        Ok({
            self.expect_token(TokenType::LParen)?;

            let mut op_text = String::new();
            while !self.check(TokenType::RParen) && !self.is_at_end() {
                op_text.push_str(self.peek_text());
                self.skip();
            }
            self.expect_token(TokenType::RParen)?;

            let mut comments = if self.current > 0 {
                std::mem::take(&mut self.tokens[self.current - 1].trailing_comments)
            } else {
                Vec::new()
            };
            if comments.is_empty() && !self.is_at_end() {
                comments = std::mem::take(&mut self.tokens[self.current].comments);
            }

            let right = self.parse_bitwise_or()?;

            Expression::Operator(Box::new(Operator {
                this: Box::new(left),
                operator: Some(Box::new(Expression::Identifier(Identifier::new(op_text)))),
                expression: Box::new(right),
                comments,
            }))
        })
    }

    fn parse_quantified_comparison(
        &mut self,
        left: Expression,
        op: QuantifiedOp,
        binary: fn(Box<BinaryOp>) -> Expression,
    ) -> Result<Expression> {
        Ok({
            if self.match_token(TokenType::Any) || self.match_token(TokenType::Some) {
                let was_any = self.previous_token_type() == Some(TokenType::Any);
                self.expect_token(TokenType::LParen)?;
                let inner = self.parse_quantified_operand()?;
                self.expect_token(TokenType::RParen)?;
                let subquery = if was_any {
                    self.maybe_wrap_in_subquery(inner)
                } else {
                    inner
                };
                Expression::Any(Box::new(QuantifiedExpr {
                    this: left,
                    subquery,
                    op: Some(op),
                }))
            } else if self.match_token(TokenType::All) {
                self.expect_token(TokenType::LParen)?;
                let inner = self.parse_quantified_operand()?;
                self.expect_token(TokenType::RParen)?;
                let subquery = self.maybe_wrap_in_subquery(inner);
                Expression::All(Box::new(QuantifiedExpr {
                    this: left,
                    subquery,
                    op: Some(op),
                }))
            } else {
                let right = self.parse_bitwise_or()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                binary(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments: Vec::new(),
                    operator_comments: Vec::new(),
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            }
        })
    }

    /// Parse bitwise OR expressions (|)
    #[inline(always)]
    pub(crate) fn parse_bitwise_or(&mut self) -> Result<Expression> {
        let start = self.current;
        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::PostgreSQL)
                | Some(crate::tokens::models::DialectType::SQLite)
        ) {
            let expr = self.parse_bitwise_same_precedence()?;
            return Ok(self.spanned(start, expr));
        }
        let left = self.parse_bitwise_xor()?;
        let expr = self.parse_bitwise_or_tail(left)?;
        Ok(self.spanned(start, expr))
    }

    /// Parses SQLite's `&`, `|`, `<<` and `>>` and PostgreSQL's (with `#`), one tier below arithmetic.
    pub(crate) fn parse_bitwise_same_precedence(&mut self) -> Result<Expression> {
        let expr_start = self.current;
        let postgres: bool =
            self.config.dialect == Some(crate::tokens::models::DialectType::PostgreSQL);
        let mut left = self.parse_addition()?;
        loop {
            let token = self.peek().token_type;
            if !matches!(
                token,
                TokenType::Pipe | TokenType::Amp | TokenType::LtLt | TokenType::GtGt
            ) && !(token == TokenType::Hash && postgres)
            {
                return Ok(left);
            }
            self.skip();
            if token == TokenType::LtLt && postgres && self.match_token(TokenType::DArrow) {
                let _ = self.parse_addition()?;
                left = Expression::Raw(Raw {
                    sql: self.tokens_to_sql(expr_start, self.current),
                });
                continue;
            }
            let right = self.parse_addition()?;
            let op = Box::new(BinaryOp::new(left, right));
            left = match token {
                TokenType::Pipe => Expression::BitwiseOr(op),
                TokenType::Amp => Expression::BitwiseAnd(op),
                TokenType::Hash => Expression::BitwiseXor(op),
                TokenType::LtLt => Expression::BitwiseLeftShift(op),
                _ => Expression::BitwiseRightShift(op),
            };
        }
    }

    #[inline(never)]
    pub(crate) fn parse_bitwise_or_tail(&mut self, mut left: Expression) -> Result<Expression> {
        loop {
            if self.match_token(TokenType::Pipe) {
                let right = self.parse_bitwise_xor()?;
                left = Expression::BitwiseOr(Box::new(BinaryOp::new(left, right)));
            } else {
                return Ok(left);
            }
        }
    }

    /// Continues binary operators after a column already parsed, for DuckDB's `@col` token.
    pub(crate) fn parse_bitwise_continuation(&mut self, left: Expression) -> Result<Expression> {
        let mult_result = self.parse_multiplication_continuation(left)?;
        let add_result = self.parse_addition_continuation(mult_result)?;
        let concat_result = self.parse_duckdb_concat_tail(add_result)?;
        self.parse_bitwise_or_continuation(concat_result)
    }

    /// Parse bitwise OR with an existing left expression
    pub(crate) fn parse_bitwise_or_continuation(
        &mut self,
        mut left: Expression,
    ) -> Result<Expression> {
        loop {
            if self.match_token(TokenType::Pipe) {
                let right = self.parse_bitwise_xor()?;
                left = Expression::BitwiseOr(Box::new(BinaryOp::new(left, right)));
            } else {
                return Ok(left);
            }
        }
    }

    /// Parse multiplication/division with an existing left expression
    pub(crate) fn parse_multiplication_continuation(
        &mut self,
        mut left: Expression,
    ) -> Result<Expression> {
        loop {
            let expr = if self.match_token(TokenType::Star) {
                let right = self.parse_power()?;
                Expression::Mul(Box::new(BinaryOp::new(left, right)))
            } else if self.match_duckdb_integer_division() {
                let right = self.parse_power()?;
                Expression::IntDiv(Box::new(
                    crate::expressions::typed_functions::models::BinaryFunc {
                        this: left,
                        expression: right,
                        original_name: None,
                        inferred_type: None,
                    },
                ))
            } else if self.match_token(TokenType::Slash) {
                let right = self.parse_power()?;
                Expression::Div(Box::new(BinaryOp::new(left, right)))
            } else if self.match_token(TokenType::Percent) {
                let right = self.parse_power()?;
                Expression::Mod(Box::new(BinaryOp::new(left, right)))
            } else if !self.check(TokenType::QuotedIdentifier)
                && (self.match_identifier("DIV") || self.match_token(TokenType::Div))
            {
                let matched_as_var = self.previous().token_type == TokenType::Var;
                if matched_as_var
                    && (self.is_at_end()
                        || self.check(TokenType::Semicolon)
                        || self.check(TokenType::From)
                        || self.check(TokenType::Where)
                        || self.check(TokenType::Comma)
                        || self.check(TokenType::RParen))
                {
                    self.current -= 1;
                    return Ok(left);
                }
                let right = self.parse_power()?;
                Expression::IntDiv(Box::new(
                    crate::expressions::typed_functions::models::BinaryFunc {
                        this: left,
                        expression: right,
                        original_name: None,
                        inferred_type: None,
                    },
                ))
            } else {
                return Ok(left);
            };
            left = expr;
        }
    }

    /// Parse addition/subtraction with an existing left expression
    pub(crate) fn parse_addition_continuation(
        &mut self,
        mut left: Expression,
    ) -> Result<Expression> {
        loop {
            let left_comments = self.previous_trailing_comments().to_vec();

            let expr = if self.match_token(TokenType::Plus) {
                let operator_comments = self.previous_trailing_comments().to_vec();
                let right = self.parse_at_time_zone()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Expression::Add(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments,
                    operator_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else if self.match_token(TokenType::Dash) {
                let operator_comments = self.previous_trailing_comments().to_vec();
                let right = self.parse_at_time_zone()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Expression::Sub(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments,
                    operator_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else if (self.config.dialect != Some(crate::tokens::models::DialectType::DuckDB))
                && self.match_token(TokenType::DPipe)
            {
                let operator_comments = self.previous_trailing_comments().to_vec();
                let right = self.parse_at_time_zone()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Expression::Concat(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments,
                    operator_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else if self.match_token(TokenType::DQMark) {
                let right = self.parse_at_time_zone()?;
                Expression::Coalesce(Box::new(
                    crate::expressions::typed_functions::models::VarArgFunc {
                        expressions: vec![left, right],
                        original_name: None,
                        inferred_type: None,
                    },
                ))
            } else {
                return Ok(left);
            };

            left = expr;
        }
    }

    /// Parse bitwise XOR expressions (^)
    #[inline(always)]
    pub(crate) fn parse_bitwise_xor(&mut self) -> Result<Expression> {
        let left = self.parse_bitwise_and()?;
        self.parse_bitwise_xor_tail(left)
    }

    #[inline(never)]
    pub(crate) fn parse_bitwise_xor_tail(&mut self, mut left: Expression) -> Result<Expression> {
        loop {
            let xor: TokenType = match self.config.dialect {
                Some(crate::tokens::models::DialectType::PostgreSQL) => TokenType::Hash,
                _ => TokenType::Caret,
            };
            if self.match_token(xor) {
                let right = self.parse_bitwise_and()?;
                left = Expression::BitwiseXor(Box::new(BinaryOp::new(left, right)));
            } else {
                return Ok(left);
            }
        }
    }

    /// Parse bitwise AND expressions (&)
    #[inline(always)]
    pub(crate) fn parse_bitwise_and(&mut self) -> Result<Expression> {
        let left = self.parse_shift()?;
        self.parse_bitwise_and_tail(left)
    }

    #[inline(never)]
    pub(crate) fn parse_bitwise_and_tail(&mut self, mut left: Expression) -> Result<Expression> {
        loop {
            if self.match_token(TokenType::Amp) {
                let right = self.parse_shift()?;
                left = Expression::BitwiseAnd(Box::new(BinaryOp::new(left, right)));
            } else {
                return Ok(left);
            }
        }
    }

    /// Parse shift expressions (<< and >>)
    #[inline(always)]
    pub(crate) fn parse_shift(&mut self) -> Result<Expression> {
        let expr_start = self.current;
        let left = self.parse_duckdb_concat()?;
        self.parse_shift_tail(left, expr_start)
    }

    #[inline(never)]
    pub(crate) fn parse_shift_tail(
        &mut self,
        mut left: Expression,
        expr_start: usize,
    ) -> Result<Expression> {
        loop {
            if self.match_token(TokenType::LtLt) {
                if self.config.dialect == Some(crate::tokens::models::DialectType::PostgreSQL)
                    && self.match_token(TokenType::DArrow)
                {
                    let _ = self.parse_duckdb_concat()?;
                    left = Expression::Raw(Raw {
                        sql: self.tokens_to_sql(expr_start, self.current),
                    });
                    continue;
                }
                let right = self.parse_duckdb_concat()?;
                left = Expression::BitwiseLeftShift(Box::new(BinaryOp::new(left, right)));
            } else if self.match_token(TokenType::GtGt) {
                let right = self.parse_duckdb_concat()?;
                left = Expression::BitwiseRightShift(Box::new(BinaryOp::new(left, right)));
            } else {
                return Ok(left);
            }
        }
    }

    pub(crate) fn parse_duckdb_concat(&mut self) -> Result<Expression> {
        let start = self.current;
        let left = self.parse_addition()?;
        let expr = self.parse_duckdb_concat_tail(left)?;
        Ok(self.spanned(start, expr))
    }

    pub(crate) fn parse_duckdb_concat_tail(&mut self, mut left: Expression) -> Result<Expression> {
        if self.config.dialect != Some(crate::tokens::models::DialectType::DuckDB) {
            return Ok(left);
        }
        loop {
            if self.parsing_duckdb_json_path && self.check(TokenType::DArrow) {
                return Ok(left);
            }
            if self.match_token(TokenType::DArrow) {
                let path = self.parse_addition()?;
                left = Expression::JsonExtractScalar(Box::new(JsonExtractFunc {
                    options: None,
                    source_dialect: None,
                    this: left,
                    path,
                    returning: None,
                    arrow_syntax: true,
                    hash_arrow_syntax: false,
                    wrapper_option: None,
                    quotes_option: None,
                    on_scalar_string: false,
                    on_error: None,
                }));
            } else if self.check(TokenType::DPipe) {
                let left_comments = self.previous_trailing_comments().to_vec();
                self.skip();
                let operator_comments = self.previous_trailing_comments().to_vec();
                let right = self.parse_addition()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                left = Expression::Concat(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments,
                    operator_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }));
            } else {
                return Ok(left);
            }
        }
    }

    /// Parse addition/subtraction
    #[inline(always)]
    pub(crate) fn parse_addition(&mut self) -> Result<Expression> {
        let start = self.current;
        let left = self.parse_at_time_zone()?;
        let expr = self.parse_addition_tail(left)?;
        Ok(self.spanned(start, expr))
    }

    #[inline(never)]
    pub(crate) fn parse_addition_tail(&mut self, mut left: Expression) -> Result<Expression> {
        loop {
            let left_comments = self.previous_trailing_comments().to_vec();

            let expr = if self.match_token(TokenType::Plus) {
                let operator_comments = self.previous_trailing_comments().to_vec();
                let right = self.parse_at_time_zone()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Expression::Add(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments,
                    operator_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else if self.match_token(TokenType::Dash) {
                let operator_comments = self.previous_trailing_comments().to_vec();
                let right = self.parse_at_time_zone()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Expression::Sub(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments,
                    operator_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else if (self.config.dialect != Some(crate::tokens::models::DialectType::DuckDB))
                && self.match_token(TokenType::DPipe)
            {
                let operator_comments = self.previous_trailing_comments().to_vec();
                let right = self.parse_at_time_zone()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                Expression::Concat(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments,
                    operator_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else if self.match_token(TokenType::DQMark) {
                let right = self.parse_at_time_zone()?;
                Expression::Coalesce(Box::new(
                    crate::expressions::typed_functions::models::VarArgFunc {
                        expressions: vec![left, right],
                        original_name: None,
                        inferred_type: None,
                    },
                ))
            } else {
                return Ok(left);
            };

            left = expr;
        }
    }

    /// Parse AT TIME ZONE expression
    #[inline(always)]
    pub(crate) fn parse_at_time_zone(&mut self) -> Result<Expression> {
        let expr = self.parse_multiplication()?;
        self.parse_at_time_zone_tail(expr)
    }

    #[inline(never)]
    pub(crate) fn parse_at_time_zone_tail(&mut self, mut expr: Expression) -> Result<Expression> {
        while self.check(TokenType::Var)
            && self.peek_text().eq_ignore_ascii_case("AT")
            && self
                .peek_nth(1)
                .is_some_and(|token| matches!(token.token_type, TokenType::Time | TokenType::Local))
        {
            self.skip();
            if self.check(TokenType::Time) {
                self.skip();
                if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("ZONE") {
                    self.skip();
                    let zone = self.parse_unary()?;
                    expr = Expression::AtTimeZone(Box::new(AtTimeZone { this: expr, zone }));
                } else {
                    return Err(self.parse_error("Expected ZONE after AT TIME"));
                }
            } else if self.match_token(TokenType::Local) {
                expr = Expression::AtTimeZone(Box::new(AtTimeZone {
                    this: expr,
                    zone: Expression::Identifier(Identifier::new("LOCAL")),
                }));
            } else {
                return Err(self.parse_error("Expected TIME after AT"));
            }
        }

        Ok(expr)
    }

    /// DuckDB's `//` integer-division operator is tokenized as two adjacent slashes.
    pub(crate) fn match_duckdb_integer_division(&mut self) -> bool {
        if self.config.dialect != Some(crate::tokens::models::DialectType::DuckDB)
            || !self.check(TokenType::Slash)
            || !self.check_next(TokenType::Slash)
            || self.tokens[self.current].span.end != self.tokens[self.current + 1].span.start
        {
            return false;
        }
        self.current += 2;
        true
    }

    /// Parse multiplication/division
    #[inline(always)]
    pub(crate) fn parse_multiplication(&mut self) -> Result<Expression> {
        let start = self.current;
        let left = self.parse_multiplicative_operand()?;
        let expr = self.parse_multiplication_tail(left)?;
        Ok(self.spanned(start, expr))
    }

    #[inline(always)]
    pub(crate) fn parse_multiplicative_operand(&mut self) -> Result<Expression> {
        if self.config.dialect == Some(crate::tokens::models::DialectType::SQLite) {
            self.parse_sqlite_concat()
        } else {
            self.parse_power()
        }
    }

    /// Parses SQLite's `||`, `->` and `->>`, one left-associative tier above multiplication.
    #[inline(never)]
    pub(crate) fn parse_sqlite_concat(&mut self) -> Result<Expression> {
        let mut left = self.parse_power()?;
        loop {
            let left_comments = self.previous_trailing_comments().to_vec();
            if self.match_token(TokenType::DPipe) {
                let operator_comments = self.previous_trailing_comments().to_vec();
                let right = self.parse_power()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                left = Expression::Concat(Box::new(BinaryOp {
                    left,
                    right,
                    left_comments,
                    operator_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }));
            } else if self.check(TokenType::Arrow)
                || (self.check(TokenType::DArrow) && !self.applies_snowflake_context_rules())
            {
                let scalar = self.match_token(TokenType::DArrow);
                if !scalar {
                    self.skip();
                }
                let path = self.parse_power()?;
                let extract = Box::new(JsonExtractFunc {
                    this: left,
                    path,
                    arrow_syntax: true,
                    source_dialect: None,
                    options: None,
                    returning: None,
                    hash_arrow_syntax: false,
                    wrapper_option: None,
                    quotes_option: None,
                    on_scalar_string: false,
                    on_error: None,
                });
                left = match scalar {
                    true => Expression::JsonExtractScalar(extract),
                    false => Expression::JsonExtract(extract),
                };
            } else {
                return Ok(left);
            }
        }
    }

    #[inline(never)]
    pub(crate) fn parse_multiplication_tail(&mut self, mut left: Expression) -> Result<Expression> {
        loop {
            let expr = if self.match_token(TokenType::Star) {
                let right = self.parse_multiplicative_operand()?;
                Expression::Mul(Box::new(BinaryOp::new(left, right)))
            } else if self.match_duckdb_integer_division() {
                let right = self.parse_power()?;
                Expression::IntDiv(Box::new(
                    crate::expressions::typed_functions::models::BinaryFunc {
                        this: left,
                        expression: right,
                        original_name: None,
                        inferred_type: None,
                    },
                ))
            } else if self.match_token(TokenType::Slash) {
                let right = self.parse_multiplicative_operand()?;
                Expression::Div(Box::new(BinaryOp::new(left, right)))
            } else if self.match_token(TokenType::Percent) {
                let right = self.parse_multiplicative_operand()?;
                Expression::Mod(Box::new(BinaryOp::new(left, right)))
            } else if !self.check(TokenType::QuotedIdentifier)
                && (self.match_identifier("MOD") || self.match_token(TokenType::Mod))
            {
                let right = self.parse_power()?;
                Expression::Mod(Box::new(BinaryOp::new(left, right)))
            } else if !self.check(TokenType::QuotedIdentifier)
                && (self.match_identifier("DIV") || self.match_token(TokenType::Div))
            {
                let matched_as_var = self.previous().token_type == TokenType::Var;
                if matched_as_var
                    && (self.is_at_end()
                        || self.check(TokenType::Semicolon)
                        || self.check(TokenType::From)
                        || self.check(TokenType::Where)
                        || self.check(TokenType::Comma)
                        || self.check(TokenType::RParen))
                {
                    self.current -= 1;
                    return Ok(left);
                }
                let right = self.parse_power()?;
                Expression::IntDiv(Box::new(
                    crate::expressions::typed_functions::models::BinaryFunc {
                        this: left,
                        expression: right,
                        original_name: None,
                        inferred_type: None,
                    },
                ))
            } else {
                return Ok(left);
            };

            left = expr;
        }
    }

    /// Parses `**` exponentiation; in PostgreSQL and Redshift `^` is POWER, not XOR.
    #[inline(always)]
    pub(crate) fn parse_power(&mut self) -> Result<Expression> {
        let left = self.parse_unary()?;
        self.parse_power_tail(left)
    }

    #[inline(never)]
    pub(crate) fn parse_power_tail(&mut self, mut left: Expression) -> Result<Expression> {
        loop {
            if !matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Snowflake)
            ) && self.match_token(TokenType::DStar)
            {
                let right = self.parse_unary()?;
                left = Expression::Power(Box::new(BinaryFunc {
                    original_name: Some("**".to_string()),
                    this: left,
                    expression: right,
                    inferred_type: None,
                }));
            } else if matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::DuckDB)
                    | Some(crate::tokens::models::DialectType::PostgreSQL)
            ) && self.match_token(TokenType::Caret)
            {
                let right = self.parse_unary()?;
                left = Expression::Power(Box::new(BinaryFunc {
                    original_name: None,
                    this: left,
                    expression: right,
                    inferred_type: None,
                }));
            } else {
                return Ok(left);
            }
        }
    }

    /// Parses a type literal such as `timestamp '2024-01-01'` as a cast, or `None`.
    pub(crate) fn try_parse_type_literal(&mut self) -> Result<Option<Expression>> {
        let start_pos = self.current;

        if !self.check(TokenType::Identifier) && !self.check(TokenType::Var) {
            return Ok(None);
        }

        let type_name = self.peek_text().to_ascii_uppercase();

        let is_type_literal_type = matches!(
            type_name.as_str(),
            "POINT"
                | "LINE"
                | "LSEG"
                | "BOX"
                | "PATH"
                | "POLYGON"
                | "CIRCLE"
                | "INET"
                | "CIDR"
                | "MACADDR"
                | "MACADDR8"
                | "UUID"
                | "JSON"
                | "JSONB"
                | "XML"
                | "BIT"
                | "VARBIT"
                | "INT4RANGE"
                | "INT8RANGE"
                | "NUMRANGE"
                | "TSRANGE"
                | "TSTZRANGE"
                | "DATERANGE"
        ) || self.config.dialect
            == Some(crate::tokens::models::DialectType::DuckDB)
            && DataType::from_unsigned_name(&type_name).is_some()
            || self.is_snowflake()
                && crate::parser::constants::SNOWFLAKE_LITERAL_TYPES.contains(&type_name.as_str());

        if !is_type_literal_type {
            return self.try_parse_snowflake_named_type_literal();
        }

        if self.current + 1 >= self.tokens.len() {
            return Ok(None);
        }

        if self.tokens[self.current + 1].token_type != TokenType::String {
            return Ok(None);
        }

        self.skip();

        let data_type = match self.parse_data_type_from_name(&type_name) {
            Ok(dt) => dt,
            Err(_) => {
                self.current = start_pos;
                return Ok(None);
            }
        };

        if !self.check(TokenType::String) {
            self.current = start_pos;
            return Ok(None);
        }

        let string_token = self.advance()?;
        let value = Expression::Literal(Box::new(Literal::String(string_token.text.to_string())));

        if matches!(data_type, DataType::Json | DataType::JsonB)
            || matches!(type_name.as_str(), "JSON" | "JSONB")
        {
            return Ok(Some(Expression::ParseJson(Box::new(UnaryFunc {
                this: value,
                original_name: None,
                inferred_type: None,
            }))));
        }

        Ok(Some(Expression::Cast(Box::new(Cast {
            this: value,
            to: self.normalize_dialect_data_type(data_type),
            trailing_comments: Vec::new(),
            double_colon_syntax: false,
            format: None,
            default: None,
            inferred_type: None,
            span: SourceSpan::default(),
        }))))
    }

    /// Parses a type-keyword shorthand cast such as `INT 1` or `VARCHAR 'x'` (generic mode).
    pub(crate) fn try_parse_type_shorthand_cast(&mut self) -> Result<Option<Expression>> {
        let is_generic = self.config.dialect.is_none()
            || matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Generic)
            );
        if !is_generic {
            return Ok(None);
        }

        let start_pos = self.current;

        if !self.is_type_keyword() {
            return Ok(None);
        }

        if self.current + 1 >= self.tokens.len() {
            return Ok(None);
        }

        let next_type = self.tokens[self.current + 1].token_type;
        if !matches!(next_type, TokenType::Number | TokenType::String) {
            return Ok(None);
        }

        let type_token = self.advance()?;
        let type_name = type_token.text.to_ascii_uppercase();

        let data_type = match type_name.as_str() {
            name if DataType::from_unsigned_name(name).is_some() => {
                DataType::from_unsigned_name(name).ok_or_else(|| {
                    crate::tokens::errors::Error::internal("a value the parser checked is missing")
                })?
            }
            name if self.is_int128_type_name(name) => DataType::Int128,
            "INT" | "INTEGER" => DataType::Int {
                length: None,
                integer_spelling: type_name == INTEGER,
            },
            "BIGINT" => DataType::BigInt { length: None },
            "SMALLINT" => DataType::SmallInt { length: None },
            "TINYINT" => DataType::TinyInt { length: None },
            "FLOAT" => DataType::Float {
                precision: None,
                scale: None,
                real_spelling: false,
            },
            "DOUBLE" => DataType::Double {
                precision: None,
                scale: None,
            },
            "DECIMAL" | "NUMERIC" => DataType::Decimal {
                precision: None,
                scale: None,
            },
            "REAL" => DataType::Float {
                precision: None,
                scale: None,
                real_spelling: true,
            },
            "VARCHAR" => DataType::VarChar {
                length: None,
                parenthesized_length: false,
            },
            "CHAR" => DataType::Char { length: None },
            "TEXT" | "STRING" => DataType::Text,
            "BOOLEAN" | "BOOL" => DataType::Boolean,
            "BINARY" => DataType::Binary { length: None },
            "VARBINARY" => DataType::VarBinary { length: None },
            _ => {
                self.current = start_pos;
                return Ok(None);
            }
        };

        let value = if self.check(TokenType::String) {
            let tok = self.advance()?;
            Expression::Literal(Box::new(Literal::String(tok.text.to_string())))
        } else if self.check(TokenType::Number) {
            let tok = self.advance()?;
            Expression::Literal(Box::new(Literal::Number(tok.text.to_string())))
        } else {
            self.current = start_pos;
            return Ok(None);
        };

        Ok(Some(Expression::Cast(Box::new(Cast {
            this: value,
            to: data_type,
            trailing_comments: Vec::new(),
            double_colon_syntax: false,
            format: None,
            default: None,
            inferred_type: None,
            span: SourceSpan::default(),
        }))))
    }

    /// Parse unary expressions
    pub(crate) fn parse_unary(&mut self) -> Result<Expression> {
        let start = self.current;
        let mut prefixes: Vec<TokenType> = Vec::new();
        while !self.is_at_end() {
            let token = self.peek().token_type;

            if !matches!(
                token,
                TokenType::Plus
                    | TokenType::Dash
                    | TokenType::Tilde
                    | TokenType::DPipeSlash
                    | TokenType::PipeSlash
            ) {
                break;
            }
            let _scope = self.enter_parser_depth(prefixes.len() + 1)?;
            prefixes.push(token);
            self.skip();
        }
        let _scope = self.enter_parser_depth(prefixes.len())?;
        let expr = self.parse_unary_operand()?;
        let expr = Self::apply_unary_prefixes(expr, prefixes)?;
        Ok(self.spanned(start, expr))
    }

    #[inline(never)]
    pub(crate) fn apply_unary_prefixes(
        mut expr: Expression,
        prefixes: Vec<TokenType>,
    ) -> Result<Expression> {
        for token in prefixes.into_iter().rev() {
            expr = match token {
                TokenType::Plus => Expression::Pos(Box::new(UnaryOp::new(expr))),
                TokenType::Dash => Expression::Neg(Box::new(UnaryOp::new(expr))),
                TokenType::Not => Expression::Not(Box::new(UnaryOp::new(expr))),
                TokenType::Tilde => Expression::BitwiseNot(Box::new(UnaryOp::new(expr))),
                TokenType::DPipeSlash => {
                    Expression::Cbrt(Box::new(UnaryFunc::with_name(expr, "||/".to_string())))
                }
                TokenType::PipeSlash => {
                    Expression::Sqrt(Box::new(UnaryFunc::with_name(expr, "|/".to_string())))
                }
                TokenType::DAt => Expression::Abs(Box::new(new_unary_func(expr))),
                TokenType::Exclamation => factorial(expr),
                _ => {
                    return Err(crate::tokens::errors::Error::internal(
                        "collected prefix operator",
                    ))
                }
            };
        }
        Ok(expr)
    }

    #[inline(never)]
    pub(crate) fn parse_unary_operand(&mut self) -> Result<Expression> {
        if self.check(TokenType::If) {
            let expr = self.parse_primary()?;
            return self.parse_postfix_operators(expr);
        }
        self.parse_unary_operand_special()
    }

    #[inline(never)]
    pub(crate) fn parse_unary_operand_special(&mut self) -> Result<Expression> {
        {
            {
                if self.check(TokenType::DAt)
                    && self.config.dialect == Some(crate::tokens::models::DialectType::PostgreSQL)
                {
                    self.skip();
                    let expr = self.with_parser_depth(|parser| parser.parse_bitwise_or())?;
                    Ok(Expression::Abs(Box::new(new_unary_func(expr))))
                } else if self.check(TokenType::Var)
                    && self.peek_text().starts_with('@')
                    && self.config.dialect == Some(crate::tokens::models::DialectType::PostgreSQL)
                {
                    let token = self.advance()?;
                    let col_expr = Self::parsed_column(Column {
                        name: Identifier::new(&token.text[1..]),
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    });
                    Ok(Expression::Abs(Box::new(new_unary_func(col_expr))))
                } else if self.check(TokenType::DAt)
                    && matches!(
                        self.config.dialect,
                        Some(crate::tokens::models::DialectType::DuckDB)
                    )
                {
                    self.skip();
                    let expr = self.with_parser_depth(|parser| parser.parse_bitwise_or())?;
                    Ok(Expression::Abs(Box::new(new_unary_func(expr))))
                } else if self.check(TokenType::Var)
                    && self.peek_text().starts_with('@')
                    && matches!(
                        self.config.dialect,
                        Some(crate::tokens::models::DialectType::DuckDB)
                    )
                {
                    let token = self.advance()?;
                    let col_name = &token.text[1..];

                    let col_expr = Self::parsed_column(Column {
                        name: Identifier::new(col_name),
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    });

                    if self.check(TokenType::Plus)
                        || self.check(TokenType::Dash)
                        || self.check(TokenType::Star)
                        || self.check(TokenType::Slash)
                        || self.check(TokenType::Percent)
                        || self.check(TokenType::Amp)
                        || self.check(TokenType::Pipe)
                        || self.check(TokenType::Caret)
                        || self.check(TokenType::LtLt)
                        || self.check(TokenType::GtGt)
                    {
                        let full_expr = self.with_parser_depth(|parser| {
                            parser.parse_bitwise_continuation(col_expr)
                        })?;
                        Ok(Expression::Abs(Box::new(new_unary_func(full_expr))))
                    } else {
                        Ok(Expression::Abs(Box::new(new_unary_func(col_expr))))
                    }
                } else if (self.check(TokenType::DAt))
                    && (self.check_next(TokenType::LParen) || self.check_next(TokenType::Dash))
                {
                    self.skip();
                    let expr = self.with_parser_depth(|parser| parser.parse_bitwise_or())?;
                    Ok(Expression::Abs(Box::new(new_unary_func(expr))))
                } else {
                    if let Some(type_literal) = self.try_parse_type_literal()? {
                        return self.parse_postfix_operators(type_literal);
                    }
                    if let Some(type_cast) = self.try_parse_type_shorthand_cast()? {
                        return self.parse_postfix_operators(type_cast);
                    }
                    let expr = self.parse_primary()?;
                    self.parse_postfix_operators(expr)
                }
            }
        }
    }

    /// Parse postfix operators like ! (model attribute in Snowflake) and : (JSON path in Snowflake)
    pub(crate) fn parse_postfix_operators(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.check(TokenType::LParen) && self.check_next(TokenType::Plus) {
            let saved_pos = self.current;
            if self.match_token(TokenType::LParen)
                && self.match_token(TokenType::Plus)
                && self.match_token(TokenType::RParen)
            {
                if let Expression::Column(ref mut col) = expr {
                    col.join_mark = true;
                }
            } else {
                self.current = saved_pos;
            }
        }

        while self.check(TokenType::Exclamation) {
            self.refuse_snowflake_bang_call(&expr)?;
            self.skip();
            let attr = self.parse_primary()?;
            expr = Expression::ModelAttribute(Box::new(ModelAttribute {
                this: Box::new(expr),
                expression: Box::new(attr),
            }));
        }

        expr = self.parse_colon_json_path(expr)?;

        while self.match_token(TokenType::DColon) {
            let data_type = self.parse_data_type_for_cast()?;
            expr = Expression::Cast(Box::new(Cast {
                this: expr,
                to: data_type,
                trailing_comments: Vec::new(),
                double_colon_syntax: true,
                format: None,
                default: None,
                inferred_type: None,
                span: SourceSpan::default(),
            }));
        }

        Ok(expr)
    }

    /// Parses Snowflake colon paths: `a:b.c` becomes GET_PATH(a, 'b.c'), casts included.
    pub(crate) fn parse_colon_json_path(&mut self, this: Expression) -> Result<Expression> {
        self.with_parser_depth(|parser| parser.parse_colon_json_path_inner(this))
    }
}

/// FACTORIAL(x), produced by Vertica's `x!` and `!! x` operators.
pub(crate) fn factorial(expr: Expression) -> Expression {
    Expression::Function(Box::new(Function::new("FACTORIAL".to_string(), vec![expr])))
}
