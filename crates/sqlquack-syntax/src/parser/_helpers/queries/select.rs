//! Parser queries: select.

use crate::expressions::functions::models::Function;
use crate::expressions::functions::models::JSONTable;
use crate::expressions::names::models::BooleanLiteral;
use crate::expressions::names::models::Identifier;
use crate::expressions::names::models::TableRef;
use crate::expressions::operators::models::Alias;
use crate::expressions::operators::models::Lateral;
use crate::expressions::operators::models::Parameter;
use crate::expressions::operators::models::ParameterStyle;
use crate::expressions::queries::models::From;
use crate::expressions::queries::models::GroupBy;
use crate::expressions::queries::models::Having;
use crate::expressions::queries::models::JoinedTable;
use crate::expressions::queries::models::Limit;
use crate::expressions::queries::models::Offset;
use crate::expressions::queries::models::Select;
use crate::expressions::queries::models::Subquery;
use crate::expressions::queries::models::Where;
use crate::expressions::statements::models::Annotated;
use crate::expressions::statements::models::Paren;
use crate::expressions::statements::models::Var;
use crate::expressions::tree::models::Expression;
use crate::parser::constants::SECOND_PREVIOUS_OFFSET;
use crate::parser::models::Parser;
use crate::tokens::models::SourceSpan;
use crate::tokens::models::TokenType;
use crate::tokens::types::Result;

impl Parser {
    /// Parse SELECT expressions
    pub(crate) fn parse_select_expressions(&mut self) -> Result<Vec<Expression>> {
        let capacity_hint = self.estimate_select_expression_capacity();
        let mut expressions = Vec::with_capacity(capacity_hint);

        loop {
            if self.is_at_end()
                || self.check(TokenType::From)
                || self.check(TokenType::Where)
                || self.check(TokenType::Into)
                || self.check(TokenType::Union)
                || self.check(TokenType::Intersect)
                || self.check(TokenType::Except)
                || self.check(TokenType::Order)
                || self.check(TokenType::Limit)
                || self.check(TokenType::Qualify)
                || self.check(TokenType::Semicolon)
                || self.check_text_seq(&["BULK", "COLLECT", "INTO"])
            {
                break;
            }

            let item_start = self.current;
            if self.check(TokenType::Star) {
                let star_expr = if let Some(star_like) = self.parse_clickhouse_bare_star_like()? {
                    star_like
                } else {
                    self.skip();
                    let star_trailing_comments = self.previous_trailing_comments().to_vec();
                    let star =
                        self.parse_star_modifiers_with_comments(None, star_trailing_comments)?;
                    Expression::Star(star)
                };

                expressions.push(self.spanned(item_start, star_expr));
            } else {
                let leading_comments = self.current_leading_comments().to_vec();
                let expr = self.parse_expression()?;

                let expr = { expr };

                let pre_alias_comments = self.previous_trailing_comments().to_vec();

                let expr = if self.check(TokenType::Colon) && !self.check_next(TokenType::Colon) {
                    self.parse_select_item_colon_alias(expr, pre_alias_comments)?
                } else if self.match_token(TokenType::As) {
                    self.parse_select_item_as_alias(expr, leading_comments, pre_alias_comments)?
                } else if ((self.check(TokenType::Var) && !self.check_keyword()
                    || self.check(TokenType::QuotedIdentifier)
                    || self.check(TokenType::String) && self.supports_string_aliases(false)
                    || self.can_be_alias_keyword()
                    || self.is_command_keyword_as_alias()
                    || self.check(TokenType::Overlaps))
                    && !self.check_text_seq(&["BULK", "COLLECT", "INTO"]))
                    && !(self.check(TokenType::Fetch)
                        || self.check(TokenType::Limit)
                        || self.check(TokenType::Offset))
                    && !self.check_text_seq(&["GROUP", "BY"])
                    && !self.check_text_seq(&["ORDER", "BY"])
                    && !self.check(TokenType::Qualify)
                    && !(self.is_snowflake_reserved_keyword() && !self.check(TokenType::Window))
                    && !(self.check(TokenType::Window)
                        && self.peek_nth(1).is_some_and(|token| {
                            matches!(token.token_type, TokenType::Var | TokenType::Identifier)
                        }))
                {
                    let alias = self.parse_projection_alias_identifier(false)?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        pre_alias_comments,
                        trailing_comments,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }))
                } else if !pre_alias_comments.is_empty() {
                    self.parse_select_item_comment_alias(expr, pre_alias_comments)
                } else if !leading_comments.is_empty() {
                    Expression::Annotated(Box::new(Annotated {
                        this: expr,
                        trailing_comments: leading_comments,
                        type_assertion: None,
                        type_like: None,
                    }))
                } else {
                    expr
                };

