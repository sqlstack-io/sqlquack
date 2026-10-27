//! Parser expressions: primary.

use crate::expressions::functions::models::Columns;
use crate::expressions::functions::models::Function;
use crate::expressions::functions::models::WithinGroup;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::names::models::Null;
use crate::expressions::names::models::Star;
use crate::expressions::names::models::UnicodeString;
use crate::expressions::operators::models::Cast;
use crate::expressions::operators::models::DotAccess;
use crate::expressions::operators::models::Exists;
use crate::expressions::operators::models::LikeOp;
use crate::expressions::operators::models::Parameter;
use crate::expressions::operators::models::ParameterStyle;
use crate::expressions::operators::models::Placeholder;
use crate::expressions::statements::models::Annotated;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::main::new_unary_func::new_unary_func;
use crate::expressions::typed_functions::models::MapConstructor;
use crate::expressions::types::models::Array;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Struct;
use crate::parser::constants::ARRAY;
use crate::parser::constants::MAP;
use crate::parser::constants::ODBC_DATE_ESCAPE;
use crate::parser::constants::ODBC_TIMESTAMP_ESCAPE;
use crate::parser::constants::ODBC_TIME_ESCAPE;
use crate::parser::constants::PYFORMAT_PARAMETER_SUFFIX;
use crate::parser::constants::SECOND_PREVIOUS_OFFSET;
use crate::parser::constants::STRUCT;
use crate::parser::models::Parser;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    #[inline(never)]
    pub(crate) fn parse_primary_slow(&mut self) -> Result<Expression> {
        if self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("APPROXIMATE") {
            if let Some(found) = self.try_parse_primary_approximate()? {
                return Ok(found);
            }
        }

        if let Some(connect_by_root) = self.try_parse_connect_by_root_expression()? {
            return Ok(connect_by_root);
        }

        if self.match_token(TokenType::LBracket) {
            return self.parse_bracket_array_literal();
        }

        if self.match_token(TokenType::LBrace) {
            return self.parse_primary_brace();
        }

        if self.match_token(TokenType::LParen) {
            let lparen_comments = self.previous_trailing_comments().to_vec();
            let set_operation_may_follow = self.current < SECOND_PREVIOUS_OFFSET
                || self.tokens[self.current - 2].token_type == TokenType::LParen;
            return self.parse_parenthesized_primary(lparen_comments, set_operation_may_follow);
        }

        if self.match_token(TokenType::Null) {
            return Ok(Expression::Null(Null));
        }

        if self.match_token(TokenType::True) {
            return Ok(Expression::Boolean(BooleanLiteral { value: true }));
        }

        if self.match_token(TokenType::False) {
            return Ok(Expression::Boolean(BooleanLiteral { value: false }));
        }

        if self.check(TokenType::Lambda) {
            if let Some(lambda) = self.parse_lambda()? {
                return Ok(lambda);
            }
        }

        if self.check(TokenType::Case) && !self.check_next(TokenType::Dot) {
            let case_expr = self.parse_case()?;
            return self.maybe_parse_over(case_expr);
        }

        if self.check(TokenType::Cast) {
            let cast_expr = self.parse_cast()?;
            return self.maybe_parse_subscript(cast_expr);
        }

        if self.check(TokenType::TryCast) {
            let cast_expr = self.parse_try_cast()?;
            return self.maybe_parse_subscript(cast_expr);
        }

        if self.check(TokenType::SafeCast) {
            let cast_expr = self.parse_safe_cast()?;
            return self.maybe_parse_subscript(cast_expr);
        }

        if self.match_token(TokenType::Exists) {
            return self.parse_primary_exists();
        }

        if self.check(TokenType::Interval) {
            if let Some(interval_expr) = self.try_parse_interval()? {
                return Ok(interval_expr);
            }
            let token = self.advance()?;
            return Ok(Expression::Identifier(
                Identifier::new(token.text).with_span(token.span),
            ));
        }

        if self.check(TokenType::Date) {
            return self.parse_primary_date_literal();
        }

        if self.check(TokenType::Time) {
            let token = self.advance()?;
            let original_text = token.text.to_string();
            if self.check(TokenType::String) {
                let str_token = self.advance()?;
                return Ok(Expression::Literal(Box::new(Literal::Time(str_token.text))));
            }
            if self.match_token(TokenType::LParen) {
                let func_expr = self.parse_typed_function(&original_text, "TIME", false)?;
                return self.maybe_parse_over(func_expr);
            }
            return self.maybe_parse_subscript(Self::parsed_column(Column {
                name: Identifier::new(original_text).with_span(token.span),
                table: None,
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            }));
        }

        if self.check(TokenType::TimestampTz)
            || self.check_keyword_text("TIMESTAMPTZ")
            || self.check_keyword_text("TIMESTAMP_TZ")
        {
            return self.parse_primary_timestamp_tz_literal();
        }

        if self.check(TokenType::Timestamp) {
            return self.parse_primary_timestamp_literal();
        }

        if self.check(TokenType::DateTime) {
            return self.parse_primary_datetime_literal();
        }

        if self.check(TokenType::Row) && self.check_next(TokenType::LParen) {
            return self.parse_primary_row();
        }

        self.parse_primary_literal()
    }

    fn parse_primary_literal(&mut self) -> Result<Expression> {
        if self.check(TokenType::Number) {
            return self.parse_primary_number();
        }

        if self.check(TokenType::String) {
            return self.parse_primary_string();
        }

        if self.check(TokenType::DollarString) {
            let token = self.advance()?;
            let literal = Expression::Literal(Box::new(Literal::DollarString(token.text)));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::TripleDoubleQuotedString) {
            let token = self.advance()?;
            let literal =
                Expression::Literal(Box::new(Literal::TripleQuotedString(token.text, '"')));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::TripleSingleQuotedString) {
            let token = self.advance()?;
            let literal =
                Expression::Literal(Box::new(Literal::TripleQuotedString(token.text, '\'')));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::NationalString) {
            let token = self.advance()?;
            let literal = Expression::Literal(Box::new(Literal::NationalString(token.text)));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::HexString) {
            let token = self.advance()?;
            let literal = Expression::Literal(Box::new(Literal::HexString(token.text)));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::HexNumber) {
            let token = self.advance()?;

            let literal = Expression::Literal(Box::new(Literal::HexNumber(token.text)));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::BitString) {
            let token = self.advance()?;
            let literal = Expression::Literal(Box::new(Literal::BitString(token.text)));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::ByteString) {
            let token = self.advance()?;
            let literal = Expression::Literal(Box::new(Literal::ByteString(token.text)));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::RawString) {
            let token = self.advance()?;
            let literal = Expression::Literal(Box::new(Literal::RawString(token.text)));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::EscapeString) {
            let token = self.advance()?;
            let literal = Expression::Literal(Box::new(Literal::EscapeString(token.text)));
            return self.maybe_parse_subscript(literal);
        }

        if self.check(TokenType::UnicodeString) {
            let token = self.advance()?;
            let this = Expression::Literal(Box::new(Literal::String(token.text)));
            let escape = if self.match_identifier("UESCAPE") {
                Some(Box::new(self.parse_primary()?))
            } else {
                None
            };
            return self.maybe_parse_subscript(Expression::UnicodeString(Box::new(
                UnicodeString {
                    this: Box::new(this),
                    escape,
                },
            )));
        }

        self.parse_primary_name_or_variable()
    }

    fn parse_primary_name_or_variable(&mut self) -> Result<Expression> {
        if self.check(TokenType::Star) {
            return self.parse_primary_star();
        }

        if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
            if let Some(found) = self.try_parse_primary_identifier_start()? {
                return Ok(found);
            }
        }

        if (self.check(TokenType::Case) || self.check(TokenType::Top))
            && self.check_next(TokenType::Dot)
        {
            return self.parse_primary_keyword_qualified_column();
        }

        if self.check(TokenType::Var)
            && self.peek_text().eq_ignore_ascii_case("BINARY")
            && !self.check_next(TokenType::LParen)
            && !self.check_next(TokenType::Dot)
            && !self.check_next(TokenType::RParen)
            && !self.check_next(TokenType::Comma)
            && !self.is_at_end()
        {
            if let Some(found) = self.try_parse_primary_binary_cast()? {
                return Ok(found);
            }
        }

        if self.check(TokenType::RLike) && self.check_next(TokenType::LParen) {
            return self.parse_primary_rlike_call();
        }

        if self.check(TokenType::Insert) && self.check_next(TokenType::LParen) {
            return self.parse_primary_insert_call();
        }

        if matches!(
            self.peek().token_type,
            TokenType::CurrentDate
                | TokenType::CurrentTimestamp
                | TokenType::CurrentTime
                | TokenType::CurrentDateTime
        ) {
            return self.parse_primary_current_datetime();
        }

        if self.is_identifier_token() && self.check_next(TokenType::String) {
            if let Some(found) = self.try_parse_primary_typed_string()? {
                return Ok(found);
            }
        }

        if self.is_identifier_token() {
            return self.parse_identifier_primary();
        }

        if self.check(TokenType::Next)
            && self.current + 2 < self.tokens.len()
            && self.tokens[self.current + 1]
                .text
                .eq_ignore_ascii_case("VALUE")
            && self.tokens[self.current + 2]
                .text
                .eq_ignore_ascii_case("FOR")
        {
            self.skip();
            if let Some(expr) = self.parse_next_value_for()? {
                return Ok(expr);
            }
        }

        if self.check(TokenType::Percent)
            && (self.check_next(TokenType::Var) || self.check_next(TokenType::LParen))
        {
            if let Some(found) = self.try_parse_primary_percent_variable()? {
                return Ok(found);
            }
        }

        if self.is_safe_keyword_as_identifier() {
            return self.parse_primary_keyword_identifier();
        }

        if self.match_token(TokenType::AtAt) {
            return self.parse_primary_session_variable();
        }

        if self.match_token(TokenType::DAt) {
            return self.parse_primary_at_variable();
        }

        if self.match_token(TokenType::QDColon) {
            let data_type = self.parse_data_type_for_cast()?;
            return Ok(Expression::Cast(Box::new(Cast {
                this: Expression::Placeholder(Placeholder { index: None }),
                to: data_type,
                trailing_comments: Vec::new(),
                double_colon_syntax: true,
                format: None,
                default: None,
                inferred_type: None,
            })));
        }

        if self.check(TokenType::Parameter) {
            let token = self.advance()?;
            if let Ok(index) = token.text.parse::<u32>() {
                let param = Expression::Parameter(Box::new(Parameter {
                    name: None,
                    index: Some(index),
                    style: ParameterStyle::Dollar,
                    quoted: false,
                    string_quoted: false,
                    expression: None,
                }));
                let result = self.parse_colon_json_path(param)?;
                return self.maybe_parse_subscript(result);
            } else {
                return Ok(Expression::Placeholder(Placeholder { index: None }));
            }
        }

        if self.match_token(TokenType::Colon) {
            return self.parse_primary_colon_parameter();
        }

        if self.match_token(TokenType::Dollar) {
            return self.parse_primary_dollar();
        }

        if self.match_token(TokenType::Percent) {
            return self.parse_primary_percent_parameter();
        }

        if (self.check(TokenType::Left)
            || self.check(TokenType::Right)
            || self.check(TokenType::Outer)
            || self.check(TokenType::Full)
            || self.check(TokenType::All)
            || self.check(TokenType::Only)
            || self.check(TokenType::Next)
            || self.check(TokenType::If))
            && self.check_next(TokenType::Dot)
        {
            return self.parse_primary_join_keyword_identifier();
        }

        if self.check(TokenType::Next) && self.check_next(TokenType::LParen) {
            return self.parse_primary_next_call();
        }

        if self.can_be_alias_keyword()
            && !self.check_next(TokenType::Join)
            && !self.check_next(TokenType::LParen)
        {
            let token = self.advance()?;
            let trailing_comments = self.previous_trailing_comments().to_vec();
            let col = Self::parsed_column(Column {
                name: Identifier::new(token.text).with_span(token.span),
                table: None,
                join_mark: false,
                trailing_comments,
                span: None,
                inferred_type: None,
            });
            return self.maybe_parse_subscript(col);
        }

        Err(self.parse_error(format!("Unexpected token: {:?}", self.peek().token_type)))
    }

    fn try_parse_primary_approximate(&mut self) -> Result<Option<Expression>> {
        let saved_pos = self.current;
        self.skip();

        if self.check(TokenType::Var)
            && matches!(
                self.peek_text().to_ascii_uppercase().as_str(),
                "PERCENTILE_DISC" | "PERCENTILE_CONT"
            )
        {
            let name = self.advance_text()?.to_ascii_uppercase();
            self.expect_token(TokenType::LParen)?;
            let args = if self.check(TokenType::RParen) {
                Vec::new()
            } else {
                self.parse_expression_list()?
            };
            self.expect_token(TokenType::RParen)?;
            let func = Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: format!("APPROXIMATE {}", name),
                args,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }));

            if self.match_token(TokenType::Within) {
                self.expect_token(TokenType::Group)?;
                self.expect_token(TokenType::LParen)?;
                self.expect_token(TokenType::Order)?;
                self.expect_token(TokenType::By)?;
                let order_by = self.parse_order_by_list()?;
                self.expect_token(TokenType::RParen)?;
                return Ok(Some(Expression::WithinGroup(Box::new(WithinGroup {
                    this: func,
                    order_by,
                }))));
            }

            return Ok(Some(func));
        }

        let func = self.parse_primary()?;
        if let Expression::Count(ref count_expr) = func {
            if count_expr.distinct {
                let this_expr = count_expr.this.clone().unwrap_or_else(|| {
                    Expression::Star(crate::expressions::names::models::Star {
                        table: None,
                        except: None,
                        replace: None,
                        rename: None,
                        trailing_comments: Vec::new(),
                        span: None,
                    })
                });
                return Ok(Some(Expression::ApproxDistinct(Box::new(
                    crate::expressions::typed_functions::models::AggFunc {
                        this: this_expr,
                        distinct: false,
                        filter: None,
                        order_by: Vec::new(),
                        name: Some("APPROX_DISTINCT".to_string()),
                        ignore_nulls: None,
                        having_max: None,
                        limit: None,
                        inferred_type: None,
                    },
                ))));
            }
        }
        self.current = saved_pos;

        Ok(None)
    }

    fn parse_primary_brace(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::RBrace) {
            return self.maybe_parse_subscript(Expression::MapFunc(Box::new(MapConstructor {
                keys: Vec::new(),
                values: Vec::new(),
                curly_brace_syntax: true,
                with_map_keyword: false,
            })));
        }

        if self.check_identifier("fn") {
            self.skip();
            let func_name = self.expect_identifier_or_keyword_with_quoted()?;
            self.expect_token(TokenType::LParen)?;

            let mut args: Vec<Expression> = Vec::new();
            if !self.check(TokenType::RParen) {
                loop {
                    args.push(self.parse_expression()?);
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
            }
            self.expect_token(TokenType::RParen)?;
            self.expect_token(TokenType::RBrace)?;

            return Ok(Expression::Function(Box::new(Function::new(
                func_name.name,
                args,
            ))));
        }

        if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
            let type_text = self.peek_text().to_lowercase();
            if (type_text == ODBC_DATE_ESCAPE
                || type_text == ODBC_TIME_ESCAPE
                || type_text == ODBC_TIMESTAMP_ESCAPE)
                && self.check_next(TokenType::String)
            {
                self.skip();
                let value = self.expect_string()?;
                self.expect_token(TokenType::RBrace)?;

                return match type_text.as_str() {
                    "d" => Ok(Expression::Date(Box::new(new_unary_func(
                        Expression::Literal(Box::new(
                            crate::expressions::names::models::Literal::String(value),
                        )),
                    )))),
                    "t" => Ok(Expression::Time(Box::new(new_unary_func(
                        Expression::Literal(Box::new(
                            crate::expressions::names::models::Literal::String(value),
                        )),
                    )))),
                    "ts" => Ok(Expression::Timestamp(Box::new(
                        crate::expressions::functions::models::TimestampFunc {
                            this: Some(Box::new(Expression::Literal(Box::new(
                                crate::expressions::names::models::Literal::String(value),
                            )))),
                            zone: None,
                            with_tz: None,
                            safe: None,
                        },
                    ))),
                    _ => {
                        Err(self.parse_error(format!("Unknown ODBC datetime type: {}", type_text)))
                    }
                };
            }
        }

        let is_table_star = (self.check(TokenType::Identifier) || self.check(TokenType::Var))
            && self.check_next(TokenType::Dot)
            && self
                .tokens
                .get(self.current + 2)
                .map(|t| t.token_type == TokenType::Star)
                .unwrap_or(false);
        let is_wildcard = self.check(TokenType::Star) || is_table_star;

        if is_wildcard {
            let wildcard_expr = if self.match_token(TokenType::Star) {
                if self.check_keyword_text("ILIKE") {
                    self.skip();
                    let pattern = self.parse_expression()?;
                    Expression::ILike(Box::new(LikeOp {
                        negated: false,
                        left: Expression::Star(Star {
                            table: None,
                            except: None,
                            replace: None,
                            rename: None,
                            trailing_comments: Vec::new(),
                            span: None,
                        }),
                        right: pattern,
                        escape: None,
                        quantifier: None,
                        inferred_type: None,
                    }))
                } else {
                    let star = self.parse_star_modifiers(None)?;
                    Expression::Star(star)
                }
            } else {
                let table_name = self.expect_identifier_or_keyword_with_quoted()?;
                self.expect_token(TokenType::Dot)?;
                self.expect_token(TokenType::Star)?;
                let star = self.parse_star_modifiers(Some(table_name))?;
                Expression::Star(star)
            };

            self.expect_token(TokenType::RBrace)?;

            return Ok(Expression::BracedWildcard(Box::new(wildcard_expr)));
        }

        let mut keys: Vec<Expression> = Vec::new();
        let mut values: Vec<Expression> = Vec::new();
        loop {
            let key = self.parse_expression()?;
            self.expect_token(TokenType::Colon)?;
            let value = self.parse_expression()?;
            keys.push(key);
            values.push(value);
            if !self.match_token(TokenType::Comma) {
                break;
            }
            if self.check(TokenType::RBrace) {
                break;
            }
        }
        self.expect_token(TokenType::RBrace)?;
        self.maybe_parse_subscript(Expression::MapFunc(Box::new(MapConstructor {
            keys,
            values,
            curly_brace_syntax: true,
            with_map_keyword: false,
        })))
    }

    fn parse_primary_exists(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::LParen)?;

        if self.check(TokenType::Select)
            || self.check(TokenType::With)
            || self.check(TokenType::From)
            || self.check(TokenType::LParen)
                && self
                    .peek_nth(1)
                    .map(|t| {
                        matches!(
                            t.token_type,
                            TokenType::Select | TokenType::With | TokenType::From
                        )
                    })
                    .unwrap_or(false)
        {
            let query = self.parse_statement()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::Exists(Box::new(Exists {
                this: query,
                not: false,
            })));
        }

        let array_expr = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let predicate = self.parse_expression()?;
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: "EXISTS".to_string(),
            args: vec![array_expr, predicate],
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: None,
            inferred_type: None,
        })))
    }

    fn parse_primary_date_literal(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        let original_text = token.text.to_string();
        if self.check(TokenType::String) {
            let str_token = self.advance()?;
            if self.config.dialect.is_none() {
                return Ok(Expression::Cast(Box::new(Cast {
                    this: Expression::Literal(Box::new(Literal::String(str_token.text))),
                    to: DataType::Date,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                })));
            }
            return Ok(Expression::Literal(Box::new(Literal::Date(str_token.text))));
        }
        if self.match_token(TokenType::LParen) {
            let func_expr = self.parse_typed_function(&original_text, "DATE", false)?;
            return self.maybe_parse_over(func_expr);
        }
        Ok(Self::parsed_column(Column {
            name: Identifier::new(original_text).with_span(token.span),
            table: None,
            join_mark: false,
            trailing_comments: Vec::new(),
            span: None,
            inferred_type: None,
        }))
    }

    fn parse_primary_timestamp_tz_literal(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        let original_text = token.text.to_string();
        let mut precision = None;

        if self.check(TokenType::LParen)
            && self.check_next(TokenType::Number)
            && self.peek_nth(2).is_some_and(|token| {
                token.token_type == TokenType::RParen
                    && self
                        .peek_nth(3)
                        .is_some_and(|next| next.token_type == TokenType::String)
            })
        {
            self.skip();
            precision = Some(self.expect_number()? as u32);
            self.expect_token(TokenType::RParen)?;
        }

        if self.check(TokenType::String) {
            let str_token = self.advance()?;
            return Ok(Expression::Cast(Box::new(Cast {
                this: Expression::Literal(Box::new(Literal::String(str_token.text))),
                to: DataType::Timestamp {
                    precision,
                    timezone: true,
                },
                trailing_comments: Vec::new(),
                double_colon_syntax: false,
                format: None,
                default: None,
                inferred_type: None,
            })));
        }

        self.maybe_parse_subscript(Self::parsed_column(Column {
            name: Identifier::new(original_text).with_span(token.span),
            table: None,
            join_mark: false,
            trailing_comments: Vec::new(),
            span: None,
            inferred_type: None,
        }))
    }

    fn parse_primary_timestamp_literal(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        let original_text = token.text.to_string();
        if self.check(TokenType::String) {
            let str_token = self.advance()?;
            if self.config.dialect.is_none() {
                return Ok(Expression::Cast(Box::new(Cast {
                    this: Expression::Literal(Box::new(Literal::String(str_token.text))),
                    to: DataType::Timestamp {
                        precision: None,
                        timezone: false,
                    },
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                })));
            }
            return Ok(Expression::Literal(Box::new(Literal::Timestamp(
                str_token.text,
            ))));
        }
        if self.check(TokenType::LParen) {
            let is_data_type = self.check_next(TokenType::Number) && {
                let mut lookahead = self.current + 2;
                if lookahead < self.tokens.len()
                    && self.tokens[lookahead].token_type == TokenType::RParen
                {
                    lookahead += 1;
                }
                lookahead < self.tokens.len()
                    && (self.tokens[lookahead].token_type == TokenType::With
                        || self.tokens[lookahead].text.eq_ignore_ascii_case("WITHOUT")
                        || self.tokens[lookahead].token_type == TokenType::String)
            };

            if is_data_type {
                self.skip();
                let precision_value = self.expect_number()? as u32;
                let precision = Some(precision_value);
                self.expect_token(TokenType::RParen)?;

                let data_type = if self.match_token(TokenType::With) {
                    if self.match_token(TokenType::Local) {
                        self.match_keyword("TIME");
                        self.match_keyword("ZONE");
                        DataType::Custom {
                            name: format!("TIMESTAMPLTZ({})", precision_value),
                        }
                    } else {
                        self.match_keyword("TIME");
                        self.match_keyword("ZONE");
                        DataType::Timestamp {
                            precision,
                            timezone: true,
                        }
                    }
                } else if self.match_keyword("WITHOUT") {
                    self.match_keyword("TIME");
                    self.match_keyword("ZONE");
                    DataType::Timestamp {
                        precision,
                        timezone: false,
                    }
                } else {
                    DataType::Timestamp {
                        precision,
                        timezone: false,
                    }
                };

                if self.check(TokenType::String) {
                    let str_token = self.advance()?;
                    return Ok(Expression::Cast(Box::new(Cast {
                        this: Expression::Literal(Box::new(Literal::String(str_token.text))),
                        to: data_type,
                        trailing_comments: Vec::new(),
                        double_colon_syntax: false,
                        format: None,
                        default: None,
                        inferred_type: None,
                    })));
                }

                return Ok(Expression::DataType(data_type));
            }

            self.skip();
            let func_expr = self.parse_typed_function(&original_text, "TIMESTAMP", false)?;
            return self.maybe_parse_over(func_expr);
        }
        if self.check(TokenType::With)
            && self.peek_nth(1).is_some_and(|t| {
                t.text.eq_ignore_ascii_case("TIME") || t.text.eq_ignore_ascii_case("LOCAL")
            })
            || self.check_keyword_text("WITHOUT")
        {
            let data_type = if self.match_token(TokenType::With) {
                if self.match_token(TokenType::Local) {
                    self.match_keyword("TIME");
                    self.match_keyword("ZONE");
                    DataType::Custom {
                        name: "TIMESTAMPLTZ".to_string(),
                    }
                } else {
                    self.match_keyword("TIME");
                    self.match_keyword("ZONE");
                    DataType::Timestamp {
                        precision: None,
                        timezone: true,
                    }
                }
            } else if self.match_keyword("WITHOUT") {
                self.match_keyword("TIME");
                self.match_keyword("ZONE");
                DataType::Timestamp {
                    precision: None,
                    timezone: false,
                }
            } else {
                DataType::Timestamp {
                    precision: None,
                    timezone: false,
                }
            };

            if self.check(TokenType::String) {
                let str_token = self.advance()?;
                return Ok(Expression::Cast(Box::new(Cast {
                    this: Expression::Literal(Box::new(Literal::String(str_token.text))),
                    to: data_type,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                })));
            }

            return Ok(Expression::DataType(data_type));
        }
        Ok(Self::parsed_column(Column {
            name: Identifier::new(original_text).with_span(token.span),
            table: None,
            join_mark: false,
            trailing_comments: Vec::new(),
            span: None,
            inferred_type: None,
        }))
    }

    fn parse_primary_datetime_literal(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        let original_text = token.text.to_string();
        if self.check(TokenType::String) {
            let str_token = self.advance()?;
            return Ok(Expression::Literal(Box::new(Literal::Datetime(
                str_token.text,
            ))));
        }
        if self.match_token(TokenType::LParen) {
            let func_expr = self.parse_typed_function(&original_text, "DATETIME", false)?;
            return self.maybe_parse_over(func_expr);
        }
        Ok(Self::parsed_column(Column {
            name: Identifier::new(original_text).with_span(token.span),
            table: None,
            join_mark: false,
            trailing_comments: Vec::new(),
            span: None,
            inferred_type: None,
        }))
    }

    fn parse_primary_row(&mut self) -> Result<Expression> {
        self.skip();
        self.expect_token(TokenType::LParen)?;
        let args = if !self.check(TokenType::RParen) {
            self.parse_expression_list()?
        } else {
            Vec::new()
        };
        self.expect_token(TokenType::RParen)?;
        let func_expr = Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: "ROW".to_string(),
            args,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: None,
            inferred_type: None,
        }));
        self.maybe_parse_over(func_expr)
    }

    fn parse_primary_number(&mut self) -> Result<Expression> {
        let token = self.advance()?;

        let literal = if let Some(sep_pos) = token.text.find("::") {
            let num_part = &token.text[..sep_pos];
            let type_name = &token.text[sep_pos + 2..];
            let num_expr = Expression::Literal(Box::new(Literal::Number(num_part.to_string())));
            let data_type = match type_name {
                "BIGINT" => crate::expressions::types::models::DataType::BigInt { length: None },
                "SMALLINT" => {
                    crate::expressions::types::models::DataType::SmallInt { length: None }
                }
                "TINYINT" => crate::expressions::types::models::DataType::TinyInt { length: None },
                "DOUBLE" => crate::expressions::types::models::DataType::Double {
                    precision: None,
                    scale: None,
                },
                "FLOAT" => crate::expressions::types::models::DataType::Float {
                    precision: None,
                    scale: None,
                    real_spelling: false,
                },
                "DECIMAL" => crate::expressions::types::models::DataType::Decimal {
                    precision: None,
                    scale: None,
                },
                _ => crate::expressions::types::models::DataType::Custom {
                    name: type_name.to_string(),
                },
            };
            Expression::Cast(Box::new(crate::expressions::operators::models::Cast {
                this: num_expr,
                to: data_type,
                trailing_comments: Vec::new(),
                double_colon_syntax: false,
                format: None,
                default: None,
                inferred_type: None,
            }))
        } else {
            Expression::Literal(Box::new(Literal::Number(token.text)))
        };
        self.maybe_parse_subscript(literal)
    }

    fn parse_primary_string(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        let first_literal = Expression::Literal(Box::new(Literal::String(token.text)));
        let first_literal = if token.trailing_comments.is_empty() {
            first_literal
        } else {
            Expression::Annotated(Box::new(Annotated {
                this: first_literal,
                trailing_comments: token.trailing_comments,
                type_assertion: None,
                type_like: None,
            }))
        };

        if self.check(TokenType::String) {
            let mut expressions = vec![first_literal];
            while self.check(TokenType::String) {
                let next_token = self.advance()?;
                let literal = Expression::Literal(Box::new(Literal::String(next_token.text)));
                let literal = if next_token.trailing_comments.is_empty() {
                    literal
                } else {
                    Expression::Annotated(Box::new(Annotated {
                        this: literal,
                        trailing_comments: next_token.trailing_comments,
                        type_assertion: None,
                        type_like: None,
                    }))
                };
                expressions.push(literal);
            }
            let concat_func = Expression::Function(Box::new(Function::new("CONCAT", expressions)));
            return self.maybe_parse_subscript(concat_func);
        }

        self.maybe_parse_subscript(first_literal)
    }

    fn parse_primary_star(&mut self) -> Result<Expression> {
        if self.check_next_identifier("COLUMNS")
            && self
                .tokens
                .get(self.current + 2)
                .map(|t| t.token_type == TokenType::LParen)
                .unwrap_or(false)
        {
            self.skip();
            self.skip();
            self.skip();

            let arg = if self.check(TokenType::Star) {
                self.skip();
                Expression::Star(Star {
                    table: None,
                    except: None,
                    replace: None,
                    rename: None,
                    trailing_comments: Vec::new(),
                    span: None,
                })
            } else if let Some(lambda) = self.parse_function_lambda()? {
                lambda
            } else {
                self.parse_expression()?
            };

            self.expect_token(TokenType::RParen)?;

            return Ok(Expression::Columns(Box::new(Columns {
                this: Box::new(arg),
                unpack: Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                }))),
            })));
        }

        if let Some(star_like) = self.parse_clickhouse_bare_star_like()? {
            return self.maybe_parse_subscript(star_like);
        }

        self.skip();
        let star = self.parse_star_modifiers(None)?;
        Ok(Expression::Star(star))
    }

    fn try_parse_primary_identifier_start(&mut self) -> Result<Option<Expression>> {
        let name_upper = self.peek_text().to_ascii_uppercase();
        if (name_upper == ARRAY || name_upper == MAP || name_upper == STRUCT)
            && self.check_next(TokenType::Lt)
        {
            self.skip();
            let data_type = self.parse_data_type_from_name(&name_upper)?;

            if self.match_token(TokenType::LParen) {
                if name_upper == STRUCT {
                    let args = if self.check(TokenType::RParen) {
                        Vec::new()
                    } else {
                        self.parse_struct_args()?
                    };
                    self.expect_token(TokenType::RParen)?;

                    let fields: Vec<(Option<String>, Expression)> =
                        args.into_iter().map(|e| (None, e)).collect();

                    let struct_expr = Expression::Struct(Box::new(Struct { fields }));
                    let cast_expr = Expression::Cast(Box::new(Cast {
                        this: struct_expr,
                        to: data_type,
                        trailing_comments: Vec::new(),
                        double_colon_syntax: false,
                        format: None,
                        default: None,
                        inferred_type: None,
                    }));
                    return (self.maybe_parse_subscript(cast_expr)).map(Some);
                } else if name_upper == ARRAY {
                    let mut expressions: Vec<Expression> = Vec::new();
                    if !self.check(TokenType::RParen) {
                        loop {
                            expressions.push(self.parse_expression()?);
                            if !self.match_token(TokenType::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect_token(TokenType::RParen)?;

                    let array_expr = Expression::Array(Box::new(Array {
                        expressions,
                        inferred_type: None,
                    }));
                    let cast_expr = Expression::Cast(Box::new(Cast {
                        this: array_expr,
                        to: data_type,
                        trailing_comments: Vec::new(),
                        double_colon_syntax: false,
                        format: None,
                        default: None,
                        inferred_type: None,
                    }));
                    return (self.maybe_parse_subscript(cast_expr)).map(Some);
                }
            } else if self.match_token(TokenType::LBracket) {
                let expressions = if self.check(TokenType::RBracket) {
                    Vec::new()
                } else {
                    self.parse_expression_list()?
                };
                self.expect_token(TokenType::RBracket)?;
                let array_expr = Expression::Array(Box::new(Array {
                    expressions,
                    inferred_type: None,
                }));
                let cast_expr = Expression::Cast(Box::new(Cast {
                    this: array_expr,
                    to: data_type,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                }));
                return (self.maybe_parse_subscript(cast_expr)).map(Some);
            }

            return Ok(Some(Expression::DataType(data_type)));
        }
        if name_upper == MAP && self.check_next(TokenType::LBrace) {
            self.skip();
            self.expect_token(TokenType::LBrace)?;

            if self.match_token(TokenType::RBrace) {
                return (self.maybe_parse_subscript(Expression::MapFunc(Box::new(
                    MapConstructor {
                        keys: Vec::new(),
                        values: Vec::new(),
                        curly_brace_syntax: true,
                        with_map_keyword: true,
                    },
                ))))
                .map(Some);
            }

            let mut keys: Vec<Expression> = Vec::new();
            let mut values: Vec<Expression> = Vec::new();
            loop {
                let key = self.parse_primary()?;
                self.expect_token(TokenType::Colon)?;
                let value = self.parse_expression()?;
                keys.push(key);
                values.push(value);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
                if self.check(TokenType::RBrace) {
                    break;
                }
            }
            self.expect_token(TokenType::RBrace)?;

            return (self.maybe_parse_subscript(Expression::MapFunc(Box::new(MapConstructor {
                keys,
                values,
                curly_brace_syntax: true,
                with_map_keyword: true,
            }))))
            .map(Some);
        }

        Ok(None)
    }

    fn parse_primary_keyword_qualified_column(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        let ident = Identifier::new(token.text).with_span(token.span);
        self.expect_token(TokenType::Dot)?;
        if self.match_token(TokenType::Star) {
            let star = self.parse_star_modifiers(Some(ident))?;
            return Ok(Expression::Star(star));
        }
        let col_ident = self.expect_identifier_or_keyword_with_quoted()?;
        let trailing_comments = self.previous_trailing_comments().to_vec();
        let mut col = Self::parsed_column(Column {
            name: col_ident,
            table: Some(ident),
            join_mark: false,
            trailing_comments,
            span: None,
            inferred_type: None,
        });
        if self.check(TokenType::LParen) && self.check_next(TokenType::Plus) {
            let saved_pos = self.current;
            if self.match_token(TokenType::LParen)
                && self.match_token(TokenType::Plus)
                && self.match_token(TokenType::RParen)
            {
                if let Expression::Column(ref mut c) = col {
                    c.join_mark = true;
                }
            } else {
                self.current = saved_pos;
            }
        }
        self.maybe_parse_subscript(col)
    }

    fn try_parse_primary_binary_cast(&mut self) -> Result<Option<Expression>> {
        let next_idx = self.current + 1;
        let has_expr = next_idx < self.tokens.len()
            && !matches!(
                self.tokens[next_idx].token_type,
                TokenType::Semicolon | TokenType::Eof | TokenType::RParen | TokenType::Comma
            );
        if has_expr {
            self.skip();
            let expr = self.parse_unary()?;
            return Ok(Some(Expression::Cast(Box::new(Cast {
                this: expr,
                to: DataType::Binary { length: None },
                trailing_comments: Vec::new(),
                double_colon_syntax: false,
                format: None,
                default: None,
                inferred_type: None,
            }))));
        }

        Ok(None)
    }

    fn parse_primary_rlike_call(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        self.skip();
        let args = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        self.expect_token(TokenType::RParen)?;
        let func = Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: token.text.to_string(),
            args,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: None,
            inferred_type: None,
        }));
        self.maybe_parse_over(func)
    }

    fn parse_primary_insert_call(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        self.skip();
        let args = if self.check(TokenType::RParen) {
            Vec::new()
        } else {
            self.parse_function_arguments()?
        };
        self.expect_token(TokenType::RParen)?;
        let func = Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: token.text.to_string(),
            args,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: None,
            inferred_type: None,
        }));
        self.maybe_parse_over(func)
    }

    fn parse_primary_current_datetime(&mut self) -> Result<Expression> {
        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) && self.peek().token_type == TokenType::CurrentTime
        {
            self.skip();
            let mut precision = None;
            if self.match_token(TokenType::LParen) {
                if !self.check(TokenType::RParen) {
                    precision = self
                        .parse_function_arguments()?
                        .into_iter()
                        .next()
                        .map(Box::new);
                }
                self.expect_token(TokenType::RParen)?;
            }
            return self.maybe_parse_subscript(Expression::Localtime(Box::new(
                crate::expressions::functions::models::Localtime { this: precision },
            )));
        }
        if self.check_next(TokenType::LParen) {
            let token = self.advance()?;
            self.skip();
            let args = if self.check(TokenType::RParen) {
                Vec::new()
            } else {
                self.parse_function_arguments()?
            };
            self.expect_token(TokenType::RParen)?;
            let func = Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: token.text.to_string(),
                args,
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: false,
                quoted: false,
                span: None,
                inferred_type: None,
            }));
            self.maybe_parse_subscript(func)
        } else {
            let token = self.advance()?;
            let func = Expression::Function(Box::new(Function {
                on_error: None,
                qualified_name: Vec::new(),
                source_dialect: None,

                name: token.text.to_string(),
                args: Vec::new(),
                distinct: false,
                trailing_comments: Vec::new(),
                use_bracket_syntax: false,
                no_parens: true,
                quoted: false,
                span: None,
                inferred_type: None,
            }));
            self.maybe_parse_subscript(func)
        }
    }

    fn try_parse_primary_typed_string(&mut self) -> Result<Option<Expression>> {
        let upper_name = self.peek_text().to_ascii_uppercase();
        if matches!(
            upper_name.as_str(),
            "NUMERIC" | "DECIMAL" | "BIGNUMERIC" | "BIGDECIMAL"
        ) {
            self.skip();
            let str_token = self.advance()?;
            let data_type = match upper_name.as_str() {
                "NUMERIC" | "DECIMAL" | "BIGNUMERIC" | "BIGDECIMAL" => {
                    crate::expressions::types::models::DataType::Decimal {
                        precision: None,
                        scale: None,
                    }
                }
                _ => {
                    return Err(crate::tokens::errors::Error::internal(
                        "type keyword already matched in outer if-condition",
                    ))
                }
            };
            return Ok(Some(Expression::Cast(Box::new(
                crate::expressions::operators::models::Cast {
                    this: Expression::Literal(Box::new(Literal::String(str_token.text))),
                    to: data_type,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                },
            ))));
        }

        Ok(None)
    }

    fn try_parse_primary_percent_variable(&mut self) -> Result<Option<Expression>> {
        self.skip();
        if self.match_token(TokenType::LParen) {
            if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                let name = self.advance_text()?;
                self.expect_token(TokenType::RParen)?;
                if self.check(TokenType::Var) && self.peek_text() == PYFORMAT_PARAMETER_SUFFIX {
                    self.skip();
                }
                return Ok(Some(Expression::Parameter(Box::new(Parameter {
                    name: Some(name),
                    index: None,
                    style: ParameterStyle::Percent,
                    quoted: false,
                    string_quoted: false,
                    expression: None,
                }))));
            } else {
                return Err(self.parse_error("Expected parameter name after %("));
            }
        }
        if self.check(TokenType::Var) && self.peek_text() == PYFORMAT_PARAMETER_SUFFIX {
            self.skip();
            return Ok(Some(Expression::Parameter(Box::new(Parameter {
                name: None,
                index: None,
                style: ParameterStyle::Percent,
                quoted: false,
                string_quoted: false,
                expression: None,
            }))));
        }
        self.current -= 1;

        Ok(None)
    }

    fn parse_primary_keyword_identifier(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        let name = token.text.to_string();

        if self.match_token(TokenType::LParen) {
            let upper_name = name.to_ascii_uppercase();

            let func_expr = self.parse_typed_function(&name, &upper_name, false)?;
            let func_expr = self.maybe_parse_clickhouse_parameterized_agg(func_expr)?;
            return self.maybe_parse_over(func_expr);
        }

        if self.match_token(TokenType::Dot) {
            if self.match_token(TokenType::Star) {
                let ident = Identifier::new(name).with_span(token.span);
                let star = self.parse_star_modifiers(Some(ident))?;
                return Ok(Expression::Star(star));
            }

            if self.check(TokenType::Number) {
                let field_name = self.advance_text()?;
                let col_expr = Expression::Dot(Box::new(DotAccess {
                    this: Self::parsed_column(Column {
                        name: Identifier::new(name).with_span(token.span),
                        table: None,
                        join_mark: false,
                        trailing_comments: Vec::new(),
                        span: None,
                        inferred_type: None,
                    }),
                    field: Identifier::new(field_name),
                    inferred_type: None,
                }));
                return self.maybe_parse_subscript(col_expr);
            }

            let col_ident = self.expect_identifier_or_keyword_with_quoted()?;

            if self.check(TokenType::LParen) {
                self.skip();
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_method_arguments_for(&col_ident.name)?
                };
                self.expect_token(TokenType::RParen)?;
                let method_call = self.parse_method_call_expression(
                    Expression::Identifier(Identifier::new(name).with_span(token.span)),
                    col_ident,
                    args,
                );
                return self.maybe_parse_subscript(method_call);
            }

            let trailing_comments = self.previous_trailing_comments().to_vec();
            let mut col = Self::parsed_column(Column {
                name: col_ident,
                table: Some(Identifier::new(name).with_span(token.span)),
                join_mark: false,
                trailing_comments,
                span: None,
                inferred_type: None,
            });
            if self.check(TokenType::LParen) && self.check_next(TokenType::Plus) {
                let saved_pos = self.current;
                if self.match_token(TokenType::LParen)
                    && self.match_token(TokenType::Plus)
                    && self.match_token(TokenType::RParen)
                {
                    if let Expression::Column(ref mut c) = col {
                        c.join_mark = true;
                    }
                } else {
                    self.current = saved_pos;
                }
            }
            return self.maybe_parse_subscript(col);
        }

        let trailing_comments = self.previous_trailing_comments().to_vec();
        let ident = Identifier::new(name).with_span(token.span);
        let col = Self::parsed_column(Column {
            name: ident,
            table: None,
            join_mark: false,
            trailing_comments,
            span: None,
            inferred_type: None,
        });
        self.maybe_parse_subscript(col)
    }

    fn parse_primary_session_variable(&mut self) -> Result<Expression> {
        let name = if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
            let mut n = self.advance_text()?;
            if self.match_token(TokenType::Dot)
                && (self.check(TokenType::Identifier)
                    || self.check(TokenType::Var)
                    || self.is_safe_keyword_as_identifier())
            {
                n.push('.');
                n.push_str(&self.advance_text()?);
            }
            n
        } else if self.check_keyword() {
            self.advance_text()?
        } else {
            return Err(self.parse_error("Expected variable name after @@"));
        };
        Ok(Expression::Parameter(Box::new(Parameter {
            name: Some(name),
            index: None,
            style: ParameterStyle::DoubleAt,
            quoted: false,
            string_quoted: false,
            expression: None,
        })))
    }

    fn parse_primary_at_variable(&mut self) -> Result<Expression> {
        let (name, quoted, string_quoted) =
            if self.check(TokenType::Identifier) || self.check(TokenType::Var) {
                (self.advance_text()?, false, false)
            } else if self.check(TokenType::QuotedIdentifier) {
                let token = self.advance()?;
                (token.text, true, false)
            } else if self.check(TokenType::String) {
                let token = self.advance()?;
                (token.text, false, true)
            } else if self.check(TokenType::Number) || self.peek().token_type.is_keyword() {
                let token = self.advance()?;
                (token.text, false, false)
            } else {
                return Err(self.parse_error("Expected variable name after @"));
            };
        Ok(Expression::Parameter(Box::new(Parameter {
            name: Some(name),
            index: None,
            style: ParameterStyle::At,
            quoted,
            string_quoted,
            expression: None,
        })))
    }

    fn parse_primary_colon_parameter(&mut self) -> Result<Expression> {
        if self.check(TokenType::Number) {
            let num_token = self.advance()?;
            if let Ok(index) = num_token.text.parse::<u32>() {
                return Ok(Expression::Parameter(Box::new(Parameter {
                    name: None,
                    index: Some(index),
                    style: ParameterStyle::Colon,
                    quoted: false,
                    string_quoted: false,
                    expression: None,
                })));
            }
            return Err(self.parse_error(format!("Invalid colon parameter: :{}", num_token.text)));
        }
        if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
            let name = self.advance_text()?;
            Ok(Expression::Parameter(Box::new(Parameter {
                name: Some(name),
                index: None,
                style: ParameterStyle::Colon,
                quoted: false,
                string_quoted: false,
                expression: None,
            })))
        } else {
            Err(self.parse_error("Expected parameter name after :"))
        }
    }

    fn parse_primary_dollar(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::LBrace) {
            if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                let name_token = self.advance()?;
                let expression = if self.match_token(TokenType::Colon) {
                    if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                        let expr_token = self.advance()?;
                        Some(expr_token.text.to_string())
                    } else {
                        return Err(self.parse_error("Expected identifier after : in ${...}"));
                    }
                } else {
                    None
                };
                self.expect_token(TokenType::RBrace)?;
                return Ok(Expression::Parameter(Box::new(Parameter {
                    name: Some(name_token.text.to_string()),
                    index: None,
                    style: ParameterStyle::DollarBrace,
                    quoted: false,
                    string_quoted: false,
                    expression,
                })));
            } else {
                return Err(self.parse_error("Expected identifier after ${"));
            }
        }
        if self.check(TokenType::Number) {
            let num_token = self.advance()?;
            if let Ok(index) = num_token.text.parse::<u32>() {
                let param_expr = Expression::Parameter(Box::new(Parameter {
                    name: None,
                    index: Some(index),
                    style: ParameterStyle::Dollar,
                    quoted: false,
                    string_quoted: false,
                    expression: None,
                }));
                let result = self.parse_colon_json_path(param_expr)?;
                return self.maybe_parse_subscript(result);
            }
            return Err(self.parse_error(format!("Invalid dollar parameter: ${}", num_token.text)));
        }
        if self.check(TokenType::Identifier)
            || self.check(TokenType::Var)
            || self.check(TokenType::QuotedIdentifier)
            || self.is_safe_keyword_as_identifier()
            || self.config.dialect == Some(crate::tokens::models::DialectType::DuckDB)
                && self.peek().token_type.is_keyword()
        {
            let name_token = self.advance()?;
            return self.maybe_parse_subscript(Expression::Parameter(Box::new(Parameter {
                name: Some(name_token.text.to_string()),
                index: None,
                style: ParameterStyle::Dollar,
                quoted: name_token.token_type == TokenType::QuotedIdentifier,
                string_quoted: false,
                expression: None,
            })));
        }
        Err(self.parse_error("Expected number or identifier after $"))
    }

    fn parse_primary_percent_parameter(&mut self) -> Result<Expression> {
        if self.match_token(TokenType::LParen) {
            if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                let name = self.advance_text()?;
                self.expect_token(TokenType::RParen)?;
                if self.check(TokenType::Var) && self.peek_text() == PYFORMAT_PARAMETER_SUFFIX {
                    self.skip();
                }
                return Ok(Expression::Parameter(Box::new(Parameter {
                    name: Some(name),
                    index: None,
                    style: ParameterStyle::Percent,
                    quoted: false,
                    string_quoted: false,
                    expression: None,
                })));
            } else {
                return Err(self.parse_error("Expected parameter name after %("));
            }
        }
        if self.check(TokenType::Var) && self.peek_text() == PYFORMAT_PARAMETER_SUFFIX {
            self.skip();
            return Ok(Expression::Parameter(Box::new(Parameter {
                name: None,
                index: None,
                style: ParameterStyle::Percent,
                quoted: false,
                string_quoted: false,
                expression: None,
            })));
        }
        Err(self.parse_error("Expected 's' or '(' after % for parameter"))
    }

    fn parse_primary_join_keyword_identifier(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        let ident = Identifier::new(token.text).with_span(token.span);
        self.expect_token(TokenType::Dot)?;
        if self.match_token(TokenType::Star) {
            let star = self.parse_star_modifiers(Some(ident))?;
            return Ok(Expression::Star(star));
        }
        let col_ident = self.expect_identifier_or_keyword_with_quoted()?;
        let trailing_comments = self.previous_trailing_comments().to_vec();
        let mut col = Self::parsed_column(Column {
            name: col_ident,
            table: Some(ident),
            join_mark: false,
            trailing_comments,
            span: None,
            inferred_type: None,
        });
        if self.check(TokenType::LParen) && self.check_next(TokenType::Plus) {
            let saved_pos = self.current;
            if self.match_token(TokenType::LParen)
                && self.match_token(TokenType::Plus)
                && self.match_token(TokenType::RParen)
            {
                if let Expression::Column(ref mut c) = col {
                    c.join_mark = true;
                }
            } else {
                self.current = saved_pos;
            }
        }
        self.maybe_parse_subscript(col)
    }

    fn parse_primary_next_call(&mut self) -> Result<Expression> {
        let token = self.advance()?;
        self.skip();
        let args = self.parse_function_args_list()?;
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: token.text,
            args,
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: false,
            no_parens: false,
            quoted: false,
            span: None,
            inferred_type: None,
        })))
    }

    /// Check if function name is a known aggregate function
    pub(crate) fn is_aggregate_function(&self, name: &str) -> bool {
        crate::functions::names::main::is_aggregate_function_name_for_dialect::is_aggregate_function_name_for_dialect(name, self.config.dialect)
    }

    /// Whether the source dialect writes LOG(base, value), base first (not BigQuery or TSQL).
    pub(crate) fn log_base_first(&self) -> bool {
        !false
    }
}
