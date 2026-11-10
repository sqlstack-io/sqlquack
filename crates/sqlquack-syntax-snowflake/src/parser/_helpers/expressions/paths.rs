//! Parser expressions: paths.

use crate::parser::constants::IDENTIFIER;
use crate::parser::constants::USER;
use crate::parser::models::Parser;
use sqlquack_syntax::expressions::columns::models::Comprehension;
use sqlquack_syntax::expressions::functions::models::Function;
use sqlquack_syntax::expressions::functions::models::JSONExtract;
use sqlquack_syntax::expressions::names::models::BooleanLiteral;
use sqlquack_syntax::expressions::names::models::Column;
use sqlquack_syntax::expressions::names::models::Identifier;
use sqlquack_syntax::expressions::names::models::Literal;
use sqlquack_syntax::expressions::operators::models::Alias;
use sqlquack_syntax::expressions::operators::models::DotAccess;
use sqlquack_syntax::expressions::operators::models::LambdaExpr;
use sqlquack_syntax::expressions::operators::models::Subscript;
use sqlquack_syntax::expressions::queries::models::Limit;
use sqlquack_syntax::expressions::queries::models::Offset;
use sqlquack_syntax::expressions::queries::models::Pseudocolumn;
use sqlquack_syntax::expressions::queries::models::PseudocolumnType;
use sqlquack_syntax::expressions::queries::models::Subquery;
use sqlquack_syntax::expressions::statements::models::Paren;
use sqlquack_syntax::expressions::statements::models::Raw;
use sqlquack_syntax::expressions::tree::models::Expression;
use sqlquack_syntax::expressions::typed_functions::models::ArrayConstructor;
use sqlquack_syntax::expressions::types::models::Tuple;
use sqlquack_syntax::tokens::models::SourceSpan;
use sqlquack_syntax::tokens::models::TokenType;
use sqlquack_syntax::tokens::types::Result;