                expressions.push(self.spanned(item_start, expr));
            }

            if !self.match_token(TokenType::Comma) {
                break;
            }

            if (self.config.allow_trailing_commas
                || matches!(
                    self.config.dialect,
                    Some(crate::tokens::models::DialectType::Snowflake)
                ))
                && (self.check_from_keyword()
                    || self.check(TokenType::Where)
                    || self.check(TokenType::GroupBy)
                    || self.check(TokenType::Having)
                    || self.check(TokenType::Order)
                    || self.check(TokenType::Limit)
                    || self.check(TokenType::Union)
                    || self.check(TokenType::Intersect)
                    || self.check(TokenType::Except)
                        && !self.check_next(TokenType::LParen)
                        && !self.check_next(TokenType::Comma)
                    || self.check(TokenType::Semicolon)
                    || self.check(TokenType::RParen)
                    || self.check(TokenType::Settings)
                        && !self.check_next(TokenType::LParen)
                        && !self.check_next(TokenType::LBracket)
                    || self.check(TokenType::Format) && !self.check_next(TokenType::LParen)
                    || self.is_at_end())
            {
                break;
            }
        }

        Ok(expressions)
    }

    fn parse_select_item_colon_alias(
        &mut self,
        expr: Expression,
        pre_alias_comments: Vec<String>,
    ) -> Result<Expression> {
        Ok({
            let alias_ident = match &expr {
                Expression::Identifier(id) => Some(id.clone()),
                Expression::Column(col) if col.table.is_none() => Some(col.name.clone()),
                _ => None,
            };
            if let Some(alias) = alias_ident {
                self.skip();
                let colon_comments = self.previous_trailing_comments().to_vec();
                let value = self.parse_expression()?;
                let value_trailing = self.previous_trailing_comments().to_vec();
                let mut all_trailing = pre_alias_comments.clone();
                all_trailing.extend(colon_comments);
                all_trailing.extend(value_trailing);
                Expression::Alias(Box::new(Alias {
                    this: value,
                    alias,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    pre_alias_comments: Vec::new(),
                    trailing_comments: all_trailing,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else {
                expr
            }
        })
    }

    fn parse_select_item_as_alias(
        &mut self,
        expr: Expression,
        leading_comments: Vec<String>,
        pre_alias_comments: Vec<String>,
    ) -> Result<Expression> {
        Ok({
            let as_token = self.previous();
            let alias_keyword = if self.source.is_some() {
                self.source_text_range(as_token.span.start, as_token.span.end)
            } else {
                as_token.text.to_string()
            };
            let alias_keyword = false.then_some(alias_keyword);
            let as_comments = self.previous_trailing_comments().to_vec();
            if self.match_token(TokenType::LParen) {
                let mut column_aliases: Vec<Identifier> = Vec::new();
                loop {
                    if let Some(col_expr) = self.parse_id_var()? {
                        if let Expression::Identifier(id) = col_expr {
                            column_aliases.push(id);
                        }
                    } else {
                        break;
                    }
                    if !self.match_token(TokenType::Comma) {
                        break;
                    }
                }
                self.match_token(TokenType::RParen);
                let mut trailing_comments = as_comments;
                trailing_comments.extend_from_slice(self.previous_trailing_comments());
                Expression::Alias(Box::new(Alias {
                    this: expr,
                    alias: Identifier::new(String::new()),
                    column_aliases,
                    alias_explicit_as: true,
                    alias_keyword: alias_keyword.clone(),
                    pre_alias_comments,
                    trailing_comments,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else {
                let duckdb_string_alias = self.config.dialect
                    == Some(crate::tokens::models::DialectType::DuckDB)
                    && self.check(TokenType::String);
                let alias = self.parse_projection_alias_identifier(true)?;
                let mut trailing_comments = if duckdb_string_alias {
                    as_comments
                } else {
                    Vec::new()
                };
                trailing_comments.extend_from_slice(self.previous_trailing_comments());
                if !self.pending_leading_comments.is_empty() {
                    trailing_comments.append(&mut self.pending_leading_comments);
                } else {
                    trailing_comments.extend(leading_comments.iter().cloned());
                }
                if duckdb_string_alias && alias.name.is_empty() {
                    let attached_comments = expr.comments();
                    let mut comments: Vec<_> = pre_alias_comments
                        .into_iter()
                        .filter(|comment| !attached_comments.contains(&comment.as_str()))
                        .collect();
                    comments.extend(trailing_comments);
                    if comments.is_empty() {
                        expr
                    } else {
                        Expression::Annotated(Box::new(Annotated {
                            this: expr,
                            trailing_comments: comments,
                            type_assertion: None,
                            type_like: None,
                        }))
                    }
                } else {
                    let column_aliases = { Vec::new() };
                    Expression::Alias(Box::new(Alias {
                        this: expr,
                        alias,
                        column_aliases,
                        alias_explicit_as: true,
                        alias_keyword,
                        pre_alias_comments,
                        trailing_comments,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }))
                }
            }
        })
    }

    fn parse_select_item_comment_alias(
        &mut self,
        expr: Expression,
        pre_alias_comments: Vec<String>,
    ) -> Expression {
        let already_has_trailing = matches!(
            &expr,
            Expression::Add(_)
                | Expression::Sub(_)
                | Expression::Mul(_)
                | Expression::Div(_)
                | Expression::Mod(_)
                | Expression::Concat(_)
                | Expression::BitwiseAnd(_)
                | Expression::BitwiseOr(_)
                | Expression::BitwiseXor(_)
                | Expression::Column(_)
                | Expression::Paren(_)
                | Expression::Annotated(_)
                | Expression::Cast(_)
                | Expression::Function(_)
                | Expression::Subquery(_)
        );
        if already_has_trailing {
            expr
        } else {
            Expression::Annotated(Box::new(Annotated {
                this: expr,
                trailing_comments: pre_alias_comments,
                type_assertion: None,
                type_like: None,
            }))
        }
    }

    /// Parses DuckDB FROM-first queries: `FROM t` and `FROM t SELECT c`.
    pub(crate) fn parse_from_first_query(&mut self) -> Result<Expression> {
        self.expect_token(TokenType::From)?;

        let from = self.parse_from()?;

        let joins = self.parse_joins()?;

        let expressions = if self.check(TokenType::Select) {
            self.skip();
            self.parse_select_expressions()?
        } else {
            vec![Expression::Star(crate::expressions::names::models::Star {
                table: None,
                ilike: None,
                except: None,
                replace: None,
                rename: None,
                trailing_comments: Vec::new(),
                span: None,
                qualified: None,
            })]
        };

        let prewhere = if self.match_token(TokenType::Prewhere) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let where_clause = if self.match_token(TokenType::Where) {
            Some(Where {
                this: self.parse_expression()?,
            })
        } else {
            None
        };

        let group_by = if self.match_token(TokenType::Group) {
            self.expect_token(TokenType::By)?;
            let mut groups: Vec<Expression> = Vec::new();
            loop {
                groups.push(self.parse_expression()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
            Some(GroupBy {
                expressions: groups,
                all: None,
                totals: false,
                comments: Vec::new(),
            })
        } else {
            None
        };

        let having = if self.match_token(TokenType::Having) {
            Some(Having {
                this: self.parse_expression()?,
                comments: Vec::new(),
            })
        } else {
            None
        };

        let order_by = if self.match_token(TokenType::Order) {
            self.expect_token(TokenType::By)?;
            Some(self.parse_order_by()?)
        } else {
            None
        };

        let limit = if self.match_token(TokenType::Limit) {
            let first_expr = self.parse_expression()?;
            Some(Limit {
                this: first_expr,
                percent: false,
                comments: Vec::new(),
            })
        } else {
            None
        };

        let offset = if self.match_token(TokenType::Offset) {
            let expr = self.parse_expression()?;
            let rows = if self.match_token(TokenType::Row) || self.match_token(TokenType::Rows) {
                Some(true)
            } else {
                None
            };
            Some(Offset { this: expr, rows })
        } else {
            None
        };

        let select = Select {
            source_dialect: None,
            total_rowcount: false,

            query_collation: None,
            query_hints: Vec::new(),
            result_serialization: None,

            vertica: None,
            expressions,
            from: Some(from),
            joins,
            lateral_views: Vec::new(),
            prewhere,
            where_clause,
            group_by,
            having,
            qualify: None,
            order_by,
            distribute_by: None,
            cluster_by: None,
            sort_by: None,
            limit,
            offset,
            limit_by: None,
            fetch: None,
            distinct: false,
            all: false,
            distinct_on: None,
            top: None,
            with: None,
            sample: None,
            settings: None,
            format: None,
            windows: None,
            hint: None,
            connect: None,
            into: None,
            locks: Vec::new(),
            for_xml: Vec::new(),
            for_json: Vec::new(),

            leading_comments: Vec::new(),
            post_select_comments: Vec::new(),
            kind: None,
            operation_modifiers: Vec::new(),
            qualify_after_window: false,
            option: None,
            exclude: None,
            span: SourceSpan::default(),
        };

        let result = Expression::Select(Box::new(select));
        self.parse_set_operation(result)
    }

    /// Parse FROM clause
    pub(crate) fn parse_from(&mut self) -> Result<From> {
        let mut expressions: Vec<Expression> = Vec::new();

        loop {
            let pre_table_comments = if !self.is_at_end() {
                self.tokens[self.current].comments.clone()
            } else {
                Vec::new()
            };
            if !pre_table_comments.is_empty() && !self.is_at_end() {
                self.tokens[self.current].comments.clear();
            }

            let mut table = self.parse_table_expression()?;

            if !pre_table_comments.is_empty() {
                match &mut table {
                    Expression::Pivot(p) => {
                        if let Expression::Table(ref mut t) = p.this {
                            t.leading_comments = pre_table_comments;
                        }
                    }
                    Expression::Table(ref mut t) => {
                        t.trailing_comments.extend(pre_table_comments);
                    }
                    _ => {}
                }
            }
            expressions.push(table);

            if !self.match_token(TokenType::Comma) {
                break;
            }

            let is_unpivot_boundary = self.check(TokenType::Unpivot);
            if self.is_at_end()
                || is_unpivot_boundary
                || matches!(
                    self.peek().token_type,
                    TokenType::Where
                        | TokenType::GroupBy
                        | TokenType::Having
                        | TokenType::Order
                        | TokenType::Limit
                        | TokenType::Offset
                        | TokenType::Union
                        | TokenType::Intersect
                        | TokenType::Except
                        | TokenType::Semicolon
                        | TokenType::RParen
                        | TokenType::Window
                        | TokenType::Qualify
                        | TokenType::Distribute
                        | TokenType::Cluster
                        | TokenType::Pivot
                )
            {
                break;
            }
        }

        Ok(From { expressions })
    }

    /// Parse a table expression (table name, subquery, etc.)
    pub(crate) fn parse_table_expression(&mut self) -> Result<Expression> {
        let start = self.current;
        let expr = self.with_parser_depth(|parser| parser.parse_table_expression_inner())?;
        Ok(self.spanned(start, expr))
    }

    #[inline(never)]
    pub(crate) fn parse_table_expression_inner(&mut self) -> Result<Expression> {
        let has_only = self.match_token(TokenType::Only);

        if self.match_text_seq(&["ROWS", "FROM"]) {
            return self.parse_rows_from();
        }

        if self.match_token(TokenType::Unpivot) {
            return self.parse_redshift_unpivot_table();
        }

        let expr = if matches!(
            self.config.dialect,
            Some(crate::tokens::models::DialectType::DuckDB)
        ) && self.check(TokenType::LParen)
            && (self.check_next(TokenType::Select)
                || self.check_next(TokenType::With)
                || self.check_next(TokenType::From))
        {
            self.expect_token(TokenType::LParen)?;
            let query = self.parse_statement()?;
            self.expect_token(TokenType::RParen)?;
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
        } else {
            self.parse_table_expression_primary()?
        };
        self.parse_table_expression_tail(expr, has_only)
    }

    #[inline(never)]
    pub(crate) fn parse_table_expression_primary(&mut self) -> Result<Expression> {
        let expr = if self.check(TokenType::Values) && self.check_next(TokenType::LParen) {
            self.parse_values_with_set_operations(false)?
        } else {
            if self.check(TokenType::DAt) {
                self.parse_stage_reference()?
            } else if self.check(TokenType::Var) && self.peek_text().starts_with('@') {
                self.parse_stage_reference_from_var()?
            } else if self.check(TokenType::String) && self.peek_text().starts_with('@') {
                self.parse_stage_reference_from_string()?
            } else if self.match_token(TokenType::Lateral) {
                self.parse_table_lateral_source()?
            } else if self.match_token(TokenType::LParen) {
                let source = self.parse_table_parenthesized_source()?;
                self.refuse_snowflake_parenthesized_call(&source)?;
                source
            } else if self.is_identifier_token()
                || self.is_safe_keyword_as_identifier()
                || self.can_be_alias_keyword()
                || self.is_mysql_numeric_identifier()
                || self.check(TokenType::Pivot) && !self.check_next(TokenType::LParen)
                || self.check(TokenType::Unpivot) && !self.check_next(TokenType::LParen)
            {
                self.parse_table_named_source()?
            } else if self.check(TokenType::LBrace) {
                self.parse_table_brace_source()?
            } else if self.check(TokenType::Dollar) && self.check_next(TokenType::LBrace) {
                self.parse_table_dollar_brace_source()?
            } else if self.check(TokenType::String) || self.check(TokenType::DollarString) {
                self.parse_table_string_source()?
            } else {
                return Err(self.parse_error(format!(
                    "Expected table name or subquery, got {:?}",
                    self.peek().token_type
                )));
            }
        };

        Ok(expr)
    }

    fn parse_table_named_source(&mut self) -> Result<Expression> {
        Ok({
            if matches!(
                self.config.dialect,
                Some(crate::tokens::models::DialectType::DuckDB)
            ) && self.check_next(TokenType::Colon)
                && !(self.current + 2 < self.tokens.len()
                    && self.tokens[self.current + 2].token_type == TokenType::Colon)
            {
                return self.parse_table_duckdb_colon_source();
            }

            let first_ident = self.parse_bigquery_table_part()?;
            let first_name = first_ident.name.clone();
            if self.is_snowflake()
                && !first_ident.quoted
                && first_name.eq_ignore_ascii_case("ASOF")
                && self.check(TokenType::Join)
            {
                return Err(self.parse_error("Expected a table before ASOF JOIN"));
            }

            if self.match_token(TokenType::Dot) {
                self.parse_table_qualified_source(first_ident, first_name)?
            } else if self.match_token(TokenType::LParen) {
                if self.is_snowflake()
                    && self.check_next(TokenType::FArrow)
                    && ["TIMESTAMP", "OFFSET", "STATEMENT", "STREAM", "INFORMATION"]
                        .iter()
                        .any(|word| self.peek().text.eq_ignore_ascii_case(word))
                {
                    return Err(
                        self.parse_error("Expected AT, BEFORE or CHANGES before the clause")
                    );
                }
                self.refuse_snowflake_bare_table_call(&first_name)?;
                self.parse_table_function_source(first_name)?
            } else {
                let table_name = first_ident;

                let trailing_comments = self.previous_trailing_comments().to_vec();
                Expression::boxed_table(TableRef {
                    sample_call: None,
                    catalog: None,
                    schema: None,
                    name: table_name,
                    alias: None,
                    alias_explicit_as: false,
                    column_aliases: Vec::new(),
                    leading_comments: Vec::new(),
                    trailing_comments,
                    when: None,
                    only: false,
                    final_: false,
                    table_sample: None,
                    hints: Vec::new(),
                    system_time: None,
                    partitions: Vec::new(),
                    identifier_func: None,
                    changes: None,
                    version: None,
                    span: None,
                    whole_quoted: false,
                    catalog_parts: Vec::new(),
                })
            }
        })
    }

    fn parse_table_duckdb_colon_source(&mut self) -> Result<Expression> {
        let alias_ident = self.parse_bigquery_table_part()?;
        let pre_alias_comments = self.previous_trailing_comments().to_vec();
        self.expect_token(TokenType::Colon)?;
        let colon_comments = self.previous_trailing_comments().to_vec();
        let mut table_expr = self.parse_table_expression()?;
        let mut all_comments = pre_alias_comments;
        all_comments.extend(colon_comments);
        match &mut table_expr {
            Expression::Table(ref mut t) => {
                t.alias = Some(alias_ident);
                t.alias_explicit_as = true;
                if !all_comments.is_empty() {
                    let existing_comments = std::mem::take(&mut t.trailing_comments);
                    t.trailing_comments = all_comments;
                    t.trailing_comments.extend(existing_comments);
                }
            }
            Expression::Subquery(ref mut s) => {
                s.alias = Some(alias_ident);
            }
            Expression::Function(ref mut _f) => {
                return Ok(Expression::Alias(Box::new(Alias {
                    this: table_expr,
                    alias: alias_ident,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    pre_alias_comments: all_comments,
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                })));
            }
            _ => {
                return Ok(Expression::Alias(Box::new(Alias {
                    this: table_expr,
                    alias: alias_ident,
                    column_aliases: Vec::new(),
                    alias_explicit_as: false,
                    alias_keyword: None,
                    pre_alias_comments: all_comments,
                    trailing_comments: Vec::new(),
                    inferred_type: None,
                    span: SourceSpan::default(),
                })));
            }
        }
        Ok(table_expr)
    }

    fn parse_table_qualified_source(
        &mut self,
        first_ident: Identifier,
        first_name: String,
    ) -> Result<Expression> {
        Ok({
            if self.check(TokenType::Dot) {
                self.skip();
                let table_ident = self.parse_bigquery_table_part()?;
                let trailing_comments = self.previous_trailing_comments().to_vec();
                return Ok(Expression::boxed_table(TableRef {
                    sample_call: None,
                    catalog: Some(first_ident),
                    schema: Some(Identifier::new("")),
                    name: table_ident,
                    alias: None,
                    alias_explicit_as: false,
                    column_aliases: Vec::new(),
                    leading_comments: Vec::new(),
                    trailing_comments,
                    when: None,
                    only: false,
                    final_: false,
                    table_sample: None,
                    hints: Vec::new(),
                    system_time: None,
                    partitions: Vec::new(),
                    identifier_func: None,
                    changes: None,
                    version: None,
                    span: None,
                    whole_quoted: false,
                    catalog_parts: Vec::new(),
                }));
            }

            let second_ident = self.parse_bigquery_table_part()?;
            let second_name = second_ident.name.clone();

            if self.match_token(TokenType::Dot) {
                let third_ident = self.parse_bigquery_table_part()?;
                let third_name = third_ident.name.clone();

                if self.match_token(TokenType::Dot) {
                    let fourth_ident = self.parse_bigquery_table_part()?;
                    let table_name = fourth_ident;

                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Expression::boxed_table(TableRef {
                        sample_call: None,
                        catalog: Some(Identifier::new(format!("{}.{}", first_name, second_name))),
                        schema: Some(third_ident),
                        name: table_name,
                        alias: None,
                        alias_explicit_as: false,
                        column_aliases: Vec::new(),
                        leading_comments: Vec::new(),
                        trailing_comments,
                        when: None,
                        only: false,
                        final_: false,
                        table_sample: None,
                        hints: Vec::new(),
                        system_time: None,
                        partitions: Vec::new(),
                        identifier_func: None,
                        changes: None,
                        version: None,
                        span: None,
                        whole_quoted: false,
                        catalog_parts: Vec::new(),
                    })
                } else {
                    if self.match_token(TokenType::LParen) {
                        let args = if self.check(TokenType::RParen) {
                            Vec::new()
                        } else {
                            self.parse_function_arguments()?
                        };
                        self.expect_token(TokenType::RParen)?;
                        let trailing_comments = self.previous_trailing_comments().to_vec();
                        Expression::Function(Box::new(Function {
                            on_error: None,
                            qualified_name: Vec::new(),
                            source_dialect: None,

                            name: format!("{}.{}.{}", first_name, second_name, third_name),
                            args,
                            distinct: false,
                            trailing_comments,
                            use_bracket_syntax: false,
                            no_parens: false,
                            quoted: false,
                            span: SourceSpan::default(),
                            inferred_type: None,
                            export_state: false,
                        }))
                    } else {
                        let table_name = third_ident;

                        let trailing_comments = self.previous_trailing_comments().to_vec();
                        Expression::boxed_table(TableRef {
                            sample_call: None,
                            catalog: Some(first_ident),
                            schema: Some(second_ident),
                            name: table_name,
                            alias: None,
                            alias_explicit_as: false,
                            column_aliases: Vec::new(),
                            leading_comments: Vec::new(),
                            trailing_comments,
                            when: None,
                            only: false,
                            final_: false,
                            table_sample: None,
                            hints: Vec::new(),
                            system_time: None,
                            partitions: Vec::new(),
                            identifier_func: None,
                            changes: None,
                            version: None,
                            span: None,
                            whole_quoted: false,
                            catalog_parts: Vec::new(),
                        })
                    }
                }
            } else {
                if self.match_token(TokenType::LParen) {
                    let args = if self.check(TokenType::RParen) {
                        Vec::new()
                    } else {
                        self.parse_function_arguments()?
                    };
                    self.expect_token(TokenType::RParen)?;
                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: format!("{}.{}", first_name, second_name),
                        args,
                        distinct: false,
                        trailing_comments,
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                        export_state: false,
                    }))
                } else {
                    let table_name = second_ident;

                    let trailing_comments = self.previous_trailing_comments().to_vec();
                    Expression::boxed_table(TableRef {
                        sample_call: None,
                        catalog: None,
                        schema: Some(first_ident),
                        name: table_name,
                        alias: None,
                        alias_explicit_as: false,
                        column_aliases: Vec::new(),
                        leading_comments: Vec::new(),
                        trailing_comments,
                        when: None,
                        only: false,
                        final_: false,
                        table_sample: None,
                        hints: Vec::new(),
                        system_time: None,
                        partitions: Vec::new(),
                        identifier_func: None,
                        changes: None,
                        version: None,
                        span: None,
                        whole_quoted: false,
                        catalog_parts: Vec::new(),
                    })
                }
            }
        })
    }

    fn parse_table_function_source(&mut self, first_name: String) -> Result<Expression> {
        Ok({
            {
                if first_name.eq_ignore_ascii_case("JSON_TABLE") {
                    let this = self
                        .parse_bitwise()?
                        .unwrap_or(Expression::Null(crate::expressions::names::models::Null));

                    let this_with_format = if self.match_text_seq(&["FORMAT", "JSON"]) {
                        Expression::JSONFormat(Box::new(
                            crate::expressions::functions::models::JSONFormat {
                                this: Some(Box::new(this)),
                                options: Vec::new(),
                                is_json: None,
                                to_json: None,
                            },
                        ))
                    } else {
                        this
                    };

                    let path = if self.match_token(TokenType::Comma) {
                        self.parse_string()?.map(Box::new)
                    } else {
                        None
                    };

                    let error_handling = if self.match_identifier("ERROR")
                        && self.match_text_seq(&["ON", "ERROR"])
                    {
                        Some(Box::new(Expression::Var(Box::new(Var {
                            this: "ERROR ON ERROR".to_string(),
                        }))))
                    } else if self.match_text_seq(&["NULL", "ON", "ERROR"]) {
                        Some(Box::new(Expression::Var(Box::new(Var {
                            this: "NULL ON ERROR".to_string(),
                        }))))
                    } else {
                        None
                    };

                    let empty_handling = if self.match_identifier("ERROR")
                        && self.match_text_seq(&["ON", "EMPTY"])
                    {
                        Some(Box::new(Expression::Var(Box::new(Var {
                            this: "ERROR ON EMPTY".to_string(),
                        }))))
                    } else if self.match_text_seq(&["NULL", "ON", "EMPTY"]) {
                        Some(Box::new(Expression::Var(Box::new(Var {
                            this: "NULL ON EMPTY".to_string(),
                        }))))
                    } else {
                        None
                    };

                    let schema = self.parse_json_table_columns()?;

                    self.expect_token(TokenType::RParen)?;

                    Expression::JSONTable(Box::new(JSONTable {
                        options: None,
                        source_dialect: None,

                        this: Box::new(this_with_format),
                        schema: schema.map(Box::new),
                        path,
                        error_handling,
                        empty_handling,
                    }))
                } else {
                    if first_name.eq_ignore_ascii_case("XMLTABLE") {
                        if let Some(xml_table) = self.parse_xml_table()? {
                            self.expect_token(TokenType::RParen)?;
                            xml_table
                        } else {
                            return Err(self.parse_error("Failed to parse XMLTABLE"));
                        }
                    } else if first_name.eq_ignore_ascii_case("OPENJSON") {
                        if let Some(openjson_expr) = self.parse_open_json()? {
                            openjson_expr
                        } else {
                            return Err(self.parse_error("Failed to parse OPENJSON"));
                        }
                    } else if first_name.eq_ignore_ascii_case("SEMANTIC_VIEW") {
                        let semantic_view = self.parse_semantic_view()?;
                        self.expect_token(TokenType::RParen)?;
                        semantic_view
                    } else if (first_name.eq_ignore_ascii_case("view")
                        || first_name.eq_ignore_ascii_case("merge"))
                        && (self.check(TokenType::Select) || self.check(TokenType::With))
                    {
                        let query = self.parse_statement()?;
                        self.expect_token(TokenType::RParen)?;
                        let trailing_comments = self.previous_trailing_comments().to_vec();
                        Expression::Function(Box::new(Function {
                            on_error: None,
                            qualified_name: Vec::new(),
                            source_dialect: None,

                            name: first_name.to_string(),
                            args: vec![query],
                            distinct: false,
                            trailing_comments,
                            use_bracket_syntax: false,
                            no_parens: false,
                            quoted: false,
                            span: SourceSpan::default(),
                            inferred_type: None,
                            export_state: false,
                        }))
                    } else {
                        let args = if self.check(TokenType::RParen) {
                            Vec::new()
                        } else {
                            self.parse_function_arguments()?
                        };
                        self.expect_token(TokenType::RParen)?;
                        let trailing_comments = self.previous_trailing_comments().to_vec();

                        if first_name.eq_ignore_ascii_case("UNNEST") {
                            let with_ordinality = self
                                .match_keywords(&[TokenType::With, TokenType::Ordinality])
                                || self.match_text_seq(&["WITH", "OFFSET"]);
                            let offset_alias = { None };
                            let mut args_iter = args.into_iter();
                            let this = args_iter
                                .next()
                                .ok_or_else(|| self.parse_error("Expected expression in UNNEST"))?;
                            let expressions: Vec<Expression> = args_iter.collect();
                            Expression::Unnest(Box::new(
                                crate::expressions::typed_functions::models::UnnestFunc {
                                    this,
                                    expressions,
                                    with_ordinality,
                                    alias: None,
                                    offset_alias,
                                    inferred_type: None,
                                },
                            ))
                        } else {
                            let with_ordinality =
                                self.match_keywords(&[TokenType::With, TokenType::Ordinality]);
                            let func_name = if with_ordinality {
                                format!("{} WITH ORDINALITY", first_name)
                            } else {
                                first_name.clone()
                            };
                            let func = Function {
                                on_error: None,
                                qualified_name: Vec::new(),
                                source_dialect: None,

                                name: func_name,
                                args,
                                distinct: false,
                                trailing_comments,
                                use_bracket_syntax: false,
                                no_parens: false,
                                quoted: false,
                                span: SourceSpan::default(),
                                inferred_type: None,
                                export_state: false,
                            };
                            let func_expr = Expression::Function(Box::new(func));

                            if self.check(TokenType::Dot) {
                                self.skip();
                                let part1 = self.parse_bigquery_table_part()?;
                                if self.match_token(TokenType::Dot) {
                                    let part2 = self.parse_bigquery_table_part()?;
                                    if self.match_token(TokenType::Dot) {
                                        let part3 = self.parse_bigquery_table_part()?;
                                        let tc = self.previous_trailing_comments().to_vec();
                                        Expression::boxed_table(TableRef {
                                            sample_call: None,
                                            catalog: Some(part1),
                                            schema: Some(part2),
                                            name: part3,
                                            alias: None,
                                            alias_explicit_as: false,
                                            column_aliases: Vec::new(),
                                            leading_comments: Vec::new(),
                                            trailing_comments: tc,
                                            when: None,
                                            only: false,
                                            final_: false,
                                            table_sample: None,
                                            hints: Vec::new(),
                                            system_time: None,
                                            partitions: Vec::new(),
                                            identifier_func: Some(Box::new(func_expr)),
                                            changes: None,
                                            version: None,
                                            span: None,
                                            whole_quoted: false,
                                            catalog_parts: Vec::new(),
                                        })
                                    } else {
                                        let tc = self.previous_trailing_comments().to_vec();
                                        Expression::boxed_table(TableRef {
                                            sample_call: None,
                                            catalog: None,
                                            schema: Some(part1),
                                            name: part2,
                                            alias: None,
                                            alias_explicit_as: false,
                                            column_aliases: Vec::new(),
                                            leading_comments: Vec::new(),
                                            trailing_comments: tc,
                                            when: None,
                                            only: false,
                                            final_: false,
                                            table_sample: None,
                                            hints: Vec::new(),
                                            system_time: None,
                                            partitions: Vec::new(),
                                            identifier_func: Some(Box::new(func_expr)),
                                            changes: None,
                                            version: None,
                                            span: None,
                                            whole_quoted: false,
                                            catalog_parts: Vec::new(),
                                        })
                                    }
                                } else {
                                    let tc = self.previous_trailing_comments().to_vec();
                                    Expression::boxed_table(TableRef {
                                        sample_call: None,
                                        catalog: None,
                                        schema: None,
                                        name: part1,
                                        alias: None,
                                        alias_explicit_as: false,
                                        column_aliases: Vec::new(),
                                        leading_comments: Vec::new(),
                                        trailing_comments: tc,
                                        when: None,
                                        only: false,
                                        final_: false,
                                        table_sample: None,
                                        hints: Vec::new(),
                                        system_time: None,
                                        partitions: Vec::new(),
                                        identifier_func: Some(Box::new(func_expr)),
                                        changes: None,
                                        version: None,
                                        span: None,
                                        whole_quoted: false,
                                        catalog_parts: Vec::new(),
                                    })
                                }
                            } else {
                                func_expr
                            }
                        }
                    }
                }
            }
        })
    }

    fn parse_table_lateral_source(&mut self) -> Result<Expression> {
        Ok({
            if self.check(TokenType::LParen) {
                self.expect_token(TokenType::LParen)?;
                if self.check(TokenType::Select)
                    || self.check(TokenType::With)
                    || self.check(TokenType::From)
                {
                    let query = self.parse_statement()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::Subquery(Box::new(Subquery {
                        this: query,
                        alias: None,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        order_by: None,
                        limit: None,
                        offset: None,
                        lateral: true,
                        modifiers_inside: false,
                        trailing_comments: Vec::new(),
                        distribute_by: None,
                        sort_by: None,
                        cluster_by: None,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }))
                } else {
                    let table_expr = self.parse_table_expression()?;
                    self.expect_token(TokenType::RParen)?;
                    Expression::Subquery(Box::new(Subquery {
                        this: table_expr,
                        alias: None,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        order_by: None,
                        limit: None,
                        offset: None,
                        lateral: true,
                        modifiers_inside: false,
                        trailing_comments: Vec::new(),
                        distribute_by: None,
                        sort_by: None,
                        cluster_by: None,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }))
                }
            } else {
                let first_ident = self.expect_identifier_or_keyword_with_quoted()?;
                let first_name = first_ident.name.clone();

                self.expect_token(TokenType::LParen)?;
                let args = if self.check(TokenType::RParen) {
                    Vec::new()
                } else {
                    self.parse_function_arguments()?
                };
                self.expect_token(TokenType::RParen)?;

                let mut func_expr = if first_name.eq_ignore_ascii_case("UNNEST") {
                    let mut args_iter = args.into_iter();
                    let this = args_iter
                        .next()
                        .ok_or_else(|| self.parse_error("Expected expression in UNNEST"))?;
                    let expressions: Vec<Expression> = args_iter.collect();
                    Expression::Unnest(Box::new(
                        crate::expressions::typed_functions::models::UnnestFunc {
                            this,
                            expressions,
                            with_ordinality: false,
                            alias: None,
                            offset_alias: None,
                            inferred_type: None,
                        },
                    ))
                } else {
                    Expression::Function(Box::new(Function {
                        on_error: None,
                        qualified_name: Vec::new(),
                        source_dialect: None,

                        name: first_name,
                        args,
                        distinct: false,
                        trailing_comments: Vec::new(),
                        use_bracket_syntax: false,
                        no_parens: false,
                        quoted: false,
                        span: SourceSpan::default(),
                        inferred_type: None,
                        export_state: false,
                    }))
                };

                let with_offset_alias: Option<crate::expressions::names::models::Identifier> = None;
                let ordinality = if self.match_token(TokenType::With) {
                    if self.match_token(TokenType::Ordinality) {
                        Some(Box::new(Expression::Boolean(BooleanLiteral {
                            value: true,
                        })))
                    } else if self.check(TokenType::Offset) || self.check_identifier("OFFSET") {
                        self.skip();

                        Some(Box::new(Expression::Boolean(BooleanLiteral {
                            value: true,
                        })))
                    } else {
                        self.current -= 1;
                        None
                    }
                } else {
                    None
                };

                if ordinality.is_some() {
                    if let Expression::Unnest(ref mut u) = func_expr {
                        u.with_ordinality = true;
                        u.offset_alias = with_offset_alias;
                    }
                }

                let alias_ident = if self.match_token(TokenType::As)
                    || !self.is_at_end()
                        && !self.check(TokenType::Comma)
                        && !self.check(TokenType::RParen)
                        && !self.check(TokenType::On)
                        && !self.check(TokenType::Cross)
                        && !self.check(TokenType::Inner)
                        && !self.check(TokenType::Left)
                        && !self.check(TokenType::Right)
                        && !self.check(TokenType::Full)
                        && !self.check(TokenType::Join)
                        && !self.check(TokenType::Where)
                        && !self.check(TokenType::Order)
                        && !self.check(TokenType::Limit)
                        && !self.check(TokenType::Semicolon)
                        && (self.check(TokenType::Identifier)
                            || self.check(TokenType::Var)
                            || self.check(TokenType::QuotedIdentifier))
                {
                    Some(self.expect_identifier_or_keyword_with_quoted()?)
                } else {
                    None
                };
                let alias_quoted = alias_ident.as_ref().is_some_and(|id| id.quoted);
                let alias = alias_ident.map(|id| id.name);

                let mut column_aliases_quoted: Vec<bool> = Vec::new();
                let column_aliases = if alias.is_some() && self.match_token(TokenType::LParen) {
                    let mut cols: Vec<String> = Vec::new();
                    loop {
                        let column = self.expect_identifier_or_keyword_with_quoted()?;
                        column_aliases_quoted.push(column.quoted);
                        cols.push(column.name);
                        if !self.match_token(TokenType::Comma) {
                            break;
                        }
                    }
                    self.expect_token(TokenType::RParen)?;
                    cols
                } else {
                    Vec::new()
                };
                if !column_aliases_quoted.iter().any(|quoted| *quoted) {
                    column_aliases_quoted.clear();
                }

                Expression::Lateral(Box::new(Lateral {
                    this: Box::new(func_expr),
                    view: None,
                    outer: None,
                    alias,
                    alias_quoted,
                    cross_apply: None,
                    ordinality,
                    column_aliases,
                    column_aliases_quoted,
                    span: SourceSpan::default(),
                }))
            }
        })
    }

    fn parse_table_parenthesized_source(&mut self) -> Result<Expression> {
        Ok({
            let set_operation_may_follow = self.current < SECOND_PREVIOUS_OFFSET
                || self.tokens[self.current - 2].token_type == TokenType::LParen;
            if self.check(TokenType::Values) {
                let mut values = self.parse_values()?;
                self.expect_token(TokenType::RParen)?;
                let (alias, column_aliases) = if let Expression::Values(ref mut v) = values {
                    (v.alias.take(), std::mem::take(&mut v.column_aliases))
                } else {
                    (None, Vec::new())
                };
                Expression::Subquery(Box::new(Subquery {
                    this: values,
                    alias,
                    column_aliases,
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
            } else if self.check(TokenType::Select)
                || self.check(TokenType::With)
                || self.check(TokenType::Pivot)
                || self.check(TokenType::Unpivot)
                || self.check(TokenType::From)
                || self.check(TokenType::Merge)
                || self.check(TokenType::Describe)
                || self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("EXPLAIN")
                || self.check(TokenType::Var) && self.peek_text().eq_ignore_ascii_case("SUMMARIZE")
            {
                let query = { self.parse_statement()? };
                self.expect_token(TokenType::RParen)?;
                let trailing = self.previous_trailing_comments().to_vec();
                let result = if set_operation_may_follow
                    && (self.check(TokenType::Union)
                        || self.check(TokenType::Intersect)
                        || self.check(TokenType::Except))
                {
                    let left = Expression::Subquery(Box::new(Subquery {
                        this: query,
                        alias: None,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        order_by: None,
                        limit: None,
                        offset: None,
                        lateral: false,
                        modifiers_inside: false,
                        trailing_comments: Vec::new(),
                        distribute_by: None,
                        sort_by: None,
                        cluster_by: None,
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }));
                    self.parse_set_operation(left)?
                } else {
                    query
                };
                Expression::Subquery(Box::new(Subquery {
                    this: result,
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
                    trailing_comments: trailing,
                    inferred_type: None,
                    span: SourceSpan::default(),
                }))
            } else if self.check(TokenType::LParen) {
                let inner = self.parse_table_expression()?;

                let inner = if self.match_token(TokenType::As)
                    || (self.is_identifier_token()
                        && !self.check(TokenType::Union)
                        && !self.check(TokenType::Intersect)
                        && !self.check(TokenType::Except)
                        && !self.check(TokenType::Cross)
                        && !self.check(TokenType::Inner)
                        && !self.check(TokenType::Left)
                        && !self.check(TokenType::Right)
                        && !self.check(TokenType::Full)
                        && !self.check(TokenType::Join)
                        && !self.check(TokenType::Order)
                        && !self.check(TokenType::Limit)
                        && !self.check(TokenType::Offset)
                        && !self.check(TokenType::Xor))
                {
                    let alias = self.expect_identifier()?;
                    if let Expression::Subquery(mut subq) = inner {
                        subq.alias = Some(Identifier::new(alias));
                        Expression::Subquery(subq)
                    } else {
                        Expression::Alias(Box::new(Alias::new(inner, Identifier::new(alias))))
                    }
                } else {
                    inner
                };

                let had_set_operation = self.check(TokenType::Union)
                    || self.check(TokenType::Intersect)
                    || self.check(TokenType::Except);
                let result = if had_set_operation {
                    self.parse_set_operation(inner)?
                } else if self.check(TokenType::Cross)
                    || self.check(TokenType::Outer)
                    || self.check(TokenType::Inner)
                    || self.check(TokenType::Left)
                    || self.check(TokenType::Right)
                    || self.check(TokenType::Full)
                    || self.check(TokenType::Join)
                {
                    let joins = self.parse_joins()?;
                    let lateral_views = self.parse_lateral_views()?;
                    Expression::JoinedTable(Box::new(JoinedTable {
                        left: inner,
                        joins,
                        lateral_views,
                        alias: None,
                    }))
                } else {
                    inner
                };

                let result = if self.check(TokenType::Order) {
                    self.expect_token(TokenType::Order)?;
                    self.expect_token(TokenType::By)?;
                    let order_by = self.parse_order_by()?;
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
                    Expression::Subquery(Box::new(Subquery {
                        this: result,
                        alias: None,
                        column_aliases: Vec::new(),
                        alias_explicit_as: false,
                        alias_keyword: None,
                        order_by: Some(order_by),
                        limit,
                        offset,
                        distribute_by: None,
                        sort_by: None,
                        cluster_by: None,
                        lateral: false,
                        modifiers_inside: true,
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }))
                } else if self.check(TokenType::Limit) || self.check(TokenType::Offset) {
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
                    Expression::Subquery(Box::new(Subquery {
                        this: result,
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
                        trailing_comments: Vec::new(),
                        inferred_type: None,
                        span: SourceSpan::default(),
                    }))
                } else {
                    result
                };

                self.expect_token(TokenType::RParen)?;
                let had_modifiers = matches!(&result, Expression::Subquery(s) if s.order_by.is_some() || s.limit.is_some() || s.offset.is_some());
                let result_is_subquery_of_set_op = matches!(&result, Expression::Subquery(s) if matches!(&s.this, Expression::Union(_) | Expression::Intersect(_) | Expression::Except(_)));
                if had_modifiers || result_is_subquery_of_set_op {
                    result
                } else {
                    Expression::Paren(Box::new(Paren {
                        this: result,
                        trailing_comments: Vec::new(),
                        span: SourceSpan::default(),
                    }))
                }
            } else if self.is_identifier_token()
                || self.is_safe_keyword_as_identifier()
                || self.can_be_alias_keyword()
            {
                let (left, joins) = self.parse_table_expression_with_joins()?;
                let lateral_views = self.parse_lateral_views()?;
                self.expect_token(TokenType::RParen)?;
                if joins.is_empty() && lateral_views.is_empty() {
                    Expression::Paren(Box::new(Paren {
                        this: left,
                        trailing_comments: Vec::new(),
                        span: SourceSpan::default(),
                    }))
                } else {
                    Expression::JoinedTable(Box::new(JoinedTable {
                        left,
                        joins,
                        lateral_views,
                        alias: None,
                    }))
                }
            } else {
                let query = self.parse_statement()?;
                self.expect_token(TokenType::RParen)?;
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
            }
        })
    }

    fn parse_table_brace_source(&mut self) -> Result<Expression> {
        Ok({
            if let Some(param) = self.parse_clickhouse_braced_parameter()? {
                param
            } else {
                self.skip();
                if self.is_identifier_token() || self.is_safe_keyword_as_identifier() {
                    let name_token = self.advance()?;
                    self.expect_token(TokenType::RBrace)?;
                    Expression::Parameter(Box::new(Parameter {
                        name: Some(name_token.text.to_string()),
                        index: None,
                        style: ParameterStyle::Brace,
                        quoted: false,
                        string_quoted: false,
                        expression: None,
                    }))
                } else {
                    return Err(self.parse_error("Expected identifier after {"));
                }
            }
        })
    }

    fn parse_table_dollar_brace_source(&mut self) -> Result<Expression> {
        Ok({
            self.skip();
            self.skip();
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
                Expression::Parameter(Box::new(Parameter {
                    name: Some(name_token.text.to_string()),
                    index: None,
                    style: ParameterStyle::DollarBrace,
                    quoted: false,
                    string_quoted: false,
                    expression,
                }))
            } else {
                return Err(self.parse_error("Expected identifier after ${"));
            }
        })
    }

    fn parse_table_string_source(&mut self) -> Result<Expression> {
        Ok({
            let string_token = self.advance()?;
            let table_name = Identifier {
                name: string_token.text.to_string(),
                quoted: true,
                trailing_comments: Vec::new(),
                span: Some(string_token.span),
            };
            let trailing_comments = self.previous_trailing_comments().to_vec();
            Expression::boxed_table(TableRef {
                sample_call: None,
                catalog: None,
                schema: None,
                name: table_name,
                alias: None,
                alias_explicit_as: false,
                column_aliases: Vec::new(),
                leading_comments: Vec::new(),
                trailing_comments,
                when: None,
                only: false,
                final_: false,
                table_sample: None,
                hints: Vec::new(),
                system_time: None,
                partitions: Vec::new(),
                identifier_func: None,
                changes: None,
                version: None,
                span: None,
                whole_quoted: false,
                catalog_parts: Vec::new(),
            })
        })
    }
}
