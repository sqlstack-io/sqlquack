//! Parser functions: generic.

use crate::expressions::functions::models::AggregateFunction;
use crate::expressions::functions::models::Function;
use crate::expressions::functions::models::JSONCast;
use crate::expressions::functions::models::Keep;
use crate::expressions::functions::models::WindowFunction;
use crate::expressions::functions::models::WithinGroup;
use crate::expressions::names::main::identifier_from_token::identifier_from_token;
use crate::expressions::names::models::Column;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::Literal;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::BinaryOp;
use crate::expressions::operators::models::Cast;
use crate::expressions::operators::models::CollationExpr;
use crate::expressions::operators::models::DotAccess;
use crate::expressions::operators::models::Filter;
use crate::expressions::operators::models::LambdaExpr;
use crate::expressions::operators::models::NamedArgSeparator;
use crate::expressions::operators::models::NamedArgument;
use crate::expressions::operators::models::TableArgument;
use crate::expressions::operators::models::UnaryOp;
use crate::expressions::queries::models::Ordered;
use crate::expressions::queries::models::Over;
use crate::expressions::queries::models::WindowFrame;
use crate::expressions::queries::models::WindowFrameBound;
use crate::expressions::queries::models::WindowFrameExclude;
use crate::expressions::queries::models::WindowFrameKind;
use crate::expressions::statements::models::Annotated;
use crate::expressions::statements::models::Paren;
use crate::expressions::statements::models::Var;
use crate::expressions::tree::models::Expression;
use crate::expressions::typed_functions::models::ArrayConstructor;
use crate::expressions::typed_functions::models::JsonExtractFunc;
use crate::expressions::typed_functions::models::JsonPathFunc;
use crate::expressions::typed_functions::models::Slice;
use crate::expressions::types::main::parse_interval_unit_from_string::parse_interval_unit_from_string;
use crate::expressions::types::models::DataType;
use crate::expressions::types::models::Interval;
use crate::expressions::types::models::IntervalSpan;
use crate::expressions::types::models::IntervalSpanExpr;
use crate::expressions::types::models::IntervalUnit;
use crate::expressions::types::models::IntervalUnitSpec;
use crate::parser::_helpers::functions::token_sets::NO_PAREN_FUNCTIONS;
use crate::parser::_helpers::functions::token_sets::TYPE_TOKENS;
use crate::parser::constants::ARRAY;
use crate::parser::constants::BINARY_ARITY;
use crate::parser::constants::LIST;
use crate::parser::constants::VARIADIC_KEYWORD;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse a generic function call (fallback for unrecognized functions)
    pub(crate) fn parse_generic_function(
        &mut self,
        name: &str,
        quoted: bool,
    ) -> Result<Expression> {
        if self.is_snowflake() && !quoted && name.eq_ignore_ascii_case("IN") {
            return Err(self.parse_error("IN is an operator, not a function"));
        }
        let is_known_agg = self.is_aggregate_function(name);

        let (mut args, distinct) = if self.check(TokenType::RParen) {
            (Vec::new(), false)
        } else if let Some(star_like) = self.parse_clickhouse_bare_star_like()? {
            (vec![star_like], false)
        } else if self.check(TokenType::Star) {
            if self.check_next_identifier("COLUMNS")
                && self
                    .tokens
                    .get(self.current + 2)
                    .map(|t| t.token_type == TokenType::LParen)
                    .unwrap_or(false)
            {
                (self.parse_function_arguments_for(Some(name))?, false)
            } else {
                self.skip();
                let star = self.parse_star_modifiers(None)?;
                let mut args = vec![Expression::Star(star)];
                if self.match_token(TokenType::Comma) {
                    let rest = self.parse_function_arguments_for(Some(name))?;
                    args.extend(rest);
                }
                (args, false)
            }
        } else if self.check(TokenType::Distinct)
            && !self.check_next(TokenType::Comma)
            && !self.check_next(TokenType::RParen)
        {
            self.skip();
            (self.parse_function_arguments_for(Some(name))?, true)
        } else {
            if is_known_agg {
                self.match_token(TokenType::All);
            }
            (self.parse_function_arguments_for(Some(name))?, false)
        };

        let (ignore_nulls, order_by, agg_limit) = if is_known_agg {
            let ignore_nulls = if self.match_token(TokenType::Ignore)
                && self.match_token(TokenType::Nulls)
            {
                Some(true)
            } else if self.match_token(TokenType::Respect) && self.match_token(TokenType::Nulls) {
                Some(false)
            } else {
                None
            };

            let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By]) {
                self.parse_order_by_list()?
            } else {
                Vec::new()
            };
            let limit = if self.match_token(TokenType::Limit) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            (ignore_nulls, order_by, limit)
        } else {
            (None, Vec::new(), None)
        };

        self.expect_token(TokenType::RParen)?;
        let trailing_comments = self.previous_trailing_comments().to_vec();

        if self.match_identifier("WITHIN") && self.match_identifier("GROUP") {
            self.expect_token(TokenType::LParen)?;
            self.expect_token(TokenType::Order)?;
            self.expect_token(TokenType::By)?;
            let within_order = self.parse_order_by_list()?;
            self.expect_token(TokenType::RParen)?;

            let func_expr = Expression::AggregateFunction(Box::new(AggregateFunction {
                source_dialect: None,

                name: name.to_string(),
                args,
                distinct,
                filter: None,
                order_by: Vec::new(),
                limit: None,
                ignore_nulls: None,
                inferred_type: None,
                span: SourceSpan::default(),
            }));

            let within = Expression::WithinGroup(Box::new(WithinGroup {
                this: func_expr,
                order_by: within_order,
            }));

            let filter = self.parse_filter_clause()?;
            if let Some(filter_expr) = filter {
                return Ok(Expression::AggregateFunction(Box::new(AggregateFunction {
                    source_dialect: None,

                    name: format!("__WITHIN_GROUP_{}", name),
                    args: vec![within, filter_expr],
                    distinct: false,
                    filter: None,
                    order_by: Vec::new(),
                    limit: None,
                    ignore_nulls: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                })));
            }

            return Ok(within);
        }

        let filter = self.parse_filter_clause()?;

        let ignore_nulls = if ignore_nulls.is_some() {
            ignore_nulls
        } else if self.match_keywords(&[TokenType::Ignore, TokenType::Nulls]) {
            Some(true)
        } else if self.match_keywords(&[TokenType::Respect, TokenType::Nulls]) {
            Some(false)
        } else {
            None
        };

        if filter.is_some() || is_known_agg || ignore_nulls.is_some() {
            Ok(Expression::AggregateFunction(Box::new(AggregateFunction {
                source_dialect: None,

                name: name.to_string(),
                args,
                distinct,
                filter,
                order_by,
                limit: agg_limit,
                ignore_nulls,
                inferred_type: None,
                span: SourceSpan::default(),
            })))
        } else {
            if !distinct
                && name.eq_ignore_ascii_case(
                    crate::expressions::tree::constants::TYPED_EXPRESSION_FUNCTION,
                )
            {
                if let Some(type_assertion) = self.typed_expression_assertion(&args)? {
                    let this = args.swap_remove(0);
                    return Ok(Expression::Annotated(Box::new(
                        crate::expressions::statements::models::Annotated {
                            this,
                            trailing_comments,
                            type_assertion: Some(type_assertion),
                            type_like: None,
                        },
                    )));
                }
            }
            if !distinct
                && args.len() == BINARY_ARITY
                && name.eq_ignore_ascii_case(
                    crate::expressions::tree::constants::TYPED_LIKE_EXPRESSION_FUNCTION,
                )
            {
                let like = args.pop().map(Box::new);
                let this = args.swap_remove(0);
                return Ok(Expression::Annotated(Box::new(
                    crate::expressions::statements::models::Annotated {
                        this,
                        trailing_comments,
                        type_assertion: None,
                        type_like: like,
                    },
                )));
            }
            args = self.normalize_date_part_arg(name, args);
            let mut func = Function::new(name.to_string(), args);
            func.distinct = distinct;
            func.trailing_comments = trailing_comments;
            func.quoted = quoted;
            Ok(Expression::Function(Box::new(func)))
        }
    }

    /// The type asserted by a well-formed `__polyglot_typed(expr, 'TYPE')` call.
    pub(crate) fn typed_expression_assertion(
        &self,
        args: &[Expression],
    ) -> Result<Option<DataType>> {
        let [_, Expression::Literal(literal)] = args else {
            return Ok(None);
        };
        let Literal::String(text) = literal.as_ref() else {
            return Ok(None);
        };
        crate::parser::models::Dialect::get(self.config.dialect.unwrap_or_default())
            .parse_data_type_text(text)
    }

    /// Check for an AS alias after an expression in ClickHouse function arg context.
    pub(crate) fn maybe_clickhouse_alias(&mut self, expr: Expression) -> Expression {
        expr
    }

    /// Parses an expression and an optional ClickHouse `AS alias` inside function arguments.
    pub(crate) fn parse_expression_with_clickhouse_alias(&mut self) -> Result<Expression> {
        let expr = self.parse_expression()?;
        Ok(self.maybe_clickhouse_alias(expr))
    }

    pub(crate) fn parse_single_function_argument(&mut self) -> Result<Expression> {
        let is_table_or_model_arg = if !self.is_at_end() {
            self.check(TokenType::Table) || self.peek_text().eq_ignore_ascii_case("MODEL")
        } else {
            false
        };
        let arg = if is_table_or_model_arg {
            let prefix = self.peek_text().to_ascii_uppercase();
            let saved_pos = self.current;
            self.skip();

            if !self.is_at_end()
                && !self.check(TokenType::FArrow)
                && !self.check(TokenType::ColonEq)
            {
                if let Some(table_expr) = self.parse_table_parts()? {
                    Expression::TableArgument(Box::new(TableArgument {
                        prefix,
                        this: table_expr,
                    }))
                } else {
                    self.current = saved_pos;
                    self.parse_expression()?
                }
            } else {
                self.current = saved_pos;
                if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                    let ident_token = self.advance()?;
                    let ident_name = ident_token.text.to_string();
                    if self.match_token(TokenType::FArrow) {
                        let value = self.parse_function_argument_value()?;
                        Expression::NamedArgument(Box::new(NamedArgument {
                            name: Identifier::new(ident_name).with_span(ident_token.span),
                            value,
                            separator: NamedArgSeparator::DArrow,
                        }))
                    } else if self.match_token(TokenType::ColonEq) {
                        let value = self.parse_expression()?;
                        Expression::NamedArgument(Box::new(NamedArgument {
                            name: Identifier::new(ident_name).with_span(ident_token.span),
                            value,
                            separator: NamedArgSeparator::ColonEq,
                        }))
                    } else {
                        self.current = saved_pos;
                        self.parse_expression()?
                    }
                } else {
                    self.parse_expression()?
                }
            }
        } else if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
            let saved_pos = self.current;
            let ident_token = self.advance()?;
            let ident_name = ident_token.text.to_string();
            let quoted = ident_token.token_type == TokenType::QuotedIdentifier;

            if ident_name.eq_ignore_ascii_case(VARIADIC_KEYWORD)
                && self.config.dialect == Some(crate::tokens::models::DialectType::PostgreSQL)
            {
                self.current = saved_pos;
                self.parse_expression()?
            } else {
                if !self.is_at_end()
                    && self.is_type_keyword()
                    && !self.check(TokenType::FArrow)
                    && !self.check(TokenType::ColonEq)
                {
                    let type_annotation = self.parse_data_type()?;

                    if self.match_token(TokenType::Arrow) {
                        let body = self.parse_expression()?;
                        Expression::Lambda(Box::new(LambdaExpr {
                            parameters: vec![identifier_from_token(ident_token, quoted)],
                            body,
                            colon: false,
                            parameter_types: vec![Some(type_annotation)],
                        }))
                    } else {
                        self.current = saved_pos;
                        self.parse_expression()?
                    }
                } else if !matches!(
                    self.config.dialect,
                    Some(crate::tokens::models::DialectType::DuckDB)
                        | Some(crate::tokens::models::DialectType::SQLite)
                ) && self.match_token(TokenType::Arrow)
                {
                    let body = self.parse_expression()?;
                    Expression::Lambda(Box::new(LambdaExpr {
                        parameters: vec![identifier_from_token(ident_token, quoted)],
                        body,
                        colon: false,
                        parameter_types: Vec::new(),
                    }))
                } else if self.match_token(TokenType::FArrow) {
                    let value = self.parse_function_argument_value()?;
                    Expression::NamedArgument(Box::new(NamedArgument {
                        name: Identifier::new(ident_name).with_span(ident_token.span),
                        value,
                        separator: NamedArgSeparator::DArrow,
                    }))
                } else if self.match_token(TokenType::ColonEq) {
                    let value = self.parse_expression()?;
                    Expression::NamedArgument(Box::new(NamedArgument {
                        name: Identifier::new(ident_name).with_span(ident_token.span),
                        value,
                        separator: NamedArgSeparator::ColonEq,
                    }))
                } else {
                    self.current = saved_pos;
                    self.parse_expression()?
                }
            }
        } else {
            self.parse_expression()?
        };

        let arg = { arg };

        let arg = self.try_clickhouse_implicit_alias(arg);

        let trailing_comments = self.previous_trailing_comments().to_vec();
        if trailing_comments.is_empty() {
            Ok(arg)
        } else {
            Ok(match &arg {
                Expression::Literal(_) | Expression::Boolean(_) | Expression::Null(_) => {
                    Expression::Annotated(Box::new(Annotated {
                        this: arg,
                        trailing_comments,
                        type_assertion: None,
                        type_like: None,
                    }))
                }
                _ => arg,
            })
        }
    }

    pub(crate) fn parse_function_argument_value(&mut self) -> Result<Expression> {
        if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::Snowflake)
        ) && (self.check(TokenType::Select) || self.check(TokenType::With))
        {
            self.parse_statement()
        } else {
            self.parse_expression()
        }
    }

    /// Parses function arguments, named (`=>`, `:=`) and BigQuery TABLE/MODEL ones included.
    pub(crate) fn parse_function_arguments(&mut self) -> Result<Vec<Expression>> {
        self.parse_function_arguments_for(None)
    }

    pub(crate) fn parse_function_arguments_for(
        &mut self,
        function: Option<&str>,
    ) -> Result<Vec<Expression>> {
        self.parse_function_arguments_in_context(function, false)
    }

    pub(crate) fn parse_method_arguments_for(&mut self, method: &str) -> Result<Vec<Expression>> {
        if let Some(query) = self.parse_snowflake_query_argument()? {
            return Ok(vec![query]);
        }
        self.parse_function_arguments_in_context(Some(method), true)
    }

    pub(crate) fn lambda_argument(mut expression: &Expression) -> Option<&LambdaExpr> {
        while let Expression::Paren(paren) = expression {
            expression = &paren.this;
        }
        match expression {
            Expression::Lambda(lambda) => Some(lambda),
            _ => None,
        }
    }

    pub(crate) fn parse_function_arguments_in_context(
        &mut self,
        function: Option<&str>,
        receiver: bool,
    ) -> Result<Vec<Expression>> {
        let lambda_index =
            if self.config.dialect == Some(crate::tokens::models::DialectType::DuckDB) {
                function.and_then(|name| {
                    if name.eq_ignore_ascii_case("COLUMNS") {
                        Some(0)
                    } else if [
                        "APPLY",
                        "ARRAY_APPLY",
                        "ARRAY_FILTER",
                        "ARRAY_REDUCE",
                        "ARRAY_TRANSFORM",
                        "FILTER",
                        "LIST_APPLY",
                        "LIST_FILTER",
                        "LIST_REDUCE",
                        "LIST_TRANSFORM",
                        "REDUCE",
                    ]
                    .iter()
                    .any(|candidate| name.eq_ignore_ascii_case(candidate))
                    {
                        Some(1)
                    } else {
                        None
                    }
                })
            } else {
                None
            };
        let start = self.current;
        let mut args: Vec<Expression> = Vec::new();

        loop {
            let receiver_reduction = receiver
                && args
                    .first()
                    .and_then(Self::lambda_argument)
                    .is_some_and(|lambda| lambda.parameters.len() > 1);
            let lambda = if lambda_index == Some(args.len()) && !receiver_reduction
                || receiver && lambda_index == Some(1) && args.is_empty()
            {
                self.parse_function_lambda()?
            } else {
                None
            };
            args.push(match lambda {
                Some(lambda) => lambda,
                None => self.parse_single_function_argument()?,
            });

            if !self.match_token(TokenType::Comma) {
                break;
            }
            if self.check(TokenType::Comma) {
                return Err(self.parse_error("Expected a function argument between commas"));
            }
        }

        if receiver
            && lambda_index == Some(1)
            && args.first().and_then(Self::lambda_argument).is_some()
            && args.get(1).and_then(Self::lambda_argument).is_some()
        {
            self.current = start;
            return self.parse_function_arguments_for(function);
        }

        Ok(args)
    }

    /// Parse optional FILTER clause
    pub(crate) fn parse_filter_clause(&mut self) -> Result<Option<Expression>> {
        if self.match_token(TokenType::Filter) {
            self.expect_token(TokenType::LParen)?;
            self.match_token(TokenType::Where);
            let filter_expr = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            Ok(Some(filter_expr))
        } else {
            Ok(None)
        }
    }

    /// Parse STRUCT arguments with optional AS aliases: STRUCT(x, y AS name, ...)
    pub(crate) fn parse_struct_args(&mut self) -> Result<Vec<Expression>> {
        let mut args: Vec<Expression> = Vec::new();

        loop {
            let expr = self.parse_expression()?;

            if self.match_token(TokenType::As) {
                let alias = self.expect_identifier_or_keyword()?;
                args.push(Expression::Alias(Box::new(Alias {
                    this: expr,
                    alias: Identifier::new(alias),
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    pre_alias_comments: Vec::new(),
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                })));
            } else {
                args.push(expr);
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }
        }

        Ok(args)
    }

    /// Maybe parse OVER clause for window functions or WITHIN GROUP for ordered-set aggregates
    pub(crate) fn maybe_parse_over(&mut self, expr: Expression) -> Result<Expression> {
        let expr = self.maybe_parse_subscript(expr)?;

        let expr = { expr };

        let expr = if self.check(TokenType::Within) && self.check_next(TokenType::Group) {
            self.skip();
            self.skip();
            self.expect_token(TokenType::LParen)?;
            self.expect_token(TokenType::Order)?;
            self.expect_token(TokenType::By)?;
            let order_by = self.parse_order_by_list()?;
            self.expect_token(TokenType::RParen)?;
            Expression::WithinGroup(Box::new(WithinGroup {
                this: expr,
                order_by,
            }))
        } else {
            expr
        };

        let expr = if self.match_token(TokenType::Filter) {
            self.expect_token(TokenType::LParen)?;
            self.expect_token(TokenType::Where)?;
            let filter_expr = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            Expression::Filter(Box::new(Filter {
                this: Box::new(expr),
                expression: Box::new(filter_expr),
            }))
        } else {
            expr
        };

        let expr = { expr };

        let keep = if self.check(TokenType::Keep) && self.check_next(TokenType::LParen) {
            self.skip();
            Some(self.parse_keep_clause()?)
        } else {
            None
        };

        if self.match_token(TokenType::Over) {
            let over = self.parse_over_clause()?;
            Ok(Expression::WindowFunction(Box::new(WindowFunction {
                this: expr,
                over,
                keep,
                inferred_type: None,
                span: SourceSpan::default(),
            })))
        } else if keep.is_some() {
            Ok(Expression::WindowFunction(Box::new(WindowFunction {
                this: expr,
                over: Over {
                    window_name: None,
                    partition_by: Vec::new(),
                    order_by: Vec::new(),
                    frame: None,
                    alias: None,
                },
                keep,
                inferred_type: None,
                span: SourceSpan::default(),
            })))
        } else {
            Ok(expr)
        }
    }

    /// ClickHouse: parse parameterized aggregate functions like func(params)(args)
    pub(crate) fn maybe_parse_clickhouse_parameterized_agg(
        &mut self,
        expr: Expression,
    ) -> Result<Expression> {
        Ok(expr)
    }

    /// Parse Oracle KEEP clause: KEEP (DENSE_RANK FIRST|LAST ORDER BY ...)
    pub(crate) fn parse_keep_clause(&mut self) -> Result<Keep> {
        self.expect_token(TokenType::LParen)?;

        if !self.match_identifier("DENSE_RANK") {
            return Err(self.parse_error("Expected DENSE_RANK in KEEP clause"));
        }

        let first = if self.match_token(TokenType::First) {
            true
        } else if self.match_token(TokenType::Last) {
            false
        } else {
            return Err(self.parse_error("Expected FIRST or LAST in KEEP clause"));
        };

        self.expect_token(TokenType::Order)?;
        self.expect_token(TokenType::By)?;

        let order_by = self.parse_order_by_list()?;

        self.expect_token(TokenType::RParen)?;

        Ok(Keep { first, order_by })
    }

    /// Parses a JSON arrow operand without subscripts, for left-to-right `->` and `->>`.
    pub(crate) fn parse_json_path_operand(&mut self) -> Result<Expression> {
        if self.check(TokenType::Dash) {
            let dash_pos = self.current;
            self.skip();
            if self.check(TokenType::Number) {
                let token = self.advance()?;
                return Ok(Expression::Neg(Box::new(UnaryOp {
                    this: Expression::Literal(Box::new(Literal::Number(token.text))),
                    inferred_type: None,
                    span: SourceSpan::default(),
                })));
            }
            self.current = dash_pos;
        }

        if self.check(TokenType::Number) {
            let token = self.advance()?;
            if let Some(sep_pos) = token.text.find("::") {
                let num_part = &token.text[..sep_pos];
                let type_name = &token.text[sep_pos + 2..];
                let num_expr = Expression::Literal(Box::new(Literal::Number(num_part.to_string())));
                let data_type = match type_name {
                    "BIGINT" => {
                        crate::expressions::types::models::DataType::BigInt { length: None }
                    }
                    "SMALLINT" => {
                        crate::expressions::types::models::DataType::SmallInt { length: None }
                    }
                    "TINYINT" => {
                        crate::expressions::types::models::DataType::TinyInt { length: None }
                    }
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
                return Ok(Expression::TryCast(Box::new(
                    crate::expressions::operators::models::Cast {
                        span: SourceSpan::default(),
                        this: num_expr,
                        to: data_type,
                        trailing_comments: Vec::new(),
                        double_colon_syntax: false,
                        format: None,
                        default: None,
                        inferred_type: None,
                    },
                )));
            }
            return Ok(Expression::Literal(Box::new(Literal::Number(token.text))));
        }

        if self.check(TokenType::String) {
            let token = self.advance()?;
            return Ok(Expression::Literal(Box::new(Literal::String(token.text))));
        }

        if self.match_token(TokenType::LParen) {
            let expr = self.parse_expression()?;
            self.expect_token(TokenType::RParen)?;
            return Ok(Expression::Paren(Box::new(Paren {
                this: expr,
                trailing_comments: Vec::new(),
                span: SourceSpan::default(),
            })));
        }

        if self.match_token(TokenType::LBracket) {
            if self.match_token(TokenType::RBracket) {
                return Ok(Expression::ArrayFunc(Box::new(ArrayConstructor {
                    expressions: Vec::new(),
                    bracket_notation: true,
                    use_list_keyword: false,
                    inferred_type: None,
                })));
            }

            let mut expressions = vec![self.parse_expression()?];
            while self.match_token(TokenType::Comma) {
                if self.check(TokenType::RBracket) {
                    break;
                }
                expressions.push(self.parse_expression()?);
            }
            self.expect_token(TokenType::RBracket)?;

            return Ok(Expression::ArrayFunc(Box::new(ArrayConstructor {
                expressions,
                bracket_notation: true,
                use_list_keyword: false,
                inferred_type: None,
            })));
        }

        if self.is_identifier_token() {
            let first_ident = self.expect_identifier_with_quoted()?;

            if self.match_token(TokenType::Dot)
                && (self.is_identifier_token() || self.is_safe_keyword_as_identifier())
            {
                let second_ident = if self.is_identifier_token() {
                    self.expect_identifier_with_quoted()?
                } else {
                    let token = self.advance()?;
                    Identifier::new(token.text).with_span(token.span)
                };
                return Ok(Self::parsed_column(Column {
                    name: second_ident,
                    table: Some(first_ident),
                    join_mark: false,
                    trailing_comments: Vec::new(),
                    span: None,
                    inferred_type: None,
                }));
            }

            return Ok(Self::parsed_column(Column {
                name: first_ident,
                table: None,
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            }));
        }

        if self.is_safe_keyword_as_identifier() {
            let token = self.advance()?;
            let first_ident = Identifier::new(token.text).with_span(token.span);

            if self.match_token(TokenType::Dot)
                && (self.is_identifier_token() || self.is_safe_keyword_as_identifier())
            {
                let second_ident = if self.is_identifier_token() {
                    self.expect_identifier_with_quoted()?
                } else {
                    let token = self.advance()?;
                    Identifier::new(token.text).with_span(token.span)
                };
                return Ok(Self::parsed_column(Column {
                    name: second_ident,
                    table: Some(first_ident),
                    join_mark: false,
                    trailing_comments: Vec::new(),
                    span: None,
                    inferred_type: None,
                }));
            }

            return Ok(Self::parsed_column(Column {
                name: first_ident,
                table: None,
                join_mark: false,
                trailing_comments: Vec::new(),
                span: None,
                inferred_type: None,
            }));
        }

        Err(self.parse_error(format!(
            "Unexpected token in JSON path: {:?}",
            self.peek().token_type
        )))
    }

    /// Maybe parse subscript access (array[index], struct.field)
    pub(crate) fn maybe_parse_subscript(&mut self, mut expr: Expression) -> Result<Expression> {
        loop {
            if self.match_token(TokenType::LBracket) {
                let array_constructor_type = match &expr {
                    Expression::Column(col) if col.table.is_none() => {
                        let upper = col.name.name.to_ascii_uppercase();
                        if upper == ARRAY || upper == LIST {
                            Some(upper)
                        } else {
                            None
                        }
                    }
                    Expression::Identifier(id) => {
                        let upper = id.name.to_ascii_uppercase();
                        if upper == ARRAY || upper == LIST {
                            Some(upper)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(constructor_type) = array_constructor_type {
                    expr = self.parse_subscript_array_constructor(constructor_type)?;
                    continue;
                }

                let is_map_constructor = match &expr {
                    Expression::Column(col) => {
                        col.name.name.eq_ignore_ascii_case("MAP") && col.table.is_none()
                    }
                    Expression::Identifier(id) => id.name.eq_ignore_ascii_case("MAP"),
                    _ => false,
                };

                if is_map_constructor {
                    expr = self.parse_subscript_map_constructor()?;
                    continue;
                }

                expr = self.parse_subscript_index_or_slice(expr)?;
            } else if self.match_token(TokenType::DotColon) {
                let data_type = { self.parse_data_type()? };
                expr = Expression::JSONCast(Box::new(JSONCast {
                    this: Box::new(expr),
                    to: data_type,
                }));
            } else if self.match_token(TokenType::Dot) {
                expr = self.parse_subscript_dot_access(expr)?;
            } else if self.match_token(TokenType::Collate) {
                if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
                    && !self.check(TokenType::String)
                {
                    return Err(self.parse_error("Expected a collation specification string"));
                }
                let (collation, quoted, double_quoted) = if self.check(TokenType::String) {
                    (self.advance_text()?, true, false)
                } else if self.check(TokenType::QuotedIdentifier) {
                    (self.advance_text()?, false, true)
                } else {
                    (self.expect_identifier_or_keyword()?, false, false)
                };
                expr = Expression::Collation(Box::new(CollationExpr {
                    this: expr,
                    collation,
                    quoted,
                    double_quoted,
                }));
            } else if self.check(TokenType::DColon)
                || self.check(TokenType::DColonDollar)
                || self.check(TokenType::DColonPercent)
                || self.check(TokenType::DColonQMark)
            {
                {
                    self.skip();

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
            } else if self.match_token(TokenType::ColonGt) {
                let data_type = self.parse_data_type_for_cast()?;
                expr = Expression::Cast(Box::new(Cast {
                    this: expr,
                    to: data_type,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }));
            } else if self.match_token(TokenType::NColonGt) {
                let data_type = self.parse_data_type_for_cast()?;
                expr = Expression::TryCast(Box::new(Cast {
                    this: expr,
                    to: data_type,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: false,
                    format: None,
                    default: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }));
            } else if self.match_token(TokenType::QDColon) {
                let data_type = self.parse_data_type_for_cast()?;
                expr = Expression::TryCast(Box::new(Cast {
                    this: expr,
                    to: data_type,
                    trailing_comments: Vec::new(),
                    double_colon_syntax: true,
                    format: None,
                    default: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }));
            } else if self.check(TokenType::Arrow)
                && !matches!(
                    self.config.dialect,
                    Some(crate::tokens::models::DialectType::DuckDB)
                        | Some(crate::tokens::models::DialectType::SQLite)
                )
            {
                self.skip();
                let path = self.parse_json_path_operand()?;
                expr = Expression::JsonExtract(Box::new(JsonExtractFunc {
                    options: None,
                    source_dialect: None,

                    this: expr,
                    path,
                    returning: None,
                    arrow_syntax: true,
                    hash_arrow_syntax: false,
                    wrapper_option: None,
                    quotes_option: None,
                    on_scalar_string: false,
                    on_error: None,
                }));
            } else if !matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::DuckDB)
                    | Some(crate::tokens::models::DialectType::SQLite)
            ) && !self.applies_snowflake_context_rules()
                && self.match_token(TokenType::DArrow)
            {
                let path = self.parse_json_path_operand()?;
                expr = Expression::JsonExtractScalar(Box::new(JsonExtractFunc {
                    options: None,
                    source_dialect: None,

                    this: expr,
                    path,
                    returning: None,
                    arrow_syntax: true,
                    hash_arrow_syntax: false,
                    wrapper_option: None,
                    quotes_option: None,
                    on_scalar_string: false,
                    on_error: None,
                }));
            } else if self.match_token(TokenType::HashArrow) {
                let path = self.parse_json_path_operand()?;
                expr = Expression::JsonExtractPath(Box::new(JsonPathFunc {
                    this: expr,
                    paths: vec![path],
                }));
            } else if self.match_token(TokenType::DHashArrow) {
                let path = self.parse_json_path_operand()?;
                expr = Expression::JsonExtractScalar(Box::new(JsonExtractFunc {
                    options: None,
                    source_dialect: None,

                    this: expr,
                    path,
                    returning: None,
                    arrow_syntax: false,
                    hash_arrow_syntax: true,
                    wrapper_option: None,
                    quotes_option: None,
                    on_scalar_string: false,
                    on_error: None,
                }));
            } else if self.check_join_marker() {
                if let Expression::Column(col) = &mut expr {
                    self.skip();
                    self.skip();
                    self.skip();
                    col.join_mark = true;
                    break;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_subscript_array_constructor(
        &mut self,
        constructor_type: String,
    ) -> Result<Expression> {
        let use_list_keyword = constructor_type == LIST;
        let expr = if self.check(TokenType::RBracket) {
            self.skip();
            Expression::ArrayFunc(Box::new(ArrayConstructor {
                expressions: Vec::new(),
                bracket_notation: false,
                use_list_keyword,
                inferred_type: None,
            }))
        } else {
            let expressions = self.parse_expression_list()?;
            self.expect_token(TokenType::RBracket)?;
            Expression::ArrayFunc(Box::new(ArrayConstructor {
                expressions,
                bracket_notation: false,
                use_list_keyword,
                inferred_type: None,
            }))
        };
        Ok(expr)
    }

    fn parse_subscript_map_constructor(&mut self) -> Result<Expression> {
        let expr;
        let keys = self.parse_expression()?;
        self.expect_token(TokenType::Comma)?;
        let values = self.parse_expression()?;
        self.expect_token(TokenType::RBracket)?;
        expr = Expression::Function(Box::new(Function {
            on_error: None,
            qualified_name: Vec::new(),
            source_dialect: None,

            name: "MAP".to_string(),
            args: vec![keys, values],
            distinct: false,
            trailing_comments: Vec::new(),
            use_bracket_syntax: true,
            no_parens: false,
            quoted: false,
            span: SourceSpan::default(),
            inferred_type: None,
        }));
        Ok(expr)
    }

    fn parse_subscript_index_or_slice(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
            && self.check(TokenType::RBracket)
        {
            return Err(self.parse_error("Expected an index in the brackets"));
        }
        if self.check(TokenType::Colon) {
            self.skip();
            let end = self.parse_slice_element()?;
            let step = if self.match_token(TokenType::Colon) {
                self.parse_slice_element()?
            } else {
                None
            };
            self.expect_token(TokenType::RBracket)?;
            if step.is_some() {
                let slice = Expression::Slice(Box::new(Slice {
                    this: None,
                    expression: end.map(Box::new),
                    step: step.map(Box::new),
                }));

                expr = self.make_subscript(expr, slice);
            } else {
                expr = self.make_array_slice(expr, None, end);
            }
        } else {
            let start = self.parse_slice_element()?;
            if self.match_token(TokenType::Colon) {
                let end = self.parse_slice_element()?;
                let step = if self.match_token(TokenType::Colon) {
                    self.parse_slice_element()?
                } else {
                    None
                };
                self.expect_token(TokenType::RBracket)?;
                if step.is_some() {
                    let slice = Expression::Slice(Box::new(Slice {
                        this: start.map(Box::new),
                        expression: end.map(Box::new),
                        step: step.map(Box::new),
                    }));

                    expr = self.make_subscript(expr, slice);
                } else {
                    expr = self.make_array_slice(expr, start, end);
                }
            } else {
                self.expect_token(TokenType::RBracket)?;
                let index = start
                    .unwrap_or_else(|| Expression::Null(crate::expressions::names::models::Null));
                expr = self.make_subscript(expr, index);
            }
        }
        Ok(expr)
    }

    fn parse_subscript_dot_access(&mut self, mut expr: Expression) -> Result<Expression> {
        if self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake)
            && matches!(&expr, Expression::Literal(_))
        {
            return Err(self.parse_error("A literal has no fields to read with a dot"));
        }
        if self.match_token(TokenType::Star) {
            let mut table_name = match &expr {
                Expression::Column(col) => {
                    if let Some(ref table) = col.table {
                        Some(Identifier::new(format!("{}.{}", table.name, col.name.name)))
                    } else {
                        Some(col.name.clone())
                    }
                }
                Expression::Dot(d) => {
                    fn dot_to_name_inner(expr: &Expression) -> String {
                        match expr {
                            Expression::Column(col) => {
                                if let Some(ref table) = col.table {
                                    format!("{}.{}", table.name, col.name.name)
                                } else {
                                    col.name.name.clone()
                                }
                            }
                            Expression::Dot(d) => {
                                format!("{}.{}", dot_to_name_inner(&d.this), d.field.name)
                            }
                            _ => String::new(),
                        }
                    }
                    Some(Identifier::new(dot_to_name_inner(&Expression::Dot(
                        d.clone(),
                    ))))
                }
                _ => None,
            };
            if let Some(table) = &mut table_name {
                table.span = Self::reference_span(&expr);
            }
            if table_name.is_some() {
                let star = self.parse_star_modifiers(table_name)?;
                expr = Expression::Star(star);
            } else {
                expr = Expression::Dot(Box::new(DotAccess {
                    this: expr,
                    field: Identifier::new("*"),
                    inferred_type: None,
                }));
            }
        } else if self.check(TokenType::Identifier)
            || self.check(TokenType::Var)
            || self.check(TokenType::QuotedIdentifier)
            || self.check_keyword()
        {
            let is_quoted = self.check(TokenType::QuotedIdentifier);
            let ident = identifier_from_token(self.advance()?, is_quoted);
            if self.check(TokenType::LParen) && !is_quoted {
                self.skip();
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_method_arguments_for(&ident.name)?
                };
                self.expect_token(TokenType::RParen)?;
                expr = self.parse_method_call_expression(expr, ident, args);
                expr = self.parse_qualified_window(expr)?;
            } else {
                expr = Expression::Dot(Box::new(DotAccess {
                    this: expr,
                    field: ident,
                    inferred_type: None,
                }));
            }
        } else if self.check(TokenType::Number) {
            let field = identifier_from_token(self.advance()?, false);
            expr = Expression::Dot(Box::new(DotAccess {
                this: expr,
                field,
                inferred_type: None,
            }));
        } else {
            return Err(self.parse_error("Expected field name after dot"));
        }
        Ok(expr)
    }

    /// Check if the next tokens are the Oracle-style join marker (+)
    pub(crate) fn check_join_marker(&self) -> bool {
        self.check(TokenType::LParen)
            && self
                .peek_nth(1)
                .is_some_and(|t| t.token_type == TokenType::Plus)
            && self
                .peek_nth(2)
                .is_some_and(|t| t.token_type == TokenType::RParen)
    }

    /// Parse OVER clause
    pub(crate) fn parse_over_clause(&mut self) -> Result<Over> {
        if !self.check(TokenType::LParen) {
            let window_name = self.parse_unquoted_identifier(Self::expect_identifier_or_keyword)?;
            return Ok(Over {
                window_name: Some(window_name),
                partition_by: Vec::new(),
                order_by: Vec::new(),
                frame: None,
                alias: None,
            });
        }

        self.expect_token(TokenType::LParen)?;

        let window_name = if (self.check(TokenType::Identifier)
            || self.check(TokenType::Var)
            || self.check_keyword())
            && !self.check(TokenType::Partition)
            && !self.check(TokenType::Order)
            && !self.check(TokenType::Rows)
            && !self.check(TokenType::Range)
            && !self.check(TokenType::Groups)
            && !self.check(TokenType::Distribute)
            && !self.check(TokenType::Sort)
        {
            let pos = self.current;
            let name = identifier_from_token(self.advance()?, false);
            if self.check(TokenType::Order)
                || self.check(TokenType::Partition)
                || self.check(TokenType::Rows)
                || self.check(TokenType::Range)
                || self.check(TokenType::Groups)
                || self.check(TokenType::RParen)
                || self.check(TokenType::Distribute)
                || self.check(TokenType::Sort)
            {
                Some(name)
            } else {
                self.current = pos;
                None
            }
        } else {
            None
        };

        let partition_by = {
            if self.match_keywords(&[TokenType::Partition, TokenType::By])
                || self.match_keywords(&[TokenType::Distribute, TokenType::By])
            {
                self.skip_semantic_view_excluding();
                self.parse_expression_list()?
            } else {
                Vec::new()
            }
        };

        let order_by = if self.match_keywords(&[TokenType::Order, TokenType::By])
            || self.match_keywords(&[TokenType::Sort, TokenType::By])
        {
            let mut exprs: Vec<Ordered> = Vec::new();
            loop {
                let expr = self.parse_expression()?;
                let (desc, explicit_asc) = if self.match_token(TokenType::Desc) {
                    (true, false)
                } else if self.match_token(TokenType::Asc) {
                    (false, true)
                } else {
                    (false, false)
                };
                if self.match_token(TokenType::Collate) {
                    if self.check(TokenType::String) || self.check(TokenType::QuotedIdentifier) {
                        self.skip();
                    } else {
                        let _ = self.expect_identifier_or_keyword();
                    }
                }
                let nulls_auto = false;
                let nulls_first = if self.match_token(TokenType::Nulls) {
                    if self.match_token(TokenType::First) {
                        Some(true)
                    } else if self.match_token(TokenType::Last) {
                        Some(false)
                    } else {
                        return Err(self.parse_error("Expected FIRST, LAST or AUTO after NULLS"));
                    }
                } else {
                    None
                };
                let with_fill = { None };
                exprs.push(Ordered {
                    nulls_auto,
                    this: expr,
                    desc,
                    nulls_first,
                    explicit_asc,
                    with_fill,
                });
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            exprs
        } else {
            Vec::new()
        };

        let frame = self.parse_window_frame()?;

        self.expect_token(TokenType::RParen)?;

        Ok(Over {
            window_name,
            partition_by,
            order_by,
            frame,
            alias: None,
        })
    }

    /// Parse window frame specification (ROWS/RANGE/GROUPS BETWEEN ...)
    pub(crate) fn parse_window_frame(&mut self) -> Result<Option<WindowFrame>> {
        let (kind, kind_text) = if self.match_token(TokenType::Rows) {
            (
                WindowFrameKind::Rows,
                self.tokens[self.current - 1].text.to_string(),
            )
        } else if self.match_token(TokenType::Range) {
            (
                WindowFrameKind::Range,
                self.tokens[self.current - 1].text.to_string(),
            )
        } else if self.match_token(TokenType::Groups) {
            (
                WindowFrameKind::Groups,
                self.tokens[self.current - 1].text.to_string(),
            )
        } else {
            return Ok(None);
        };

        let (start, start_side_text, end, end_side_text) = if self.match_token(TokenType::Between) {
            let (start, st) = self.parse_window_frame_bound()?;
            self.expect_token(TokenType::And)?;
            let (end, et) = self.parse_window_frame_bound()?;
            (start, st, Some(end), et)
        } else {
            let (start, st) = self.parse_window_frame_bound()?;
            (start, st, None, None)
        };

        let exclude = if self.match_token(TokenType::Exclude) {
            if self.match_token(TokenType::Current) {
                self.expect_token(TokenType::Row)?;
                Some(WindowFrameExclude::CurrentRow)
            } else if self.match_token(TokenType::Group) {
                Some(WindowFrameExclude::Group)
            } else if self.match_token(TokenType::Ties) {
                Some(WindowFrameExclude::Ties)
            } else if self.match_token(TokenType::No) {
                self.expect_token(TokenType::Others)?;
                Some(WindowFrameExclude::NoOthers)
            } else {
                return Err(self
                    .parse_error("Expected CURRENT ROW, GROUP, TIES, or NO OTHERS after EXCLUDE"));
            }
        } else {
            None
        };

        Ok(Some(WindowFrame {
            kind,
            start,
            end,
            exclude,
            kind_text: Some(kind_text),
            start_side_text,
            end_side_text,
        }))
    }

    /// Parse a window frame bound, returning the bound and the original text of the side keyword
    pub(crate) fn parse_window_frame_bound(
        &mut self,
    ) -> Result<(WindowFrameBound, Option<String>)> {
        let snowflake: bool =
            self.config.dialect == Some(crate::tokens::models::DialectType::Snowflake);
        if self.match_token(TokenType::Current) {
            self.expect_token(TokenType::Row)?;
            Ok((WindowFrameBound::CurrentRow, None))
        } else if self.match_token(TokenType::Unbounded) {
            if self.match_token(TokenType::Preceding) {
                let text = self.tokens[self.current - 1].text.to_string();
                Ok((WindowFrameBound::UnboundedPreceding, Some(text)))
            } else if self.match_token(TokenType::Following) {
                let text = self.tokens[self.current - 1].text.to_string();
                Ok((WindowFrameBound::UnboundedFollowing, Some(text)))
            } else {
                Err(self.parse_error("Expected PRECEDING or FOLLOWING after UNBOUNDED"))
            }
        } else if self.match_token(TokenType::Preceding) {
            let text = self.tokens[self.current - 1].text.to_string();
            if snowflake {
                Err(self.parse_error(
                    "Snowflake frame bounds need a value or UNBOUNDED before PRECEDING",
                ))
            } else if self.check(TokenType::RParen) || self.check(TokenType::Comma) {
                Ok((WindowFrameBound::BarePreceding, Some(text)))
            } else {
                let expr = self.parse_primary()?;
                Ok((WindowFrameBound::Preceding(Box::new(expr)), Some(text)))
            }
        } else if self.match_token(TokenType::Following) {
            let text = self.tokens[self.current - 1].text.to_string();
            if snowflake {
                Err(self.parse_error(
                    "Snowflake frame bounds need a value or UNBOUNDED before FOLLOWING",
                ))
            } else if self.check(TokenType::RParen) || self.check(TokenType::Comma) {
                Ok((WindowFrameBound::BareFollowing, Some(text)))
            } else {
                let expr = self.parse_primary()?;
                Ok((WindowFrameBound::Following(Box::new(expr)), Some(text)))
            }
        } else {
            if snowflake && self.check(TokenType::String) {
                return Err(self.parse_error("Snowflake frame bounds take a number, not a string"));
            }
            let expr = self.parse_addition()?;
            if self.applies_snowflake_context_rules()
                && matches!(&expr, Expression::Literal(literal) if matches!(**literal, Literal::String(_)))
            {
                return Err(self.parse_error("A Snowflake frame bound value is not a string"));
            }
            if self.match_token(TokenType::Preceding) {
                let text = self.tokens[self.current - 1].text.to_string();
                Ok((WindowFrameBound::Preceding(Box::new(expr)), Some(text)))
            } else if self.match_token(TokenType::Following) {
                let text = self.tokens[self.current - 1].text.to_string();
                Ok((WindowFrameBound::Following(Box::new(expr)), Some(text)))
            } else if snowflake {
                Err(self.parse_error("Expected PRECEDING or FOLLOWING after a frame bound value"))
            } else {
                Ok((WindowFrameBound::Value(Box::new(expr)), None))
            }
        }
    }

    /// Try to parse INTERVAL expression. Returns None if INTERVAL should be treated as identifier.
    pub(crate) fn try_parse_interval(&mut self) -> Result<Option<Expression>> {
        self.try_parse_interval_internal(true)
    }

    /// Parses an interval, the INTERVAL keyword optional for chained value-unit pairs.
    pub(crate) fn try_parse_interval_internal(
        &mut self,
        match_interval: bool,
    ) -> Result<Option<Expression>> {
        self.with_parser_depth(|parser| parser.try_parse_interval_internal_inner(match_interval))
    }

    #[inline(never)]
    pub(crate) fn try_parse_interval_internal_inner(
        &mut self,
        match_interval: bool,
    ) -> Result<Option<Expression>> {
        let start_pos = self.current;

        if match_interval {
            if !self.check(TokenType::Interval) {
                return Ok(None);
            }
            self.expect_token(TokenType::Interval)?;

            if self.check(TokenType::Eq)
                || self.check(TokenType::Neq)
                || self.check(TokenType::Lt)
                || self.check(TokenType::Gt)
                || self.check(TokenType::Lte)
                || self.check(TokenType::Gte)
                || self.check(TokenType::And)
                || self.check(TokenType::Or)
                || self.check(TokenType::Is)
                || self.check(TokenType::In)
                || self.check(TokenType::Like)
                || self.check(TokenType::ILike)
                || self.check(TokenType::Between)
                || self.check(TokenType::Then)
                || self.check(TokenType::Else)
                || self.check(TokenType::When)
                || self.check(TokenType::End)
                || self.check(TokenType::Comma)
                || self.check(TokenType::RParen)
                || self.check(TokenType::DColon)
            {
                self.current = start_pos;
                return Ok(None);
            }
        }

        let value = if self.check(TokenType::String) {
            let token = self.advance()?;
            Some(Expression::Literal(Box::new(Literal::String(token.text))))
        } else if !self.is_at_end() && !self.is_statement_terminator() {
            Some(self.parse_addition()?)
        } else {
            None
        };

        if let Some(ref val) = value {
            if let Expression::Column(col) = val {
                if col.table.is_none() {
                    let is_quoted = col.name.quoted;
                    if !is_quoted && !self.is_valid_interval_unit() && !self.check(TokenType::As) {
                        self.current = start_pos;
                        return Ok(None);
                    }
                }
            } else if let Expression::Identifier(id) = val {
                let is_quoted = id.quoted;
                if !is_quoted && !self.is_valid_interval_unit() && !self.check(TokenType::As) {
                    self.current = start_pos;
                    return Ok(None);
                }
            }
        } else if self.is_at_end() || self.is_statement_terminator() {
            self.current = start_pos;
            return Ok(None);
        }

        self.refuse_snowflake_interval_name(value.as_ref())?;
        let unit = self.try_parse_interval_unit()?;

        let is_generic = self.config.dialect.is_none()
            || matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::Generic)
            );
        let (value, unit) = self.parse_interval_generic_value_and_unit(is_generic, unit, value);

        let value = match value {
            Some(Expression::Literal(lit))
                if unit.is_some() && matches!(lit.as_ref(), Literal::Number(_)) =>
            {
                let Literal::Number(n) = lit.as_ref() else {
                    return Err(crate::tokens::errors::Error::internal(
                        "the parser reached a state its checks rule out",
                    ));
                };
                Some(Expression::Literal(Box::new(Literal::String(n.clone()))))
            }
            other => other,
        };

        let interval = Expression::Interval(Box::new(Interval { this: value, unit }));

        let before_plus = self.current;
        let has_plus = self.match_token(TokenType::Plus);

        if self.check(TokenType::String) || self.check(TokenType::Number) {
            if let Some(next_interval) = self.try_parse_interval_internal(false)? {
                return Ok(Some(Expression::Add(Box::new(BinaryOp::new(
                    interval,
                    next_interval,
                )))));
            }
        }

        if has_plus {
            self.current = before_plus;
        }

        Ok(Some(interval))
    }

    fn parse_interval_generic_value_and_unit(
        &mut self,
        is_generic: bool,
        mut unit: Option<IntervalUnitSpec>,
        value: Option<Expression>,
    ) -> (Option<Expression>, Option<IntervalUnitSpec>) {
        let value = if unit.is_none() && is_generic {
            if let Some(Expression::Literal(ref lit)) = value {
                if let Literal::String(ref s) = lit.as_ref() {
                    let trimmed = s.trim();
                    let mut split_pos = None;
                    let mut found_space = false;
                    let bytes = trimmed.as_bytes();
                    let mut i = 0;
                    if i < bytes.len() && bytes[i] == b'-' {
                        i += 1;
                    }
                    let digit_start = i;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i > digit_start {
                        if i < bytes.len() && bytes[i] == b'.' {
                            i += 1;
                            while i < bytes.len() && bytes[i].is_ascii_digit() {
                                i += 1;
                            }
                        }
                        let space_start = i;
                        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                            i += 1;
                        }
                        if i > space_start {
                            found_space = true;
                            split_pos = Some(i);
                        }
                    }
                    if found_space {
                        if let Some(pos) = split_pos {
                            let unit_text = &trimmed[pos..];
                            if !unit_text.is_empty()
                                && unit_text.chars().all(|c| c.is_ascii_alphabetic())
                            {
                                let num_part = trimmed[..pos].trim_end().to_string();
                                let unit_upper = unit_text.to_ascii_uppercase();
                                if let Some(parsed_unit) =
                                    parse_interval_unit_from_string(&unit_upper)
                                {
                                    unit = Some(IntervalUnitSpec::Simple {
                                        unit: parsed_unit,
                                        use_plural: Self::interval_unit_uses_plural_form(
                                            &unit_upper,
                                        ),
                                    });
                                    Some(Expression::Literal(Box::new(Literal::String(num_part))))
                                } else {
                                    value
                                }
                            } else {
                                value
                            }
                        } else {
                            value
                        }
                    } else {
                        value
                    }
                } else {
                    None
                }
            } else {
                value
            }
        } else {
            value
        };
        (value, unit)
    }

    /// Check if current token is a valid interval unit
    pub(crate) fn is_valid_interval_unit(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let text = self.peek_text().to_ascii_uppercase();
        parse_interval_unit_from_string(&text).is_some()
    }

    /// Check if current token terminates a statement/expression context
    pub(crate) fn is_statement_terminator(&self) -> bool {
        if self.is_at_end() {
            return true;
        }
        matches!(
            self.peek().token_type,
            TokenType::Semicolon
                | TokenType::RParen
                | TokenType::RBracket
                | TokenType::Comma
                | TokenType::From
                | TokenType::Where
                | TokenType::GroupBy
                | TokenType::Having
                | TokenType::OrderBy
                | TokenType::Limit
                | TokenType::Union
                | TokenType::Intersect
                | TokenType::Except
                | TokenType::End
                | TokenType::Then
                | TokenType::Else
                | TokenType::When
        )
    }

    /// Try to parse interval unit - returns None if no unit present
    pub(crate) fn try_parse_interval_unit(&mut self) -> Result<Option<IntervalUnitSpec>> {
        if let Some(qualifier) = self.try_parse_snowflake_interval_qualifier()? {
            return Ok(Some(qualifier));
        }
        if !self.is_snowflake() && self.is_function_start() {
            let start = self.parse_primary()?;
            if self.check_keyword_text("TO") {
                let saved = self.current;
                self.skip();
                if let Some(end) = self.try_parse_interval_unit_expression()? {
                    return Ok(Some(IntervalUnitSpec::ExprSpan(IntervalSpanExpr {
                        this: Box::new(start),
                        expression: Box::new(end),
                    })));
                }
                self.current = saved;
            }
            return Ok(Some(IntervalUnitSpec::Expr(Box::new(start))));
        }

        if let Some((unit, use_plural)) = self.try_parse_simple_interval_unit()? {
            if self.check_keyword_text("TO") {
                let saved = self.current;
                self.skip();
                if self.is_function_start() {
                    let end = self.parse_primary()?;
                    return Ok(Some(IntervalUnitSpec::ExprSpan(IntervalSpanExpr {
                        this: Box::new(Self::interval_unit_expression(unit)),
                        expression: Box::new(end),
                    })));
                }
                if let Some((end_unit, _)) = self.try_parse_simple_interval_unit()? {
                    return Ok(Some(IntervalUnitSpec::Span(IntervalSpan {
                        this: unit,
                        expression: end_unit,
                    })));
                } else {
                    self.current = saved;
                }
            }
            return Ok(Some(IntervalUnitSpec::Simple { unit, use_plural }));
        }

        Ok(None)
    }

    pub(crate) fn try_parse_interval_unit_expression(&mut self) -> Result<Option<Expression>> {
        if self.is_function_start() {
            return self.parse_primary().map(Some);
        }
        Ok(self
            .try_parse_simple_interval_unit()?
            .map(|(unit, _)| Self::interval_unit_expression(unit)))
    }

    pub(crate) fn interval_unit_expression(unit: IntervalUnit) -> Expression {
        let name = match unit {
            IntervalUnit::Year => "YEAR",
            IntervalUnit::Quarter => "QUARTER",
            IntervalUnit::Month => "MONTH",
            IntervalUnit::Week => "WEEK",
            IntervalUnit::Day => "DAY",
            IntervalUnit::Hour => "HOUR",
            IntervalUnit::Minute => "MINUTE",
            IntervalUnit::Second => "SECOND",
            IntervalUnit::Millisecond => "MILLISECOND",
            IntervalUnit::Microsecond => "MICROSECOND",
            IntervalUnit::Nanosecond => "NANOSECOND",
        };
        Expression::Var(Box::new(Var {
            this: name.to_string(),
        }))
    }

    /// Return whether an interval unit token should regenerate in plural form.
    pub(crate) fn interval_unit_uses_plural_form(s: &str) -> bool {
        matches!(
            s.trim().to_ascii_uppercase().as_str(),
            "YEARS"
                | "YRS"
                | "QUARTERS"
                | "QTRS"
                | "MONTHS"
                | "MONS"
                | "WEEKS"
                | "WKS"
                | "DAYS"
                | "HOURS"
                | "HRS"
                | "MINUTES"
                | "MINS"
                | "SECONDS"
                | "SECS"
                | "MILLISECONDS"
                | "MSECS"
                | "MSECONDS"
                | "MILLISECS"
                | "MICROSECONDS"
                | "USECS"
                | "USECONDS"
                | "MICROSECS"
                | "NANOSECONDS"
                | "NSECS"
                | "NSECONDS"
                | "NANOSECS"
        )
    }

    /// Try to parse a simple interval unit (YEAR, MONTH, etc.) - returns (unit, is_plural)
    pub(crate) fn try_parse_simple_interval_unit(
        &mut self,
    ) -> Result<Option<(IntervalUnit, bool)>> {
        if self.is_at_end() {
            return Ok(None);
        }

        let text_upper = self.peek_text().to_ascii_uppercase();
        let result = parse_interval_unit_from_string(&text_upper)
            .map(|unit| (unit, Self::interval_unit_uses_plural_form(&text_upper)));

        if result.is_some() {
            self.skip();
        }

        Ok(result)
    }

    /// Check if current position starts a function call or no-paren function
    pub(crate) fn is_function_start(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token_type = self.peek().token_type;

        if NO_PAREN_FUNCTIONS.contains(&token_type) {
            return true;
        }

        if matches!(
            token_type,
            TokenType::Cast | TokenType::TryCast | TokenType::SafeCast
        ) {
            return true;
        }

        let text_upper = self.peek_text().to_ascii_uppercase();
        if crate::functions::names::main::is_no_paren_function_name_upper::is_no_paren_function_name_upper(text_upper.as_str()) {
            return true;
        }

        if self.is_identifier_token() && self.check_next(TokenType::LParen) {
            return true;
        }

        false
    }

    /// Whether a typed column list `(c1 type1, ...)` starts here, for table function aliases.
    pub(crate) fn check_typed_column_list(&self) -> bool {
        if self.is_at_end() {
            return false;
        }

        let has_identifier = self.check(TokenType::Identifier)
            || self.check(TokenType::QuotedIdentifier)
            || self.check(TokenType::Var);

        if !has_identifier {
            return false;
        }

        let next_pos = self.current + 1;
        if next_pos >= self.tokens.len() {
            return false;
        }

        let next_token = &self.tokens[next_pos];

        if next_token.token_type == TokenType::Comma || next_token.token_type == TokenType::RParen {
            return false;
        }

        TYPE_TOKENS.contains(&next_token.token_type)
            || next_token.token_type == TokenType::Identifier
            || next_token.token_type == TokenType::Var
    }

    /// Check if current token is a no-paren function
    pub(crate) fn is_no_paren_function(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token_type = self.peek().token_type;
        if NO_PAREN_FUNCTIONS.contains(&token_type) {
            return true;
        }
        let text_upper = self.peek_text().to_ascii_uppercase();
        if crate::functions::names::main::is_no_paren_function_name_upper::is_no_paren_function_name_upper(text_upper.as_str()) {
            return true;
        }
        false
    }

    /// Match a keyword by text (case-insensitive)
    pub(crate) fn match_keyword(&mut self, keyword: &str) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.current_token_allows_text_match() && self.peek_text().eq_ignore_ascii_case(keyword)
        {
            self.skip();
            true
        } else {
            false
        }
    }

    /// Match a sequence of keywords by text (case-insensitive)
    pub(crate) fn match_text_seq(&mut self, keywords: &[&str]) -> bool {
        for (i, &kw) in keywords.iter().enumerate() {
            if self.current + i >= self.tokens.len() {
                return false;
            }
            let token = &self.tokens[self.current + i];
            if Self::text_match_excludes_token(token.token_type)
                || !token.text.eq_ignore_ascii_case(kw)
            {
                return false;
            }
        }
        self.current += keywords.len();
        true
    }
}