impl Parser {
    #[inline(never)]
    pub(crate) fn parse_colon_json_path_inner(
        &mut self,
        mut this: Expression,
    ) -> Result<Expression> {
        if matches!(
            self.config.dialect,
            Some(sqlquack_syntax::tokens::models::DialectType::DuckDB)
        ) {
            return Ok(this);
        }

        let is_valid_json_path_base = matches!(
            &this,
            Expression::Column(_)
                | Expression::Identifier(_)
                | Expression::Dot(_)
                | Expression::JSONExtract(_)
                | Expression::Subscript(_)
                | Expression::Function(_)
                | Expression::ParseJson(_)
                | Expression::Parameter(_)
                | Expression::Paren(_)
        );

        if !is_valid_json_path_base {
            return Ok(this);
        }

        if !self.check(TokenType::Colon) {
            return Ok(this);
        }

        if self.check_next(TokenType::Colon) {
            return Ok(this);
        }

        let mut path_string = String::new();

        while self.check(TokenType::Colon) && !self.check_next(TokenType::Colon) {
            let saved_pos = self.current;
            let saved_path_len = path_string.len();

            self.skip();

            let mut had_initial_component = false;
            if self.check(TokenType::QuotedIdentifier) {
                path_string = self.parse_json_path_quoted_head(path_string)?;
                had_initial_component = true;
            } else if self.is_identifier_token()
                || self.is_safe_keyword_as_identifier()
                || self.is_reserved_keyword_as_identifier()
            {
                if !path_string.is_empty() {
                    path_string.push('.');
                }
                let first_part = self.advance_text()?;
                path_string.push_str(&first_part);
                had_initial_component = true;
            } else if self.check(TokenType::LBracket) {
                had_initial_component = true;
            } else {
                self.refuse_snowflake_string_path()?;
            }

            if !had_initial_component {
                self.current = saved_pos;
                path_string.truncate(saved_path_len);
                break;
            }

            loop {
                if self.match_token(TokenType::LBracket) {
                    if self.check(TokenType::Number) && self.check_next(TokenType::RBracket) {
                        path_string.push('[');
                        let idx = self.advance_text()?;
                        path_string.push_str(&idx);
                        self.expect_token(TokenType::RBracket)?;
                        path_string.push(']');
                    } else if self.check(TokenType::Star) {
                        path_string.push('[');
                        self.skip();
                        path_string.push('*');
                        self.expect_token(TokenType::RBracket)?;
                        path_string.push(']');
                    } else if self.check(TokenType::String) && self.check_next(TokenType::RBracket)
                    {
                        path_string = self.parse_json_path_bracket_string_key(path_string)?;
                    } else if self.check(TokenType::QuotedIdentifier) {
                        let key = self.advance_text()?;
                        self.expect_token(TokenType::RBracket)?;
                        path_string.push_str("[\"");
                        for c in key.chars() {
                            if c == '"' {
                                path_string.push_str("\\\"");
                            } else {
                                path_string.push(c);
                            }
                        }
                        path_string.push_str("\"]");
                    } else if self.is_identifier_token()
                        || self.config.dialect
                            == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
                    {
                        let saved_bracket_pos = self.current;
                        let ident_text = self.advance_text()?;
                        if self.check(TokenType::Dot)
                            || self.config.dialect
                                == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
                        {
                            (path_string, this) = self.parse_json_path_bracket_subscript(
                                path_string,
                                saved_bracket_pos,
                                this,
                            )?;

                            break;
                        } else {
                            path_string.push('[');
                            path_string.push_str(&ident_text);
                            self.expect_token(TokenType::RBracket)?;
                            path_string.push(']');
                        }
                    } else {
                        path_string.push('[');
                        self.expect_token(TokenType::RBracket)?;
                        path_string.push(']');
                    }
                } else if self.match_token(TokenType::Dot) {
                    path_string = self.parse_json_path_dot_key(path_string)?;
                } else {
                    break;
                }
            }
        }

        if path_string.is_empty() {
            return Ok(this);
        }

        let path_expr = Expression::Literal(Box::new(Literal::String(path_string)));
        let json_extract = Expression::JSONExtract(Box::new(JSONExtract {
            this: Box::new(this),
            expression: Box::new(path_expr),
            only_json_types: None,
            expressions: Vec::new(),
            variant_extract: Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            }))),
            json_query: None,
            option: None,
            quote: None,
            on_condition: None,
            requires_json: None,
        }));

        Ok(json_extract)
    }

    fn parse_json_path_bracket_string_key(&mut self, mut path_string: String) -> Result<String> {
        let key = self.advance_text()?;
        self.expect_token(TokenType::RBracket)?;
        let needs_brackets = key.contains(' ')
            || key.contains('"')
            || key.contains('\'')
            || self.faithful_path_key_needs_brackets(&key);
        if needs_brackets {
            path_string.push_str("[\"");
            for c in key.chars() {
                if c == '"' {
                    path_string.push_str("\\\"");
                } else {
                    path_string.push(c);
                }
            }
            path_string.push_str("\"]");
        } else {
            if !path_string.is_empty() {
                path_string.push('.');
            }
            path_string.push_str(&key);
        }
        Ok(path_string)
    }

    fn parse_json_path_bracket_subscript(
        &mut self,
        mut path_string: String,
        saved_bracket_pos: usize,
        mut this: Expression,
    ) -> Result<(String, Expression)> {
        self.current = saved_bracket_pos;
        let index_expr = self.parse_expression()?;
        self.expect_token(TokenType::RBracket)?;

        let path_expr = Expression::Literal(Box::new(Literal::String(path_string)));
        let json_extract = Expression::JSONExtract(Box::new(JSONExtract {
            this: Box::new(this),
            expression: Box::new(path_expr),
            only_json_types: None,
            expressions: Vec::new(),
            variant_extract: Some(Box::new(Expression::Boolean(BooleanLiteral {
                value: true,
            }))),
            json_query: None,
            option: None,
            quote: None,
            on_condition: None,
            requires_json: None,
        }));

        let subscript = Expression::Subscript(Box::new(Subscript {
            this: json_extract,
            index: index_expr,
        }));

        let mut suffix_path = String::new();
        loop {
            if self.match_token(TokenType::Dot) {
                if !suffix_path.is_empty() {
                    suffix_path.push('.');
                }
                if self.is_identifier_token()
                    || self.is_safe_keyword_as_identifier()
                    || self.is_reserved_keyword_as_identifier()
                {
                    let part = self.advance_text()?;
                    suffix_path.push_str(&part);
                } else {
                    return Err(self.parse_error("Expected identifier after . in JSON path"));
                }
            } else {
                break;
            }
        }

        let result_base = if suffix_path.is_empty() {
            subscript
        } else {
            Expression::JSONExtract(Box::new(JSONExtract {
                this: Box::new(subscript),
                expression: Box::new(Expression::Literal(Box::new(Literal::String(suffix_path)))),
                only_json_types: None,
                expressions: Vec::new(),
                variant_extract: Some(Box::new(Expression::Boolean(BooleanLiteral {
                    value: true,
                }))),
                json_query: None,
                option: None,
                quote: None,
                on_condition: None,
                requires_json: None,
            }))
        };

        if self.match_token(TokenType::LBracket) {
            let index_expr2 = self.parse_expression()?;
            self.expect_token(TokenType::RBracket)?;
            let subscript2 = Expression::Subscript(Box::new(Subscript {
                this: result_base,
                index: index_expr2,
            }));
            this = subscript2;
            path_string = String::new();
        } else {
            this = result_base;
            path_string = String::new();
        }
        Ok((path_string, this))
    }

    fn parse_json_path_quoted_head(&mut self, mut path_string: String) -> Result<String> {
        let quoted_name = self.advance_text()?;
        let is_snowflake = matches!(
            self.config.dialect,
            Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
        );
        let needs_bracket = quoted_name.contains(' ')
            || quoted_name.contains('\'')
            || self.faithful_path_key_needs_brackets(&quoted_name);
        if is_snowflake && !needs_bracket {
            if !path_string.is_empty() {
                path_string.push('.');
            }
            path_string.push_str(&quoted_name);
        } else if is_snowflake && needs_bracket {
            path_string.push_str("[\"");
            path_string.push_str(&quoted_name);
            path_string.push_str("\"]");
        } else {
            path_string.push_str("[\"");
            for c in quoted_name.chars() {
                if c == '"' {
                    path_string.push_str("\\\"");
                } else {
                    path_string.push(c);
                }
            }
            path_string.push_str("\"]");
        }
        Ok(path_string)
    }

    fn parse_json_path_dot_key(&mut self, mut path_string: String) -> Result<String> {
        path_string.push('.');
        if self.config.faithful_calls && self.check(TokenType::QuotedIdentifier) {
            let part = self.advance_text()?;
            if self.faithful_path_key_needs_brackets(&part) || part.contains(' ') {
                path_string.pop();
                path_string.push_str("[\"");
                path_string.push_str(&part.replace('"', "\\\""));
                path_string.push_str("\"]");
            } else {
                path_string.push_str(&part);
            }
        } else if self.is_identifier_token()
            || self.is_safe_keyword_as_identifier()
            || self.is_reserved_keyword_as_identifier()
        {
            let part = self.advance_text()?;
            path_string.push_str(&part);
        } else {
            return Err(self.parse_error("Expected identifier after . in JSON path"));
        }
        Ok(path_string)
    }

    /// Check if the current token is a reserved keyword that can be used as identifier in JSON path
    pub(crate) fn is_reserved_keyword_as_identifier(&self) -> bool {
        if self.is_at_end() {
            return false;
        }
        let token = self.peek();
        matches!(
            token.token_type,
            TokenType::From
                | TokenType::Select
                | TokenType::Where
                | TokenType::And
                | TokenType::Or
                | TokenType::Not
                | TokenType::In
                | TokenType::As
                | TokenType::On
                | TokenType::Join
                | TokenType::Left
                | TokenType::Right
                | TokenType::Inner
                | TokenType::Outer
                | TokenType::Cross
                | TokenType::Full
                | TokenType::Group
                | TokenType::Order
                | TokenType::By
                | TokenType::Having
                | TokenType::Limit
                | TokenType::Offset
                | TokenType::Union
                | TokenType::Except
                | TokenType::Intersect
                | TokenType::All
                | TokenType::Distinct
                | TokenType::Case
                | TokenType::When
                | TokenType::Then
                | TokenType::Else
                | TokenType::End
                | TokenType::Null
                | TokenType::True
                | TokenType::False
                | TokenType::Between
                | TokenType::Like
                | TokenType::Is
                | TokenType::Exists
                | TokenType::Insert
                | TokenType::Update
                | TokenType::Delete
                | TokenType::Create
                | TokenType::Alter
                | TokenType::Drop
                | TokenType::Table
                | TokenType::View
                | TokenType::Index
                | TokenType::Set
                | TokenType::Values
                | TokenType::Into
                | TokenType::Default
                | TokenType::Key
                | TokenType::Unique
                | TokenType::Check
                | TokenType::Constraint
                | TokenType::References
        )
    }

    pub(crate) fn maybe_parse_clickhouse_array_alias(
        &mut self,
        expr: Expression,
    ) -> Result<Expression> {
        {
            Ok(expr)
        }
    }

    pub(crate) fn build_bracket_array(expressions: Vec<Expression>) -> Expression {
        Expression::ArrayFunc(Box::new(ArrayConstructor {
            expressions,
            bracket_notation: true,
            use_list_keyword: false,
            inferred_type: None,
        }))
    }

    pub(crate) fn parse_bracket_array_literal(&mut self) -> Result<Expression> {
        struct ArrayFrame {
            expressions: Vec<Expression>,
        }

        let mut frames = vec![ArrayFrame {
            expressions: Vec::new(),
        }];
        let mut completed_expr: Option<Expression> = None;

        loop {
            if let Some(expr) = completed_expr.take() {
                let expr = self.maybe_parse_clickhouse_array_alias(expr)?;
                let expr = self.maybe_parse_subscript(expr)?;
                if let Some(frame) = frames.last_mut() {
                    frame.expressions.push(expr);
                } else {
                    return Err(self.parse_error("Unbalanced array literal"));
                }

                if self.match_token(TokenType::Comma) {
                    continue;
                }

                self.expect_token(TokenType::RBracket)?;
                let frame = frames
                    .pop()
                    .ok_or_else(|| self.parse_error("Unbalanced array literal"))?;
                let array_expr = Self::build_bracket_array(frame.expressions);

                if frames.is_empty() {
                    return self.maybe_parse_subscript(array_expr);
                }

                completed_expr = Some(array_expr);
                continue;
            }

            if self.match_token(TokenType::RBracket) {
                let frame = frames
                    .pop()
                    .ok_or_else(|| self.parse_error("Unbalanced array literal"))?;
                let array_expr = Self::build_bracket_array(frame.expressions);

                if frames.is_empty() {
                    return self.maybe_parse_subscript(array_expr);
                }

                completed_expr = Some(array_expr);
                continue;
            }

            if self.match_token(TokenType::LBracket) {
                frames.push(ArrayFrame {
                    expressions: Vec::new(),
                });
                continue;
            }

            let expr = self.parse_expression()?;

            if frames
                .last()
                .is_some_and(|frame| frame.expressions.is_empty())
                && self.match_token(TokenType::For)
            {
                let loop_var = self.parse_primary()?;

                let position = if self.match_token(TokenType::Comma) {
                    Some(self.parse_primary()?)
                } else {
                    None
                };

                if !self.match_token(TokenType::In) {
                    return Err(self.parse_error("Expected IN in comprehension"));
                }

                let iterator = self.parse_expression()?;
                let condition = if self.match_token(TokenType::If) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };

                self.expect_token(TokenType::RBracket)?;

                let comprehension = Expression::Comprehension(Box::new(Comprehension {
                    this: Box::new(expr),
                    expression: Box::new(loop_var),
                    position: position.map(Box::new),
                    iterator: Some(Box::new(iterator)),
                    condition: condition.map(Box::new),
                }));

                if frames.len() == 1 {
                    return Ok(comprehension);
                }

                frames
                    .pop()
                    .ok_or_else(|| self.parse_error("Unbalanced array comprehension"))?;
                completed_expr = Some(comprehension);
                continue;
            }

            completed_expr = Some(expr);
        }
    }

    /// Parses the rest of a parenthesized primary; a set operation may follow when allowed.
    pub(crate) fn parse_parenthesized_primary(
        &mut self,
        lparen_comments: Vec<String>,
        set_operation_may_follow: bool,
    ) -> Result<Expression> {
        let raw_start = self.previous().span.start;

        if self.check(TokenType::RParen) {
            return self.parse_empty_parentheses();
        }

        if self.check(TokenType::Values) {
            return self.parse_parenthesized_values();
        }

        let is_explain_subquery = self.check(TokenType::Var)
            && self.peek_text().eq_ignore_ascii_case("EXPLAIN")
            && self.peek_nth(1).is_some_and(|t| {
                matches!(
                    t.token_type,
                    TokenType::Select
                        | TokenType::Insert
                        | TokenType::Create
                        | TokenType::Alter
                        | TokenType::Drop
                        | TokenType::Set
                        | TokenType::System
                        | TokenType::Table
                ) || matches!(
                    t.text.to_ascii_uppercase().as_str(),
                    "SYNTAX" | "AST" | "PLAN" | "PIPELINE" | "ESTIMATE" | "CURRENT" | "QUERY"
                ) || t.token_type == TokenType::Var
                    && self
                        .peek_nth(2)
                        .is_some_and(|t2| t2.token_type == TokenType::Eq)
            });

        if self.check(TokenType::Select)
            || self.check(TokenType::With)
            || self.check(TokenType::From)
            || is_explain_subquery
        {
            return self.parse_parenthesized_query(is_explain_subquery, set_operation_may_follow);
        }

        if self.check(TokenType::LParen) {
            return self.parse_doubly_parenthesized(lparen_comments, set_operation_may_follow);
        }

        let expr = self.parse_expression()?;

        let first_expr = if self.match_token(TokenType::As) {
            let alias = self.expect_identifier_or_keyword_with_quoted()?;
            Expression::Alias(Box::new(Alias::new(expr, alias)))
        } else {
            expr
        };

        if self.match_token(TokenType::Comma) {
            return self.parse_parenthesized_tuple(first_expr, raw_start);
        }

        self.expect_token(TokenType::RParen)?;
        let mut paren_comments = lparen_comments.clone();
        paren_comments.extend_from_slice(self.previous_trailing_comments());

        if !matches!(
            self.config.dialect,
            Some(sqlquack_syntax::tokens::models::DialectType::DuckDB)
                | Some(sqlquack_syntax::tokens::models::DialectType::SQLite)
        ) && self.match_token(TokenType::Arrow)
        {
            let parameters = if let Expression::Column(c) = first_expr {
                vec![c.name]
            } else if let Expression::Identifier(id) = first_expr {
                vec![id]
            } else {
                return Err(self.parse_error("Expected identifier as lambda parameter"));
            };
            let body = self.parse_expression()?;
            return Ok(Expression::Lambda(Box::new(LambdaExpr {
                parameters,
                body,
                colon: false,
                parameter_types: Vec::new(),
            })));
        }

        self.maybe_parse_over(Expression::Paren(Box::new(Paren {
            this: first_expr,
            trailing_comments: paren_comments,
            span: SourceSpan::default(),
        })))
    }

    fn parse_empty_parentheses(&mut self) -> Result<Expression> {
        self.skip();
        if self.config.dialect == Some(sqlquack_syntax::tokens::models::DialectType::Snowflake) {
            return Err(self.parse_error("Expected an expression in the parentheses"));
        }
        if self.config.dialect != Some(sqlquack_syntax::tokens::models::DialectType::DuckDB)
            && (self.match_token(TokenType::Arrow) || self.match_token(TokenType::FArrow))
        {
            let body = self.parse_expression()?;
            return Ok(Expression::Lambda(Box::new(LambdaExpr {
                parameters: Vec::new(),
                body,
                colon: false,
                parameter_types: Vec::new(),
            })));
        }
        self.maybe_parse_subscript(Expression::Tuple(Box::new(Tuple {
            expressions: Vec::new(),
        })))
    }

    fn parse_parenthesized_values(&mut self) -> Result<Expression> {
        let values = self.parse_values()?;
        self.expect_token(TokenType::RParen)?;
        Ok(Expression::Subquery(Box::new(Subquery {
            this: values,
            alias: None,
            column_aliases: Vec::new(),
            alias_explicit_as: false,
            alias_keyword: None,
            order_by: None,
            limit: None,
            offset: None,
            distribute_by: None,
            sort_by: None,
            cluster_by: None,
            lateral: false,
            modifiers_inside: false,
            trailing_comments: self.previous_trailing_comments().to_vec(),
            inferred_type: None,
            span: SourceSpan::default(),
        })))
    }

    fn parse_parenthesized_query(
        &mut self,
        is_explain_subquery: bool,
        set_operation_may_follow: bool,
    ) -> Result<Expression> {
        let query = if is_explain_subquery {
            self.parse_clickhouse_command_until_rparen()?
        } else {
            self.parse_statement()?
        };

        let limit = if self.match_token(TokenType::Limit) {
            Some(Limit {
                this: self.parse_expression()?,
                percent: false,
                comments: Vec::new(),
            })
        } else {
            None
        };
        let offset = if self.match_token(TokenType::Offset) {
            Some(Offset {
                this: self.parse_expression()?,
                rows: None,
            })
        } else {
            None
        };

        self.expect_token(TokenType::RParen)?;

        let subquery = if limit.is_some() || offset.is_some() {
            Expression::Subquery(Box::new(Subquery {
                this: query,
                alias: None,
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                order_by: None,
                limit,
                offset,
                distribute_by: None,
                sort_by: None,
                cluster_by: None,
                lateral: false,
                modifiers_inside: true,
                trailing_comments: self.previous_trailing_comments().to_vec(),
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        } else {
            Expression::Subquery(Box::new(Subquery {
                this: query,
                alias: None,
                column_aliases: Vec::new(),
                alias_explicit_as: false,
                alias_keyword: None,
                order_by: None,
                limit: None,
                offset: None,
                distribute_by: None,
                sort_by: None,
                cluster_by: None,
                lateral: false,
                modifiers_inside: false,
                trailing_comments: self.previous_trailing_comments().to_vec(),
                inferred_type: None,
                span: SourceSpan::default(),
            }))
        };

        let set_result = if set_operation_may_follow {
            self.parse_set_operation(subquery)?
        } else {
            subquery
        };
        let had_set_operation = matches!(
            &set_result,
            Expression::Union(_) | Expression::Intersect(_) | Expression::Except(_)
        );

        let result = if had_set_operation {
            let order_by = if self.check(TokenType::Order) {
                self.expect_token(TokenType::Order)?;
                self.expect_token(TokenType::By)?;
                Some(self.parse_order_by()?)
            } else {
                None
            };
            let limit_after = if self.match_token(TokenType::Limit) {
                Some(Limit {
                    this: self.parse_expression()?,
                    percent: false,
                    comments: Vec::new(),
                })
            } else {
                None
            };
            let offset_after = if self.match_token(TokenType::Offset) {
                Some(Offset {
                    this: self.parse_expression()?,
                    rows: None,
                })
            } else {
                None
            };

            if order_by.is_some() || limit_after.is_some() || offset_after.is_some() {
                Expression::Subquery(Box::new(Subquery {
                    this: set_result,
                    alias: None,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    order_by,
                    limit: limit_after,
                    offset: offset_after,
                    lateral: false,
                    modifiers_inside: false,
                    trailing_comments: Vec::new(),
                    distribute_by: None,
                    sort_by: None,
                    cluster_by: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else {
                set_result
            }
        } else {
            set_result
        };
        self.maybe_parse_subscript(result)
    }

    /// Parses `((...` past its first parenthesis, where a parenthesised set operation may start a longer one.
    fn parse_doubly_parenthesized(
        &mut self,
        lparen_comments: Vec<String>,
        set_operation_may_follow: bool,
    ) -> Result<Expression> {
        let expr = self.parse_expression()?;

        let first_expr = if self.match_token(TokenType::As) {
            let alias = self.expect_identifier_or_alias_keyword_with_quoted()?;
            Expression::Alias(Box::new(Alias::new(expr, alias)))
        } else {
            expr
        };

        if self.match_token(TokenType::Comma) {
            let mut expressions = vec![first_expr];
            loop {
                if self.check(TokenType::RParen) {
                    break;
                }
                let elem = self.parse_expression()?;
                let elem = if self.match_token(TokenType::As) {
                    let alias = self.expect_identifier_or_keyword()?;
                    Expression::Alias(Box::new(Alias::new(elem, Identifier::new(alias))))
                } else {
                    elem
                };
                expressions.push(elem);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            self.expect_token(TokenType::RParen)?;
            let tuple_expr = Expression::Tuple(Box::new(Tuple { expressions }));
            return self.maybe_parse_subscript(tuple_expr);
        }

        let result = first_expr;

        self.expect_token(TokenType::RParen)?;
        let mut nested_paren_comments = lparen_comments.clone();
        nested_paren_comments.extend_from_slice(self.previous_trailing_comments());
        if set_operation_may_follow
            && (self.check(TokenType::Union)
                || self.check(TokenType::Intersect)
                || self.check(TokenType::Except))
        {
            let left = match &result {
                Expression::Subquery(subq) => Some(subq.this.clone()),
                Expression::Union(_) | Expression::Intersect(_) | Expression::Except(_) => {
                    Some(result.clone())
                }
                _ => None,
            };
            if let Some(left) = left {
                let set_result = self.parse_set_operation(left)?;

                let order_by = if self.check(TokenType::Order) {
                    self.expect_token(TokenType::Order)?;
                    self.expect_token(TokenType::By)?;
                    Some(self.parse_order_by()?)
                } else {
                    None
                };
                let limit = if self.match_token(TokenType::Limit) {
                    Some(Limit {
                        this: self.parse_expression()?,
                        percent: false,
                        comments: Vec::new(),
                    })
                } else {
                    None
                };
                let offset = if self.match_token(TokenType::Offset) {
                    Some(Offset {
                        this: self.parse_expression()?,
                        rows: None,
                    })
                } else {
                    None
                };

                return Ok(Expression::Subquery(Box::new(Subquery {
                    this: set_result,
                    alias: None,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    order_by,
                    limit,
                    offset,
                    lateral: false,
                    modifiers_inside: false,
                    trailing_comments: Vec::new(),
                    distribute_by: None,
                    sort_by: None,
                    cluster_by: None,
                    inferred_type: None,
                    span: SourceSpan::default(),
                })));
            }
        }
        self.maybe_parse_over(Expression::Paren(Box::new(Paren {
            this: result,
            trailing_comments: nested_paren_comments,
            span: SourceSpan::default(),
        })))
    }

    fn parse_parenthesized_tuple(
        &mut self,
        first_expr: Expression,
        raw_start: usize,
    ) -> Result<Expression> {
        let mut expressions = vec![first_expr];
        if self.check(TokenType::RParen) {
            self.skip();
            let tuple_expr = Expression::Tuple(Box::new(Tuple { expressions }));
            return self.maybe_parse_subscript(tuple_expr);
        }

        loop {
            let elem = self.parse_expression()?;
            let elem_with_alias = if self.match_token(TokenType::As) {
                let alias = self.expect_identifier_or_keyword_with_quoted()?;
                Expression::Alias(Box::new(Alias::new(elem, alias)))
            } else {
                elem
            };
            expressions.push(elem_with_alias);
            if !self.match_token(TokenType::Comma) {
                break;
            }
            if self.check(TokenType::RParen) {
                break;
            }
        }

        self.expect_token(TokenType::RParen)?;

        if self.config.dialect != Some(sqlquack_syntax::tokens::models::DialectType::DuckDB)
            && self.match_token(TokenType::Arrow)
        {
            let expression_count = expressions.len();
            let parameters: Vec<Identifier> = expressions
                .into_iter()
                .filter_map(|e| {
                    if let Expression::Column(c) = e {
                        Some(c.name)
                    } else if let Expression::Identifier(id) = e {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect();
            let body = self.parse_expression()?;
            if parameters.len() != expression_count {
                return Ok(Expression::Raw(Raw {
                    sql: self.source_text_range(raw_start, self.previous().span.end),
                }));
            }
            return Ok(Expression::Lambda(Box::new(LambdaExpr {
                parameters,
                body,
                colon: false,
                parameter_types: Vec::new(),
            })));
        }

        let tuple_expr = Expression::Tuple(Box::new(Tuple { expressions }));
        let result = if self.check(TokenType::As) {
            let after_as = self.current + 1;
            let after_ident = self.current + 2;
            let is_type_constructor = after_ident < self.tokens.len()
                && (self.tokens[after_as].token_type == TokenType::Identifier
                    || self.tokens[after_as].token_type == TokenType::Var
                    || self.tokens[after_as].token_type == TokenType::Nullable
                    || self.tokens[after_as].token_type == TokenType::Struct
                    || self.tokens[after_as].token_type == TokenType::Array)
                && (self.tokens[after_ident].token_type == TokenType::LParen
                    || self.tokens[after_ident].token_type == TokenType::Lt);
            let is_cast_type = after_ident < self.tokens.len()
                && (self.tokens[after_as].token_type == TokenType::Identifier
                    || self.tokens[after_as].token_type == TokenType::Var
                    || self.tokens[after_as].token_type.is_keyword())
                && self.tokens[after_ident].token_type == TokenType::RParen;
            if is_type_constructor || is_cast_type {
                tuple_expr
            } else {
                self.skip();
                let alias = self.expect_identifier()?;
                Expression::Alias(Box::new(Alias::new(tuple_expr, Identifier::new(alias))))
            }
        } else {
            tuple_expr
        };

        self.maybe_parse_subscript(result)
    }

    pub(crate) fn parse_identifier_primary(&mut self) -> Result<Expression> {
        let upper_name = self.peek_text().to_ascii_uppercase();

        if (!self.check_next(TokenType::LParen))
            && !self.check_next(TokenType::Dot)
            && (sqlquack_syntax::functions::names::main::is_no_paren_function_name_upper::is_no_paren_function_name_upper(upper_name.as_str())
                || upper_name == USER
                    && self.peek().token_type != TokenType::QuotedIdentifier
                    && sqlquack_syntax::functions::names::main::bare_user_is_function::bare_user_is_function(self.config.dialect))
        {
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
                span: SourceSpan::default(),
                inferred_type: None,
                export_state: false,
            }));
            return self.maybe_parse_subscript(func);
        }

        let name_start = self.peek().span.start;
        let ident = self.expect_identifier_with_quoted()?;
        let name = ident.name.clone();
        let quoted = ident.quoted;

        if self.match_token(TokenType::LParen) {
            let upper_name = name.to_ascii_uppercase();

            let func_expr = self.parse_typed_function(&name, &upper_name, quoted)?;
            let func_expr = self.maybe_parse_clickhouse_parameterized_agg(func_expr)?;
            if matches!(
                self.config.dialect,
                Some(sqlquack_syntax::tokens::models::DialectType::Snowflake)
            ) && upper_name == IDENTIFIER
                && self.check(TokenType::LParen)
            {
                let name_end = self.previous().span.end;
                let dynamic_name = self.source_text_range(name_start, name_end);
                self.skip();
                let args = self.parse_function_args_list()?;
                self.expect_token(TokenType::RParen)?;
                return self.maybe_parse_over(Expression::Function(Box::new(Function {
                    on_error: None,
                    qualified_name: Vec::new(),
                    source_dialect: None,

                    name: dynamic_name,
                    args,
                    distinct: false,
                    trailing_comments: Vec::new(),
                    use_bracket_syntax: false,
                    no_parens: false,
                    quoted: false,
                    span: SourceSpan::default(),
                    inferred_type: None,
                    export_state: false,
                })));
            }
            return self.maybe_parse_over(func_expr);
        }

        if self.match_token(TokenType::Dot) {
            if let Some(call) = self.parse_snowflake_default_schema_call(ident.clone())? {
                return self.maybe_parse_subscript(call);
            }
            self.refuse_snowflake_reserved_part()?;
            if self.match_token(TokenType::Star) {
                let star = self.parse_star_modifiers(Some(ident))?;
                let star_expr = Expression::Star(Box::new(star));

                return Ok(star_expr);
            }
            if self.check(TokenType::Number) {
                let field_name = self.advance_text()?;
                let col_expr = Expression::Dot(Box::new(DotAccess {
                    this: Self::parsed_column(Column {
                        name: ident,
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

            if self.check(TokenType::LParen) && self.check_next(TokenType::Plus) {
                let saved_pos = self.current;
                if self.match_token(TokenType::LParen)
                    && self.match_token(TokenType::Plus)
                    && self.match_token(TokenType::RParen)
                {
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    let col = Self::parsed_column(Column {
                        name: col_ident,
                        table: Some(ident),
                        join_mark: true,
                        trailing_comments,
                        span: None,
                        inferred_type: None,
                    });
                    return self.maybe_parse_subscript(col);
                } else {
                    self.current = saved_pos;
                }
            }

            if self.check(TokenType::LParen) {
                self.skip();
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_method_arguments_for(&col_ident.name)?
                };
                self.expect_token(TokenType::RParen)?;
                let this = Self::parsed_column(Column {
                    name: ident.clone(),
                    table: None,
                    join_mark: false,
                    trailing_comments: Vec::new(),
                    span: None,
                    inferred_type: None,
                });
                let method_call = self.parse_method_call_expression(this, col_ident, args);
                let method_call = self.parse_qualified_window(method_call)?;
                return self.maybe_parse_subscript(method_call);
            }

            let trailing_comments = self.previous_trailing_comments().to_vec();
            let col = Self::parsed_column(Column {
                name: col_ident,
                table: Some(ident),
                join_mark: false,
                trailing_comments,
                span: None,
                inferred_type: None,
            });
            return self.maybe_parse_subscript(col);
        }

        if !quoted && self.config.dialect.is_none() {
            if let Some(pseudocolumn_type) = PseudocolumnType::from_name(&name) {
                return Ok(Expression::Pseudocolumn(Pseudocolumn {
                    kind: pseudocolumn_type,
                }));
            }
        }

        if self.check(TokenType::Arrow)
            && !matches!(
                self.config.dialect,
                Some(sqlquack_syntax::tokens::models::DialectType::DuckDB)
                    | Some(sqlquack_syntax::tokens::models::DialectType::PostgreSQL)
                    | Some(sqlquack_syntax::tokens::models::DialectType::SQLite)
            )
            && !self
                .peek_nth(1)
                .is_some_and(|t| t.token_type == TokenType::String)
        {
            self.skip();
            let body = self.parse_expression()?;
            return Ok(Expression::Lambda(Box::new(LambdaExpr {
                parameters: vec![ident],
                body,
                colon: false,
                parameter_types: Vec::new(),
            })));
        }

        let trailing_comments = self.previous_trailing_comments().to_vec();
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

    /// Parse primary expressions
    #[inline(always)]
    pub(crate) fn parse_primary(&mut self) -> Result<Expression> {
        let start = self.current;
        let expr = self.with_parser_depth(|parser| parser.parse_primary_inner())?;
        Ok(self.spanned(start, expr))
    }

    #[inline(never)]
    pub(crate) fn parse_primary_inner(&mut self) -> Result<Expression> {
        if self.tokens.is_empty() {
            return Err(self.end_of_input_error());
        }
        self.refuse_snowflake_stage_reference()?;

        if self.check(TokenType::If)
            && !self.if_expr_ruled_out.contains(&self.current)
            && !self.check_next(TokenType::Dot)
            && (!self.check_next(TokenType::LParen))
        {
            let saved_pos = self.current;
            self.skip();
            if let Some(if_expr) = self.parse_if()? {
                return Ok(if_expr);
            }
            self.current = saved_pos;
            self.if_expr_ruled_out.insert(saved_pos);
        }

        self.parse_primary_slow()
    }
}
